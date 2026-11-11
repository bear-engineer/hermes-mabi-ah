//! Option-family predicates (enchant, reforge, erg, mod/piercing).

use std::str::FromStr;

use super::enchant_rank::EnchantRank;
pub use super::filter::FilterExpr;
use super::{ItemOption, OptionRegistry};

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
            | Self::BadOperator(t) => t,
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

macro_rules! stub_family {
    ($t:ty) => {
        impl FromStr for $t {
            type Err = PredicateParseError;
            fn from_str(_s: &str) -> Result<Self, Self::Err> {
                todo!()
            }
        }
        impl $t {
            pub fn matches(&self, _options: &[ItemOption], _reg: &OptionRegistry) -> bool {
                todo!()
            }
        }
    };
}
stub_family!(EnchantPred);
stub_family!(ReforgePred);
stub_family!(ErgPred);
stub_family!(ModPred);

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
