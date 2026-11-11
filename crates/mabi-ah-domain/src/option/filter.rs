//! Client-side option filter: `<option_type>[:<sub_type>](>=|<=|=|~)<value>`.

use std::str::FromStr;

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
        if matches!(op, FilterOp::Ge | FilterOp::Le) && number(value).is_none() {
            return Err(FilterParseError::NotNumeric(value.to_string()));
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
        let scalar = scalar(&reg.parse_value(o));
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
        OptionValue::Enchant { rank, .. } => Some(f64::from(*rank)),
        OptionValue::Raw(o) => o.value.as_deref().and_then(number),
        OptionValue::Rgb { .. } | OptionValue::Text(_) | OptionValue::Flag(_) => None,
    }
}
