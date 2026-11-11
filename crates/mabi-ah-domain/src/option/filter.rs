//! Client-side option filter: `<option_type>[:<sub_type>](>=|<=|=|~)<value>`.

use std::str::FromStr;

use super::enchant_rank::EnchantRank;
use super::predicate::Cmp;
use super::{ItemOption, OptionRegistry, OptionValue};

/// Filter string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FilterParseError {
    #[error("missing operator (>=, <=, = or ~) in '{0}'")]
    MissingOperator(String),
    #[error("missing option_type in '{0}'")]
    MissingType(String),
    #[error("missing value in '{0}'")]
    MissingValue(String),
    #[error("operator needs a numeric value, got '{0}'")]
    NotNumeric(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Ge,
    Le,
    Eq,
    Contains,
}

impl FilterOp {
    /// Ordering comparison, if this op is one.
    pub(crate) fn cmp(self) -> Option<Cmp> {
        match self {
            FilterOp::Ge => Some(Cmp::Ge),
            FilterOp::Le => Some(Cmp::Le),
            FilterOp::Eq => Some(Cmp::Eq),
            FilterOp::Contains => None,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            FilterOp::Ge => ">=",
            FilterOp::Le => "<=",
            FilterOp::Eq => "=",
            FilterOp::Contains => "~",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionFilter {
    pub option_type: String,
    pub sub_type: Option<String>,
    pub op: FilterOp,
    pub value: String,
}

impl FromStr for OptionFilter {
    type Err = FilterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (idx, op, len) = s
            .char_indices()
            .find_map(|(i, c)| {
                let rest = &s[i..];
                match c {
                    '>' if rest.starts_with(">=") => Some((i, FilterOp::Ge, 2)),
                    '<' if rest.starts_with("<=") => Some((i, FilterOp::Le, 2)),
                    '=' => Some((i, FilterOp::Eq, 1)),
                    '~' => Some((i, FilterOp::Contains, 1)),
                    _ => None,
                }
            })
            .ok_or_else(|| FilterParseError::MissingOperator(s.to_string()))?;
        let (left, value) = (s[..idx].trim(), s[idx + len..].trim());
        let (ty, sub) = match left.split_once(':') {
            Some((t, sub)) => (t.trim(), Some(sub.trim()).filter(|x| !x.is_empty())),
            None => (left, None),
        };
        if ty.is_empty() {
            return Err(FilterParseError::MissingType(s.to_string()));
        }
        if value.is_empty() {
            return Err(FilterParseError::MissingValue(s.to_string()));
        }
        if matches!(op, FilterOp::Ge | FilterOp::Le) {
            let ok = if is_enchant_type(ty) {
                value.parse::<EnchantRank>().is_ok()
            } else {
                number(value).is_some()
            };
            if !ok {
                return Err(FilterParseError::NotNumeric(value.to_string()));
            }
        }
        Ok(Self {
            option_type: ty.to_string(),
            sub_type: sub.map(String::from),
            op,
            value: value.to_string(),
        })
    }
}

impl OptionFilter {
    /// True if any option satisfies the filter.
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        options.iter().any(|o| self.matches_one(o, reg))
    }

    fn matches_one(&self, o: &ItemOption, reg: &OptionRegistry) -> bool {
        if o.option_type != self.option_type {
            return false;
        }
        if self
            .sub_type
            .as_deref()
            .is_some_and(|s| o.sub_type.as_deref().map(str::trim) != Some(s))
        {
            return false;
        }
        if self.op == FilterOp::Contains {
            return [&o.value, &o.value2, &o.desc]
                .into_iter()
                .flatten()
                .any(|f| f.contains(self.value.as_str()));
        }
        let parsed = reg.parse_value(o);
        if let (OptionValue::Enchant { rank, .. }, Some(cmp), Ok(want)) =
            (&parsed, self.op.cmp(), self.value.parse::<EnchantRank>())
        {
            return cmp.test(rank, &want);
        }
        let scalar = scalar(&parsed);
        let want = number(&self.value);
        match (self.op, scalar, want) {
            (FilterOp::Ge, Some(a), Some(b)) => a >= b,
            (FilterOp::Le, Some(a), Some(b)) => a <= b,
            (FilterOp::Eq, Some(a), Some(b)) => a == b,
            (FilterOp::Eq, _, _) => o.value.as_deref().map(str::trim) == Some(self.value.as_str()),
            _ => false,
        }
    }
}

fn is_enchant_type(ty: &str) -> bool {
    matches!(ty, "인챈트" | "인챈트 종류")
}

fn number(s: &str) -> Option<f64> {
    s.trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|f| f.is_finite())
}

/// Single comparable number for a parsed value (range -> max, current/max -> current).
fn scalar(v: &OptionValue) -> Option<f64> {
    match v {
        OptionValue::Number(n) => Some(*n as f64),
        OptionValue::Range { max, .. } => Some(*max as f64),
        OptionValue::Percent(p) => Some(*p),
        OptionValue::CurrentMax { cur, .. } => Some(*cur as f64),
        OptionValue::Leveled { level, .. } => Some(f64::from(*level)),
        // Enchant ranks are compared through `EnchantRank`, never as numbers.
        OptionValue::Enchant { .. } => None,
        OptionValue::Raw(o) => o.value.as_deref().and_then(number),
        OptionValue::Rgb { .. } | OptionValue::Text(_) | OptionValue::Flag(_) => None,
    }
}

/// One filter expression: an optional leading `!` (negation) over `|`-separated alternatives.
///
/// Escapes: `\|` literal pipe, `\!` literal leading `!`, `\\` backslash.
/// An alternative with no operator inherits type/sub_type/operator of the first
/// one (`색상~빨강|파랑` = `색상~빨강` or `색상~파랑`); one with an operator
/// is a full filter (`에르그>=40|품질>=3`). `!` negates the whole group.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterExpr {
    pub negated: bool,
    pub alternatives: Vec<OptionFilter>,
}

impl FromStr for FilterExpr {
    type Err = FilterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (negated, body) = match s.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        let mut segments = split_unescaped(body).into_iter();
        let first_raw = segments.next().unwrap_or_default();
        let first: OptionFilter = first_raw.parse()?;
        let mut alternatives = vec![first];
        for seg in segments {
            let f = if has_operator(&seg) {
                seg.parse()?
            } else {
                let base = &alternatives[0];
                let mut left = base.option_type.clone();
                if let Some(sub) = &base.sub_type {
                    left.push(':');
                    left.push_str(sub);
                }
                format!("{left}{}{}", base.op.symbol(), seg.trim()).parse()?
            };
            alternatives.push(f);
        }
        Ok(Self {
            negated,
            alternatives,
        })
    }
}

impl FilterExpr {
    /// Any alternative matches some option; flipped when negated.
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        self.alternatives.iter().any(|f| f.matches(options, reg)) != self.negated
    }
}

fn has_operator(s: &str) -> bool {
    s.contains(['=', '~']) || s.contains(">=") || s.contains("<=")
}

/// Split on unescaped `|` and resolve escapes (`\x` -> `x`).
fn split_unescaped(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => cur.push(chars.next().unwrap_or('\\')),
            '|' => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}
