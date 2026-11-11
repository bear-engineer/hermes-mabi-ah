//! Trend: price/volume time series over UTC buckets (pure).

use time::OffsetDateTime;

use crate::trade::Trade;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    Hour,
    Day,
}

/// One bucket. `min`/`max` are unit prices and 0 when the bucket has no trades;
/// `vwap` is `None` when `units == 0`.
#[derive(Debug, Clone, PartialEq)]
pub struct TrendPoint {
    pub start: OffsetDateTime,
    pub trades: u64,
    pub units: u64,
    pub vwap: Option<f64>,
    pub min: u64,
    pub max: u64,
    pub has_gap: bool,
}

/// Trades with `from <= traded_at < to`, bucketed in UTC. The first bucket starts
/// at `from` floored to the bucket size. A bucket has `has_gap` when any gap
/// `(start, end)` overlaps it.
pub fn trend(
    trades: &[Trade],
    bucket: Bucket,
    from: OffsetDateTime,
    to: OffsetDateTime,
    gaps: &[(OffsetDateTime, OffsetDateTime)],
) -> Vec<TrendPoint> {
    let _ = (trades, bucket, from, to, gaps);
    todo!()
}
