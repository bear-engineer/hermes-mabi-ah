//! Option-family predicates (enchant, reforge, erg, mod/piercing).

use std::str::FromStr;

use super::enchant_rank::EnchantRank;
pub use super::filter::FilterExpr;
use super::{ItemOption, OptionRegistry, OptionValue};

/// Numeric comparison used by level predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Ge,
    Le,
    Eq,
}

impl Cmp {
    pub fn test<T: PartialOrd>(self, actual: &T, wanted: &T) -> bool {
        match self {
            Cmp::Ge => actual >= wanted,
            Cmp::Le => actual <= wanted,
            Cmp::Eq => actual == wanted,
        }
    }
}

/// A predicate string could not be parsed; every variant carries the offending token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PredicateParseError {
    #[error("empty predicate")]
    Empty,
    #[error("unknown slot '{0}'")]
    BadSlot(String),
    #[error("unknown enchant rank '{0}'")]
    BadRank(String),
    #[error("level is not a number: '{0}'")]
    BadLevel(String),
    #[error("unknown erg grade '{0}'")]
    BadGrade(String),
    #[error("unknown mod kind '{0}'")]
    UnknownModKind(String),
    #[error("missing option name in '{0}'")]
    MissingName(String),
    #[error("operator not allowed here: '{0}'")]
    BadOperator(String),
    #[error("empty enchant rank range '{0}' (minimum is above maximum)")]
    EmptyRange(String),
}

