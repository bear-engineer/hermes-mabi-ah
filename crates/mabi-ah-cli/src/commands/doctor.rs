//! `doctor`: environment and data health check. Never prints the key.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::doctor;
use mabi_ah_nexon::key::ApiKey;
use serde_json::{Value, json};

use crate::config::resolve_key_file;
use crate::context::Context;
use crate::output::rfc3339;

#[derive(Debug, Args)]
pub struct DoctorArgs {}

pub fn run(ctx: &mut Context, _args: &DoctorArgs) -> Result<Value, AppError> {
    let key_present = ApiKey::load(
        resolve_key_file(ctx.cfg.key_file.clone()).as_deref(),
        std::env::var("NEXON_OPEN_API_KEY").ok(),
    )
    .is_ok();
    let r = doctor(
        &ctx.clock,
        &ctx.store,
        &ctx.ledger,
        ctx.cfg.daily_cap(),
        key_present,
    )?;
    Ok(json!({
        "key_present": r.key_present,
        "db_path": ctx.cfg.db_path.display().to_string(),
        "db_exists": ctx.cfg.db_path.exists(),
        "staleness": {
            "newest_trade_at": r.staleness.newest_trade_at.map(rfc3339),
            "lag_seconds": r.staleness.lag_seconds,
            "stale": r.staleness.stale,
        },
        "budget": {
            "tier": ctx.cfg.tier_name(),
            "used_today": r.used_today,
            "limit_today": r.limit,
        },
    }))
}
