//! `budget`: today's API call usage.
use clap::Args;
use mabi_ah_app::error::AppError;
use serde_json::{Value, json};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct BudgetArgs {}

pub fn run(ctx: &mut Context, _args: &BudgetArgs) -> Result<Value, AppError> {
    let used = ctx.used_today()?;
    let limit = ctx.cfg.daily_cap();
    Ok(json!({
        "tier": ctx.cfg.tier_name(),
        "used_today": used,
        "limit_today": limit,
        "remaining": limit.saturating_sub(used),
    }))
}
