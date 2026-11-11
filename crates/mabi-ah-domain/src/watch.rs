//! Watch rules: evaluate listings against a saved rule (pure).

use crate::listing::Listing;
use crate::option::{FilterParseError, OptionFilter, OptionRegistry};

/// `filters` are `OptionFilter` source strings; all must match (each matches if
/// any option satisfies it).
#[derive(Debug, Clone, PartialEq)]
pub struct WatchRule {
    pub id: String,
    pub base_name: String,
    pub max_unit_price: Option<u64>,
    pub category: Option<String>,
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    pub rule_id: String,
    /// `rule_id|display|unit_price|expires_at(unix)|options`, options being
    /// `type/sub/value/value2/desc` (absent parts empty) joined by `;`.
    pub dedupe_key: String,
    pub listing: Listing,
}

/// Listings matching the rule (same `ItemName::base`, price cap, category and
/// every option filter), in input order. A malformed filter is an error.
pub fn evaluate(
    rule: &WatchRule,
    listings: &[Listing],
    reg: &OptionRegistry,
) -> Result<Vec<Alert>, FilterParseError> {
    let _ = (rule, listings, reg);
    todo!()
}
