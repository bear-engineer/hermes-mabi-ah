//! `listings`: current auction listings.
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::query_listings_in;
use serde_json::{Value, json};

use super::common::{QueryArgs, checked_filter, listing_json, query_env, stats_json};
use crate::context::Context;

pub fn run(ctx: &mut Context, args: &QueryArgs) -> Result<Value, AppError> {
    let filter = checked_filter(ctx, &args.filter)?;
    let query = args.query()?;
    let mut gw = ctx.gateway()?;
    let r = query_listings_in(
        &mut gw,
        &query,
        &filter,
        &mut query_env(ctx),
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
