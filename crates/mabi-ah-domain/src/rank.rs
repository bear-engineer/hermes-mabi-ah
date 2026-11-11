//! Rank: top items by volume or value, grouped by `ItemName::base` (pure).

use crate::trade::Trade;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankBy {
    Volume,
    Value,
}

/// `display_sample` is the display name of the first trade seen for that base.
/// `value` is the sum of `unit_price * count`, saturating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankEntry {
    pub base_name: String,
    pub display_sample: String,
    pub units: u64,
    pub value: u64,
    pub trades: u64,
}

/// Top `top` entries, descending by the chosen metric, ties by `base_name`
/// ascending. `category` (exact match) restricts the trades considered.
pub fn rank(trades: &[Trade], by: RankBy, top: usize, category: Option<&str>) -> Vec<RankEntry> {
    let _ = (trades, by, top, category);
    todo!()
}
