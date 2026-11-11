//! `horn`: recent horn-of-bugle world shouts.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::query_horn;
use serde_json::{Value, json};

use crate::context::Context;
use crate::output::rfc3339;

#[derive(Debug, Args)]
pub struct HornArgs {
    #[arg(long)]
    pub server: String,
    /// Replace character names with `***`.
    #[arg(long)]
    pub redact_names: bool,
}

pub fn run(ctx: &mut Context, args: &HornArgs) -> Result<Value, AppError> {
    let mut gw = ctx.gateway()?;
    let messages = query_horn(&mut gw, &args.server, args.redact_names)?;
    Ok(json!({
        "server": args.server,
        "count": messages.len(),
        "messages": messages.iter().map(|m| json!({
            "character_name": m.character_name,
            "message": m.message,
            "sent_at": rfc3339(m.sent_at),
        })).collect::<Vec<_>>(),
    }))
}
