//! Shared argument parsing and JSON shaping for the query commands.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::ListingQuery;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::{ItemOption, OptionFilter};
use mabi_ah_domain::price::PriceStats;
use mabi_ah_domain::trade::Trade;
use serde_json::{Value, json};

use crate::output::rfc3339;

/// Arguments shared by `listings` and `trades`.
#[derive(Debug, Args)]
pub struct QueryArgs {
    /// Exact item name (a leading '@' is part of the raw API name).
    #[arg(long)]
    pub name: Option<String>,
    /// Auction category (e.g. 검).
    #[arg(long)]
    pub category: Option<String>,
    /// Option filter `<type>[:<sub>](>=|<=|=|~)<value>`; repeatable.
    #[arg(long = "option", short = 'o')]
    pub option: Vec<String>,
    /// Include price statistics.
    #[arg(long)]
    pub stats: bool,
    /// Follow next_cursor until the end (bounded by --max-pages).
    #[arg(long)]
    pub all_pages: bool,
    #[arg(long, default_value_t = 10)]
    pub max_pages: u32,
}

impl QueryArgs {
    pub fn query(&self) -> Result<ListingQuery, AppError> {
        let (name, category) = (clean(&self.name), clean(&self.category));
        if name.is_none() && category.is_none() {
            return Err(AppError::Usage(
                "--name or --category is required".to_owned(),
            ));
        }
        Ok(ListingQuery {
            name: name.map(str::to_owned),
            category: category.map(str::to_owned),
            cursor: None,
        })
    }
}

fn clean(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

pub fn parse_filters(raw: &[String]) -> Result<Vec<OptionFilter>, AppError> {
    raw.iter()
        .map(|s| {
            s.parse::<OptionFilter>()
                .map_err(|e| AppError::Usage(e.to_string()))
        })
        .collect()
}

pub fn option_json(o: &ItemOption) -> Value {
    json!({
        "option_type": o.option_type,
        "sub_type": o.sub_type,
        "value": o.value,
        "value2": o.value2,
        "desc": o.desc,
    })
}

pub fn listing_json(l: &Listing) -> Value {
    json!({
        "item_name": l.name.raw,
        "base_name": l.name.base,
        "display_name": l.name.display,
        "count": l.count,
        "category": l.category,
        "unit_price": l.unit_price,
        "total_price": l.total_price(),
        "expires_at": rfc3339(l.expires_at),
        "options": l.options.iter().map(option_json).collect::<Vec<_>>(),
    })
}

pub fn trade_json(t: &Trade) -> Value {
    json!({
        "trade_id": t.trade_id,
        "item_name": t.name.raw,
        "base_name": t.name.base,
        "display_name": t.name.display,
        "count": t.count,
        "category": t.category,
        "unit_price": t.unit_price,
        "total_price": t.unit_price.saturating_mul(t.count),
        "traded_at": rfc3339(t.traded_at),
        "options": t.options.iter().map(option_json).collect::<Vec<_>>(),
    })
}

pub fn stats_json(s: &Option<PriceStats>) -> Value {
    match s {
        None => Value::Null,
        Some(s) => json!({
            "min": s.min, "p25": s.p25, "median": s.median, "p75": s.p75, "max": s.max,
            "listings": s.listings, "units": s.units,
        }),
    }
}
