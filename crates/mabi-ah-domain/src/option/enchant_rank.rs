//! Enchant rank: letters F..A then digits 9..1 (1 is the best).

use std::fmt;
use std::str::FromStr;

/// Rank ordering is weakest to strongest: F < E < D < C < B < A < 9 < ... < 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnchantRank {
    F,
    E,
    D,
    C,
    B,
    A,
    N9,
    N8,
    N7,
    N6,
    N5,
    N4,
    N3,
    N2,
    N1,
}

/// Rank text that is not one of F..A or 9..1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid enchant rank: {0:?}")]
pub struct EnchantRankParseError(pub String);

impl EnchantRank {
    /// All ranks, weakest first.
    pub const ALL: [EnchantRank; 15] = [
        Self::F,
        Self::E,
        Self::D,
        Self::C,
        Self::B,
        Self::A,
        Self::N9,
        Self::N8,
        Self::N7,
        Self::N6,
        Self::N5,
        Self::N4,
        Self::N3,
        Self::N2,
        Self::N1,
    ];

    /// 0 for F up to 14 for rank 1.
    pub fn ordinal(self) -> u8 {
        todo!()
    }
}

impl fmt::Display for EnchantRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl FromStr for EnchantRank {
    type Err = EnchantRankParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl From<EnchantRank> for f64 {
    fn from(r: EnchantRank) -> f64 {
        f64::from(r.ordinal())
    }
}