impl PredicateParseError {
    pub fn token(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::BadSlot(t)
            | Self::BadRank(t)
            | Self::BadLevel(t)
            | Self::BadGrade(t)
            | Self::UnknownModKind(t)
            | Self::MissingName(t)
            | Self::BadOperator(t)
            | Self::EmptyRange(t) => t,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnchantSlot {
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantPred {
    pub slot: Option<EnchantSlot>,
    pub name: Option<String>,
    pub min: Option<EnchantRank>,
    pub max: Option<EnchantRank>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReforgePred {
    pub slot: Option<u8>,
    pub name: String,
    pub level: Option<(Cmp, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErgPred {
    pub grade: String,
    pub level: Option<(Cmp, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModKind {
    Normal,
    Special,
    Gem,
    Master,
    Piercing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModCond {
    Num(Cmp, i64),
    Contains(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModPred {
    pub kind: ModKind,
    pub sub: Option<String>,
    pub cond: Option<ModCond>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionPredicate {
    Enchant(EnchantPred),
    Reforge(ReforgePred),
    Erg(ErgPred),
    Mod(ModPred),
    Generic(FilterExpr),
}

/// Comparison operator found in a predicate string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Cmp(Cmp),
    Contains,
}

/// Split `head<op>tail` at the first operator.
fn split_cond(s: &str) -> (&str, Option<(Op, &str)>) {
    for (i, c) in s.char_indices() {
        let rest = &s[i..];
        let found = match c {
            '>' if rest.starts_with(">=") => Some((Op::Cmp(Cmp::Ge), 2)),
            '<' if rest.starts_with("<=") => Some((Op::Cmp(Cmp::Le), 2)),
            '=' => Some((Op::Cmp(Cmp::Eq), 1)),
            '~' => Some((Op::Contains, 1)),
            _ => None,
        };
        if let Some((op, len)) = found {
            return (s[..i].trim(), Some((op, s[i + len..].trim())));
        }
    }
    (s.trim(), None)
}

fn only_cmp(op: Op, whole: &str) -> Result<Cmp, PredicateParseError> {
    match op {
        Op::Cmp(c) => Ok(c),
        Op::Contains => Err(PredicateParseError::BadOperator(whole.to_string())),
    }
}

fn level_cond(s: &str) -> Result<Option<(Cmp, u32)>, PredicateParseError> {
    let (_, cond) = split_cond(s);
    cond.map(|(op, v)| {
        let cmp = only_cmp(op, s)?;
        let n = v
            .parse::<u32>()
            .map_err(|_| PredicateParseError::BadLevel(v.to_string()))?;
        Ok((cmp, n))
    })
    .transpose()
}

fn level_of(v: &OptionValue, o: &ItemOption) -> Option<i64> {
    match v {
        OptionValue::Number(n) => Some(*n),
        OptionValue::CurrentMax { cur, .. } => Some(*cur),
        OptionValue::Leveled { level, .. } => Some(i64::from(*level)),
        OptionValue::Raw(_) => o.value.as_deref().and_then(|s| s.trim().parse().ok()),
        _ => None,
    }
}

impl EnchantSlot {
    fn from_token(t: &str) -> Option<Self> {
        match t.trim() {
            "접두" => Some(Self::Prefix),
            "접미" => Some(Self::Suffix),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prefix => "접두",
            Self::Suffix => "접미",
        }
    }
}

const ENCHANT_TYPES: [&str; 2] = ["인챈트", "인챈트 종류"];

/// `[slot][:name][<op>rank[,<op>rank]]`, e.g. `접두:날카로운`, `>=E`, `접미>=7`, `>=F,<=A`.
impl FromStr for EnchantPred {
    type Err = PredicateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PredicateParseError::Empty);
        }
        let first_op = s.find(['>', '<', '=']);
        let (head, ranks) = match first_op {
            Some(i) => (s[..i].trim(), Some(&s[i..])),
            None => (s, None),
        };
        let mut pred = Self {
            slot: None,
            name: None,
            min: None,
            max: None,
        };
        if let Some((slot, name)) = head.split_once(':') {
            pred.slot = Some(
                EnchantSlot::from_token(slot)
                    .ok_or_else(|| PredicateParseError::BadSlot(slot.trim().to_string()))?,
            );
            pred.name = Some(name.trim().to_string()).filter(|n| !n.is_empty());
        } else if let Some(slot) = EnchantSlot::from_token(head) {
            pred.slot = Some(slot);
        } else if !head.is_empty() {
            pred.name = Some(head.to_string());
        }
        for part in ranks.into_iter().flat_map(|r| r.split(',')) {
            let (_, cond) = split_cond(part);
            let Some((op, v)) = cond else {
                return Err(PredicateParseError::BadOperator(part.trim().to_string()));
            };
            let cmp = only_cmp(op, part)?;
            let rank: EnchantRank = v
                .parse()
                .map_err(|_| PredicateParseError::BadRank(v.to_string()))?;
            match cmp {
                Cmp::Ge => pred.min = Some(rank),
                Cmp::Le => pred.max = Some(rank),
                Cmp::Eq => {
                    pred.min = Some(rank);
                    pred.max = Some(rank);
                }
            }
        }
        if let (Some(min), Some(max)) = (pred.min, pred.max)
            && min > max
        {
            return Err(PredicateParseError::EmptyRange(s.to_string()));
        }
        Ok(pred)
    }
}

impl EnchantPred {
    /// Some single enchant option satisfies slot, name and rank range together.
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        options
            .iter()
            .filter(|o| ENCHANT_TYPES.contains(&o.option_type.as_str()))
            .any(|o| match reg.parse_value(o) {
                OptionValue::Enchant {
                    slot, name, rank, ..
                } => {
                    self.slot.is_none_or(|s| s.as_str() == slot)
                        && self.name.as_deref().is_none_or(|n| n == name)
                        && self.min.is_none_or(|m| rank >= m)
                        && self.max.is_none_or(|m| rank <= m)
                }
                _ => false,
            })
    }
}

/// `[slot:]effect[<op>level]`, e.g. `최대 공격력>=18`, `2:최대 공격력>=18`.
impl FromStr for ReforgePred {
    type Err = PredicateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PredicateParseError::Empty);
        }
        let (head, _) = split_cond(s);
        let level = level_cond(s)?;
        let (slot, name) = match head.split_once(':') {
            Some((n, rest))
                if !n.trim().is_empty() && n.trim().chars().all(|c| c.is_ascii_digit()) =>
            {
                let n = n.trim();
                let slot = n
                    .parse::<u8>()
                    .ok()
                    .filter(|v| (1..=3).contains(v))
                    .ok_or_else(|| PredicateParseError::BadSlot(n.to_string()))?;
                (Some(slot), rest.trim())
            }
            _ => (None, head),
        };
        if name.is_empty() {
            return Err(PredicateParseError::MissingName(s.to_string()));
        }
        Ok(Self {
            slot,
            name: name.to_string(),
            level,
        })
    }
}

/// The one reforge name rule: `wanted` equals the effect name or its trailing words.
pub fn reforge_name_matches(effect: &str, wanted: &str) -> bool {
    let (effect, wanted) = (effect.trim(), wanted.trim());
    !wanted.is_empty() && (effect == wanted || effect.ends_with(&format!(" {wanted}")))
}

impl ReforgePred {
    /// Effect name matches exactly or as trailing words (`최대 대미지` matches `듀얼건 최대 대미지`).
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        options
            .iter()
            .filter(|o| o.option_type == "세공 옵션")
            .filter(|o| {
                self.slot.is_none_or(|s| {
                    o.sub_type.as_deref().map(str::trim) == Some(s.to_string().as_str())
                })
            })
            .any(|o| match reg.parse_value(o) {
                OptionValue::Leveled { effect, level, .. } => {
                    reforge_name_matches(&effect, &self.name)
                        && self.level.is_none_or(|(c, n)| c.test(&level, &n))
                }
                _ => false,
            })
    }
}

/// `grade[<op>level]`, e.g. `S>=40`.
impl FromStr for ErgPred {
    type Err = PredicateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PredicateParseError::Empty);
        }
        let (head, _) = split_cond(s);
        let mut chars = head.chars();
        let grade = match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_alphabetic() => c.to_ascii_uppercase().to_string(),
            _ => return Err(PredicateParseError::BadGrade(head.to_string())),
        };
        Ok(Self {
            grade,
            level: level_cond(s)?,
        })
    }
}

