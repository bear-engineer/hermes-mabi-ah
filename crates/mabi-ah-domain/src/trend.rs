//! Trend: price/volume time series over UTC buckets (pure).

use time::{Duration, OffsetDateTime, Time, UtcOffset};

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
    let step = match bucket {
        Bucket::Hour => Duration::hours(1),
        Bucket::Day => Duration::days(1),
    };
    let from_utc = from.to_offset(UtcOffset::UTC);
    let floored = match bucket {
        Bucket::Hour => {
            from_utc.replace_time(Time::from_hms(from_utc.hour(), 0, 0).unwrap_or(Time::MIDNIGHT))
        }
        Bucket::Day => from_utc.replace_time(Time::MIDNIGHT),
    };
    let mut points = Vec::new();
    let mut start = floored;
    while start < to {
        let end = start + step;
        let (mut n, mut units, mut value) = (0u64, 0u64, 0u128);
        let (mut min, mut max) = (u64::MAX, 0u64);
        for t in trades.iter().filter(|t| {
            t.traded_at >= from && t.traded_at < to && t.traded_at >= start && t.traded_at < end
        }) {
            n += 1;
            units = units.saturating_add(t.count);
            value += u128::from(t.unit_price) * u128::from(t.count);
            min = min.min(t.unit_price);
            max = max.max(t.unit_price);
        }
        points.push(TrendPoint {
            start,
            trades: n,
            units,
            vwap: (units > 0).then(|| value as f64 / units as f64),
            min: if n == 0 { 0 } else { min },
            max,
            has_gap: gaps.iter().any(|&(gs, ge)| gs < end && ge > start),
        });
        start = end;
    }
    points
}
