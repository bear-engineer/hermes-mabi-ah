//! `collect`: pull recent trades into the local store.
use std::io::Write;

use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{Clock, TradeStore};
use mabi_ah_app::usecases::{CollectReport, collect_trades};
use mabi_ah_store::CollectLock;
use serde_json::{Value, json};
use time::Duration;

use crate::context::Context;
use crate::output::rfc3339;

#[derive(Debug, Args)]
pub struct CollectArgs {
    /// Maximum history pages (API calls) for this run.
    #[arg(long, default_value_t = 20)]
    pub max_calls: u32,
    /// Delete stored trades older than this many days.
    #[arg(long, default_value_t = 30)]
    pub retain_days: u32,
    /// Print nothing on success (for cron).
    #[arg(long)]
    pub quiet: bool,
}

/// Writes `text` (nothing when empty) to stdout and ends the process with
/// success, bypassing the JSON envelope. Callers must have dropped every lock
/// first: `exit` skips destructors.
pub fn finish_raw(text: &str) -> ! {
    let mut out = std::io::stdout().lock();
    if !text.is_empty() {
        let _ = writeln!(out, "{text}");
    }
    let _ = out.flush();
    std::process::exit(0)
}

fn collect_once(ctx: &mut Context, args: &CollectArgs) -> Result<(CollectReport, usize), AppError> {
    let _lock = CollectLock::acquire(&ctx.cfg.db_path)?;
    let mut gw = ctx.gateway()?;
    let report = collect_trades(&mut gw, &mut ctx.store, &ctx.clock, args.max_calls)?;
    let cutoff = ctx.clock.now() - Duration::days(i64::from(args.retain_days));
    let purged = ctx.store.purge_older_than(cutoff)?;
    Ok((report, purged))
}

pub fn run(ctx: &mut Context, args: &CollectArgs) -> Result<Value, AppError> {
    let (r, purged) = collect_once(ctx, args)?;
    if args.quiet {
        finish_raw("");
    }
    Ok(json!({
        "pages": r.pages,
        "fetched": r.fetched,
        "inserted": r.inserted,
        "reached_watermark": r.reached_watermark,
        "gap": r.gap.map(|(a, b)| json!({"start": rfc3339(a), "end": rfc3339(b)})),
        "purged": purged,
    }))
}
