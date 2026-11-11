//! `option-prices`: lowest current listing price per (option value, level) of one
//! option family in a category.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::option_prices;
use mabi_ah_domain::price_ko::price_ko;
use serde_json::{Value, json};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct OptionPricesArgs {
    /// Auction category (e.g. 토템).
    #[arg(long)]
    pub category: String,
    /// Option family = exact option_type (e.g. 세공 옵션, 인챈트, 에르그, 토템 효과).
    #[arg(long)]
    pub family: String,
    /// Maximum listing pages (API calls) to scan.
    #[arg(long, default_value_t = 10)]
    pub max_pages: u32,
}

pub fn run(ctx: &mut Context, args: &OptionPricesArgs) -> Result<Value, AppError> {
    let mut gw = ctx.gateway()?;
    let r = option_prices(
        &mut gw,
        &ctx.options,
        &args.category,
        &args.family,
        args.max_pages,
    )?;
    let rows: Vec<Value> = r
        .rows
        .iter()
        .map(|row| {
            json!({
                "value": row.value,
                "level": row.level,
                "min_price": row.min_price,
                "min_price_ko": price_ko(row.min_price),
                "count": row.count,
            })
        })
        .collect();
    Ok(json!({
        "category": r.category,
        "family": r.family,
        "rows": rows,
        "count": r.rows.len(),
        "listings": r.listings,
        "pages": r.pages,
        "truncated": r.truncated,
    }))
}
