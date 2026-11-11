//! Staleness: how far behind the newest collected trade is (pure).

use time::{Duration, OffsetDateTime};

/// SOT for the default staleness threshold.
pub const DEFAULT_STALE_THRESHOLD: Duration = Duration::minutes(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staleness {
    pub newest_trade_at: Option<OffsetDateTime>,
    pub lag_seconds: Option<i64>,
    pub stale: bool,
}

/// `lag = now - newest` (clamped at 0). Stale when there is no trade at all or
/// the lag is strictly greater than `threshold`.
pub fn assess(newest: Option<OffsetDateTime>, now: OffsetDateTime, threshold: Duration) -> Staleness {
    let _ = (newest, now, threshold);
    todo!()
}
