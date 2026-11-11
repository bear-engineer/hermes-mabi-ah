//! `rank`: top items by volume or value from collected trades.
use clap::{Args, ValueEnum};
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::Clock;
use mabi_ah_app::usecases::compute_rank;
use mabi_ah_domain::rank::RankBy;
use serde_json::{Value, json};

use super::trend::parse_window;
use crate::context::Context;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ByArg {
    Volume,
    Value,
}

#[derive(Debug, Args)]
pub struct RankArgs {
    #[arg(long, value_enum, default_value = "volume")]
    pub by: ByArg,
    /// Look-back window such as 24h or 7d.
    #[arg(long, default_value = "24h")]
    pub window: String,
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub top: usize,
}

pub fn run(ctx: &mut Context, args: &RankArgs) -> Result<Value, AppError> {
    let window = parse_window(&args.window)?;
    let (by, by_name) = match args.by {
        ByArg::Volume => (RankBy::Volume, "volume"),
        ByArg::Value => (RankBy::Value, "value"),
    };
    let category = args.category.as_deref().map(str::trim).filter(|c| !c.is_empty());
    let to = ctx.clock.now();
    let rows = compute_rank(&ctx.store, to - window, to, by, args.top, category)?;
    Ok(json!({
        "by": by_name,
        "window": args.window,
        "category": category,
        "items": rows.iter().enumerate().map(|(i, r)| json!({
            "rank": i + 1,
            "base_name": r.base_name,
            "display_name": r.display_sample,
            "units": r.units,
            "value": r.value,
            "trades": r.trades,
        })).collect::<Vec<_>>(),
    }))
}
