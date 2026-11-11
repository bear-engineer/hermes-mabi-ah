//! `search`: keyword search over current listings.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::search_listings_in;
use serde_json::{Value, json};

use super::common::{FilterArgs, checked_filter, listing_json, query_env, stats_json};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Keywords (1 to 10).
    #[arg(required = true, num_args = 1..)]
    pub words: Vec<String>,
    /// Auction category (e.g. 검).
    #[arg(long)]
    pub category: Option<String>,
    #[command(flatten)]
    pub filter: FilterArgs,
}

pub fn run(ctx: &mut Context, args: &SearchArgs) -> Result<Value, AppError> {
    let filter = checked_filter(ctx, &args.filter)?;
    let mut gw = ctx.gateway()?;
    let r = search_listings_in(
        &mut gw,
        &args.words,
        args.category.as_deref(),
        &filter,
        &mut query_env(ctx),
    )?;
    Ok(json!({
        "items": r.listings.iter().map(listing_json).collect::<Vec<_>>(),
        "count": r.listings.len(),
        "pages": r.pages,
        "truncated": r.truncated,
        "stats": stats_json(&r.stats),
    }))
}
