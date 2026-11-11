//! Incremental trade collection with watermark and gap detection.
use time::OffsetDateTime;

use crate::error::AppError;
use crate::ports::{AuctionGateway, Clock, TradeStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectReport {
    pub pages: u32,
    pub fetched: usize,
    pub inserted: usize,
    pub reached_watermark: bool,
    pub gap: Option<(OffsetDateTime, OffsetDateTime)>,
}

pub fn collect_trades(
    _gw: &mut dyn AuctionGateway,
    _store: &mut dyn TradeStore,
    _clock: &dyn Clock,
    _max_calls: u32,
) -> Result<CollectReport, AppError> {
    todo!("node A green: collect_trades")
}
