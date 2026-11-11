//! `config`: resolved paths and settings. Never prints secrets.
use clap::Args;
use mabi_ah_app::error::AppError;
use serde_json::{Value, json};

use crate::config::{Format, resolve_key_file};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct ConfigArgs {}

pub fn run(ctx: &mut Context, _args: &ConfigArgs) -> Result<Value, AppError> {
    let c = &ctx.cfg;
    let key_file = resolve_key_file(c.key_file.clone());
    Ok(json!({
        "db_path": c.db_path.display().to_string(),
        "data_dir": mabi_ah_store::data_dir().display().to_string(),
        "key_file": key_file.as_ref().map(|p| p.display().to_string()),
        "key_file_exists": key_file.as_ref().is_some_and(|p| p.exists()),
        "key_env_set": std::env::var_os("NEXON_OPEN_API_KEY").is_some(),
        "base_url": c.base_url,
        "tier": c.tier_name(),
        "daily_cap": c.daily_cap(),
        "force_budget": c.force_budget,
        "format": match c.format { Format::Json => "json", Format::Table => "table" },
    }))
}
