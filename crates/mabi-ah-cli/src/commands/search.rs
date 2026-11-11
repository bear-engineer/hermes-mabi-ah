//! `search`: keyword search over current listings.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::search_listings;
use serde_json::{Value, json};

use super::common::{listing_json, parse_filters, stats_json};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Keywords (1 to 10).
    #[arg(required = true, num_args = 1..)]
    pub words: Vec<String>,
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long = "option", short = 'o')]
    pub option: Vec<String>,
}

pub fn run(ctx: &mut Context, args: &SearchArgs) -> Result<Value, AppError> {
    let filters = parse_filters(&args.option)?;
    let mut gw = ctx.gateway()?;
    let r = search_listings(
        &mut gw,
        &args.words,
        args.category.as_deref(),
        &filters,
        &ctx.options,
        &mut ctx.categories,
    )?;
    Ok(json!({
        "items": r.listings.iter().map(listing_json).collect::<Vec<_>>(),
        "count": r.listings.len(),
        "pages": r.pages,
        "truncated": r.truncated,
        "stats": stats_json(&r.stats),
    }))
}
