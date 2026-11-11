//! `trades`: completed auction trades from the API (not the local store).
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::query_trades_in;
use serde_json::{Value, json};

use super::common::{QueryArgs, checked_filter, query_env, stats_json, trade_json};
use crate::context::Context;

pub fn run(ctx: &mut Context, args: &QueryArgs) -> Result<Value, AppError> {
    let filter = checked_filter(ctx, &args.filter)?;
    let query = args.query()?;
    let mut gw = ctx.gateway()?;
    let r = query_trades_in(
        &mut gw,
        &query,
        &filter,
        &mut query_env(ctx),
        args.all_pages,
        args.max_pages,
    )?;
    let mut data = json!({
        "items": r.trades.iter().map(trade_json).collect::<Vec<_>>(),
        "count": r.trades.len(),
        "pages": r.pages,
        "truncated": r.truncated,
    });
    if args.stats {
        data["stats"] = stats_json(&r.stats);
    }
    Ok(data)
}
