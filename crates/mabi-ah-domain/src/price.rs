//! Count-weighted price statistics over listings or trades.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceStats {
    pub min: u64,
    pub p25: u64,
    pub median: u64,
    pub p75: u64,
    pub max: u64,
    /// Number of (price, count) entries with count > 0.
    pub listings: usize,
    /// Sum of counts.
    pub units: u64,
}

impl PriceStats {
    /// `(unit_price, count)` pairs; percentiles are nearest-rank weighted by
    /// count. Returns `None` when there are no units.
    pub fn from_units<I: IntoIterator<Item = (u64, u64)>>(iter: I) -> Option<Self> {
        let _ = iter.into_iter().count();
        todo!("from_units")
    }
}
