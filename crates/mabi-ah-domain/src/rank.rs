//! Rank: top items by volume or value, grouped by `ItemName::base` (pure).

use std::collections::HashMap;

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
    let mut groups: Vec<RankEntry> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for t in trades.iter().filter(|t| category.is_none_or(|c| t.category == c)) {
        let value = t.unit_price.saturating_mul(t.count);
        let i = *index.entry(t.name.base.as_str()).or_insert_with(|| {
            groups.push(RankEntry {
                base_name: t.name.base.clone(),
                display_sample: t.name.display.clone(),
                units: 0,
                value: 0,
                trades: 0,
            });
            groups.len() - 1
        });
        let e = &mut groups[i];
        e.units = e.units.saturating_add(t.count);
        e.value = e.value.saturating_add(value);
        e.trades += 1;
    }
    let key = |e: &RankEntry| match by {
        RankBy::Volume => e.units,
        RankBy::Value => e.value,
    };
    groups.sort_by(|a, b| key(b).cmp(&key(a)).then_with(|| a.base_name.cmp(&b.base_name)));
    groups.truncate(top);
    groups
}
