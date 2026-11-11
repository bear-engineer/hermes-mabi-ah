//! `collect`: pull recent trades into the local store.
use std::io::Write;

use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{AuctionGateway, CatalogStore, Clock, ListingQuery, Page, TradeStore};
use mabi_ah_app::usecases::{CollectReport, collect_trades};
use mabi_ah_domain::catalog;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::ItemOption;
use mabi_ah_domain::trade::Trade;
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

/// Gateway wrapper that remembers `(category, options)` of every fetched trade so
/// the CLI can record catalog observations after the collector ran.
struct Observing<'g> {
    inner: &'g mut dyn AuctionGateway,
    seen: Vec<(String, Vec<ItemOption>)>,
}

impl AuctionGateway for Observing<'_> {
    fn listings(&mut self, query: &ListingQuery) -> Result<Page<Listing>, AppError> {
        self.inner.listings(query)
    }
    fn trades(&mut self, query: &ListingQuery) -> Result<Page<Trade>, AppError> {
        let page = self.inner.trades(query)?;
        self.seen.extend(
            page.items
                .iter()
                .map(|t| (t.category.clone(), t.options.clone())),
        );
        Ok(page)
    }
    fn search(
        &mut self,
        keywords: &[String],
        cursor: Option<&str>,
    ) -> Result<Page<Listing>, AppError> {
        self.inner.search(keywords, cursor)
    }
}

fn collect_once(ctx: &mut Context, args: &CollectArgs) -> Result<(CollectReport, usize), AppError> {
    let _lock = CollectLock::acquire(&ctx.cfg.db_path)?;
    let mut client = ctx.gateway()?;
    let mut gw = Observing {
        inner: &mut client,
        seen: Vec::new(),
    };
    let report = collect_trades(&mut gw, &mut ctx.store, &ctx.clock, args.max_calls)?;
    let obs: Vec<_> = gw
        .seen
        .iter()
        .flat_map(|(cat, opts)| catalog::extract(&ctx.options, cat, opts))
        .collect();
    if !obs.is_empty() {
        CatalogStore::record_catalog(&mut ctx.store, &obs, ctx.clock.now())?;
    }
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