impl ErgPred {
    /// Level is the current erg level (`option_value`).
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        options
            .iter()
            .filter(|o| o.option_type == "에르그")
            .filter(|o| {
                o.sub_type
                    .as_deref()
                    .is_some_and(|g| g.trim().eq_ignore_ascii_case(&self.grade))
            })
            .any(|o| {
                self.level.is_none_or(|(c, n)| {
                    level_of(&reg.parse_value(o), o).is_some_and(|l| c.test(&l, &i64::from(n)))
                })
            })
    }
}

impl ModKind {
    fn from_token(t: &str) -> Option<Self> {
        match t.trim().trim_end_matches(" 개조").trim_end_matches(" 레벨") {
            "일반" => Some(Self::Normal),
            "특별" => Some(Self::Special),
            "보석" => Some(Self::Gem),
            "장인" => Some(Self::Master),
            "피어싱" => Some(Self::Piercing),
            _ => None,
        }
    }

    pub fn option_type(self) -> &'static str {
        match self {
            Self::Normal => "일반 개조",
            Self::Special => "특별 개조",
            Self::Gem => "보석 개조",
            Self::Master => "장인 개조",
            Self::Piercing => "피어싱 레벨",
        }
    }
}

/// `kind[:sub][<op>n]` / `장인~text`, e.g. `특별:R>=7`, `일반=5`, `피어싱>=3`, `장인~보호`.
impl FromStr for ModPred {
    type Err = PredicateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PredicateParseError::Empty);
        }
        let (head, cond) = split_cond(s);
        let (kind_tok, sub) = match head.split_once(':') {
            Some((k, sub)) => (
                k.trim(),
                Some(sub.trim().to_string()).filter(|x| !x.is_empty()),
            ),
            None => (head, None),
        };
        let kind = ModKind::from_token(kind_tok)
            .ok_or_else(|| PredicateParseError::UnknownModKind(kind_tok.to_string()))?;
        let cond = match cond {
            None => None,
            Some((Op::Contains, v)) if !v.is_empty() => Some(ModCond::Contains(v.to_string())),
            Some((Op::Contains, _)) => {
                return Err(PredicateParseError::BadOperator(s.to_string()));
            }
            Some((Op::Cmp(c), v)) => Some(ModCond::Num(
                c,
                v.parse::<i64>()
                    .map_err(|_| PredicateParseError::BadLevel(v.to_string()))?,
            )),
        };
        Ok(Self { kind, sub, cond })
    }
}

impl ModPred {
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        options
            .iter()
            .filter(|o| o.option_type == self.kind.option_type())
            .filter(|o| {
                self.sub
                    .as_deref()
                    .is_none_or(|s| o.sub_type.as_deref().map(str::trim) == Some(s))
            })
            .any(|o| match &self.cond {
                None => true,
                Some(ModCond::Num(c, n)) => {
                    level_of(&reg.parse_value(o), o).is_some_and(|l| c.test(&l, n))
                }
                Some(ModCond::Contains(t)) => [&o.value, &o.value2, &o.desc]
                    .into_iter()
                    .flatten()
                    .any(|f| f.contains(t.as_str())),
            })
    }
}

impl OptionPredicate {
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        match self {
            Self::Enchant(p) => p.matches(options, reg),
            Self::Reforge(p) => p.matches(options, reg),
            Self::Erg(p) => p.matches(options, reg),
            Self::Mod(p) => p.matches(options, reg),
            Self::Generic(p) => p.matches(options, reg),
        }
    }
}
