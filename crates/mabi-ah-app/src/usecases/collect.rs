//! Incremental trade collection with watermark and gap detection.
use time::{Duration, OffsetDateTime};

use crate::error::AppError;
use crate::ports::{AuctionGateway, Clock, ListingQuery, TradeStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectReport {
    pub pages: u32,
    pub fetched: usize,
    pub inserted: usize,
    pub reached_watermark: bool,
    pub gap: Option<(OffsetDateTime, OffsetDateTime)>,
}

/// Pages `history` newest-first until a page holds an already-stored trade.
/// Without overlap (and with stored data) the uncovered span is recorded as a
/// gap: `(newest stored, oldest fetched)`. At most `max_calls` page requests.
pub fn collect_trades(
    gw: &mut dyn AuctionGateway,
    store: &mut dyn TradeStore,
    _clock: &dyn Clock,
    max_calls: u32,
) -> Result<CollectReport, AppError> {
    let newest_stored = store.newest_trade(None)?.map(|t| t.traded_at);
    let mut report = CollectReport {
        pages: 0,
        fetched: 0,
        inserted: 0,
        reached_watermark: false,
        gap: None,
    };
    let mut oldest_fetched: Option<OffsetDateTime> = None;
    let mut query = ListingQuery::default();
    while report.pages < max_calls {
        let page = gw.trades(&query)?;
        report.pages += 1;
        report.fetched += page.items.len();
        let bounds = page.items.iter().map(|t| t.traded_at);
        if let (Some(lo), Some(hi)) = (bounds.clone().min(), bounds.max()) {
            oldest_fetched = Some(oldest_fetched.map_or(lo, |o| o.min(lo)));
            let stored = store.trades_between(lo, hi + Duration::nanoseconds(1), None)?;
            report.reached_watermark = page
                .items
                .iter()
                .any(|t| stored.iter().any(|s| s.trade_id == t.trade_id));
        }
        report.inserted += store.insert_trades(&page.items)?;
        match page.next_cursor {
            Some(c) if !report.reached_watermark => query.cursor = Some(c),
            _ => break,
        }
    }
    if let (false, Some(newest), Some(oldest)) =
        (report.reached_watermark, newest_stored, oldest_fetched)
    {
        if oldest > newest {
            store.record_gap(newest, oldest)?;
            report.gap = Some((newest, oldest));
        }
    }
    Ok(report)
}
