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
    let filters = rule
        .filters
        .iter()
        .map(|f| f.parse::<OptionFilter>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(listings
        .iter()
        .filter(|l| {
            l.name.base == rule.base_name
                && rule.max_unit_price.is_none_or(|max| l.unit_price <= max)
                && rule.category.as_deref().is_none_or(|c| l.category == c)
                && filters.iter().all(|f| f.matches(&l.options, reg))
        })
        .map(|l| Alert {
            rule_id: rule.id.clone(),
            dedupe_key: dedupe_key(&rule.id, l),
            listing: l.clone(),
        })
        .collect())
}

fn dedupe_key(rule_id: &str, l: &Listing) -> String {
    let opts = l
        .options
        .iter()
        .map(|o| {
            let part = |v: &Option<String>| v.clone().unwrap_or_default();
            format!(
                "{}/{}/{}/{}/{}",
                o.option_type,
                part(&o.sub_type),
                part(&o.value),
                part(&o.value2),
                part(&o.desc)
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{rule_id}|{}|{}|{}|{opts}",
        l.name.display,
        l.unit_price,
        l.expires_at.unix_timestamp()
    )
}
