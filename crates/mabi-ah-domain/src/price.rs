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
        let mut v: Vec<(u64, u64)> = iter.into_iter().filter(|&(_, c)| c > 0).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_unstable_by_key(|&(p, _)| p);
        let units: u64 = v
            .iter()
            .map(|&(_, c)| u128::from(c))
            .sum::<u128>()
            .min(u128::from(u64::MAX)) as u64;
        let rank = |num: u128, den: u128| -> u64 {
            let target = (u128::from(units) * num).div_ceil(den).max(1);
            let mut cum: u128 = 0;
            for &(p, c) in &v {
                cum += u128::from(c);
                if cum >= target {
                    return p;
                }
            }
            v[v.len() - 1].0
        };
        Some(Self {
            min: v[0].0,
            p25: rank(1, 4),
            median: rank(1, 2),
            p75: rank(3, 4),
            max: v[v.len() - 1].0,
            listings: v.len(),
            units,
        })
    }
}
