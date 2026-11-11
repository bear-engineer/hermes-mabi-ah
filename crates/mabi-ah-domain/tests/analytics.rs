#![allow(clippy::unwrap_used)]
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::trend::{Bucket, TrendPoint, trend};
use time::OffsetDateTime;
use time::macros::datetime;

fn trade(id: &str, base: &str, cat: &str, price: u64, count: u64, at: OffsetDateTime) -> Trade {
    Trade {
        trade_id: id.into(),
        name: ItemName::new(base, format!("{base} 표시")),
        count,
        category: cat.into(),
        unit_price: price,
        traded_at: at,
        options: vec![],
    }
}

// ---------- trend ----------

#[test]
fn trend_hourly_hand_computed() {
    let t = vec![
        trade("1", "a", "c", 100, 2, datetime!(2026-11-10 00:10 UTC)),
        trade("2", "a", "c", 200, 3, datetime!(2026-11-10 00:50 UTC)),
        trade("3", "a", "c", 50, 1, datetime!(2026-11-10 02:59:59 UTC)),
        trade("4", "a", "c", 999, 1, datetime!(2026-11-10 03:00 UTC)), // == to: excluded
        trade("5", "a", "c", 999, 1, datetime!(2026-11-09 23:59 UTC)), // < from: excluded
    ];
    let gaps = [(datetime!(2026-11-10 01:30 UTC), datetime!(2026-11-10 02:10 UTC))];
    let got = trend(&t, Bucket::Hour, datetime!(2026-11-10 00:00 UTC), datetime!(2026-11-10 03:00 UTC), &gaps);
    let want = vec![
        TrendPoint { start: datetime!(2026-11-10 00:00 UTC), trades: 2, units: 5, vwap: Some(160.0), min: 100, max: 200, has_gap: false },
        TrendPoint { start: datetime!(2026-11-10 01:00 UTC), trades: 0, units: 0, vwap: None, min: 0, max: 0, has_gap: true },
        TrendPoint { start: datetime!(2026-11-10 02:00 UTC), trades: 1, units: 1, vwap: Some(50.0), min: 50, max: 50, has_gap: true },
    ];
    assert_eq!(got, want);
}

#[test]
fn trend_daily_floors_from_and_uses_utc() {
    let t = vec![
        trade("1", "a", "c", 10, 1, datetime!(2026-11-10 01:00 UTC)), // before from
        trade("2", "a", "c", 10, 1, datetime!(2026-11-10 23:00 UTC)),
        trade("3", "a", "c", 30, 1, datetime!(2026-11-11 01:00 UTC)),
    ];
    let got = trend(&t, Bucket::Day, datetime!(2026-11-10 06:00 UTC), datetime!(2026-11-11 06:00 UTC), &[]);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].start, datetime!(2026-11-10 00:00 UTC));
    assert_eq!((got[0].trades, got[0].vwap), (1, Some(10.0)));
    assert_eq!(got[1].start, datetime!(2026-11-11 00:00 UTC));
    assert_eq!((got[1].trades, got[1].vwap), (1, Some(30.0)));
}

#[test]
fn trend_empty_range_is_empty() {
    let at = datetime!(2026-11-10 00:00 UTC);
    assert!(trend(&[], Bucket::Hour, at, at, &[]).is_empty());
}
