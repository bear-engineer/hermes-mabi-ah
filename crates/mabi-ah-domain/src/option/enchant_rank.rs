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
        self as u8
    }
}

impl fmt::Display for EnchantRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = self.ordinal();
        if o <= Self::A.ordinal() {
            write!(f, "{}", char::from(b'F' - o))
        } else {
            write!(f, "{}", 9 - (o - Self::N9.ordinal()))
        }
    }
}

impl FromStr for EnchantRank {
    type Err = EnchantRankParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        let t = t.strip_prefix("랭크").unwrap_or(t).trim();
        let err = || EnchantRankParseError(s.to_string());
        let mut chars = t.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err(err());
        };
        let c = c.to_ascii_uppercase();
        Self::ALL
            .into_iter()
            .find(|r| r.to_string().starts_with(c))
            .ok_or_else(err)
    }
}

impl From<EnchantRank> for f64 {
    fn from(r: EnchantRank) -> f64 {
        f64::from(r.ordinal())
    }
}
