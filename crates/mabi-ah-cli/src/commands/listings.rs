//! `listings`: current auction listings.
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::query_listings;
use serde_json::{Value, json};

use super::common::{QueryArgs, listing_json, parse_filters, stats_json};
use crate::context::Context;

pub fn run(ctx: &mut Context, args: &QueryArgs) -> Result<Value, AppError> {
    let filters = parse_filters(&args.option)?;
    let query = args.query()?;
    let mut gw = ctx.gateway()?;
    let r = query_listings(
        &mut gw,
        &query,
        &filters,
        &ctx.options,
        &mut ctx.categories,
        args.all_pages,
        args.max_pages,
    )?;
    let mut data = json!({
        "items": r.listings.iter().map(listing_json).collect::<Vec<_>>(),
        "count": r.listings.len(),
        "pages": r.pages,
        "truncated": r.truncated,
    });
    if args.stats {
        data["stats"] = stats_json(&r.stats);
    }
    Ok(data)
}
