//! Lowest current listing price per (option value, level) of one option family in one
//! category — the "option x level" price table.

use std::collections::BTreeMap;

use mabi_ah_domain::option::{ItemOption, OptionRegistry, OptionValue};

use crate::error::AppError;
use crate::ports::{AuctionGateway, ListingQuery};
use crate::usecases::query::fetch_pages;

/// One table row: listings having an option of the family with this value and level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionPriceRow {
    /// Effect / enchant name / text value (or the sub type for numeric options).
    pub value: String,
    /// Level, enchant rank or numeric value; `None` for plain text values.
    pub level: Option<String>,
    /// Lowest unit price among those listings.
    pub min_price: u64,
    /// Number of listings.
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionPrices {
    pub category: String,
    pub family: String,
    /// Sorted by value, then level (numeric order; enchant ranks weakest first).
    pub rows: Vec<OptionPriceRow>,
    /// Listings scanned (all of them, with or without the family).
    pub listings: usize,
    pub pages: u32,
    /// More pages existed than were fetched.
    pub truncated: bool,
}

/// `(value, level order, level)` of an option of the family. The order puts numeric
/// levels in numeric order and enchant ranks weakest first; text levels last.
type RowKey = (String, (bool, i64), Option<String>);

fn row_key(reg: &OptionRegistry, o: &ItemOption) -> RowKey {
    let label = || {
        o.sub_type
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| o.option_type.clone())
    };
    match reg.parse_value(o) {
        OptionValue::Leveled { effect, level, .. } => {
            (effect, (false, i64::from(level)), Some(level.to_string()))
        }
        OptionValue::Enchant { name, rank, .. } => (
            name,
            (false, i64::from(rank.ordinal())),
            Some(rank.to_string()),
        ),
        OptionValue::Number(n) => (label(), (false, n), Some(n.to_string())),
        OptionValue::CurrentMax { cur, .. } => (label(), (false, cur), Some(cur.to_string())),
        OptionValue::Text(t) => (t, (true, 0), None),
        _ => (
            o.value.clone().unwrap_or_else(label),
            (true, 0),
            o.value2.clone().filter(|s| !s.trim().is_empty()),
        ),
    }
}

/// Scan current listings of `category` (up to `max_pages` pages) and tabulate options
/// whose type is `family` (e.g. `세공 옵션`, `인챈트`, `에르그`). A listing counts once
/// per distinct row.
pub fn option_prices(
    gw: &mut dyn AuctionGateway,
    reg: &OptionRegistry,
    category: &str,
    family: &str,
    max_pages: u32,
) -> Result<OptionPrices, AppError> {
    let (category, family) = (category.trim(), family.trim());
    if category.is_empty() || family.is_empty() {
        return Err(AppError::Usage(
            "category and option family are required".to_owned(),
        ));
    }
    let query = ListingQuery {
        category: Some(category.to_owned()),
        ..ListingQuery::default()
    };
    let (listings, pages, truncated) = fetch_pages(&query, true, max_pages, |q| gw.listings(q))?;
    let mut table: BTreeMap<RowKey, (u64, u64)> = BTreeMap::new();
    for l in &listings {
        let mut keys: Vec<_> = l
            .options
            .iter()
            .filter(|o| o.option_type.trim() == family)
            .map(|o| row_key(reg, o))
            .collect();
        keys.sort();
        keys.dedup();
        for k in keys {
            let e = table.entry(k).or_insert((l.unit_price, 0));
            e.0 = e.0.min(l.unit_price);
            e.1 += 1;
        }
    }
    let rows = table
        .into_iter()
        .map(|((value, _, level), (min_price, count))| OptionPriceRow {
            value,
            level,
            min_price,
            count,
        })
        .collect();
    Ok(OptionPrices {
        category: category.to_owned(),
        family: family.to_owned(),
        rows,
        listings: listings.len(),
        pages,
        truncated,
    })
}
