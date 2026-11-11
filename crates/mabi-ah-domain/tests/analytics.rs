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
    let gaps = [(
        datetime!(2026-11-10 01:30 UTC),
        datetime!(2026-11-10 02:10 UTC),
    )];
    let got = trend(
        &t,
        Bucket::Hour,
        datetime!(2026-11-10 00:00 UTC),
        datetime!(2026-11-10 03:00 UTC),
        &gaps,
    );
    let want = vec![
        TrendPoint {
            start: datetime!(2026-11-10 00:00 UTC),
            trades: 2,
            units: 5,
            vwap: Some(160.0),
            min: 100,
            max: 200,
            has_gap: false,
        },
        TrendPoint {
            start: datetime!(2026-11-10 01:00 UTC),
            trades: 0,
            units: 0,
            vwap: None,
            min: 0,
            max: 0,
            has_gap: true,
        },
        TrendPoint {
            start: datetime!(2026-11-10 02:00 UTC),
            trades: 1,
            units: 1,
            vwap: Some(50.0),
            min: 50,
            max: 50,
            has_gap: true,
        },
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
    let got = trend(
        &t,
        Bucket::Day,
        datetime!(2026-11-10 06:00 UTC),
        datetime!(2026-11-11 06:00 UTC),
        &[],
    );
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

// ---------- rank ----------
use mabi_ah_domain::rank::{RankBy, RankEntry, rank};

fn rank_fixture() -> Vec<Trade> {
    let at = datetime!(2026-11-10 00:00 UTC);
    vec![
        trade("1", "a", "W", 100, 2, at),
        trade("2", "@a", "W", 300, 1, at), // same base as "a"
        trade("3", "b", "A", 50, 10, at),
        trade("4", "c", "W", 500, 1, at),
        trade("5", "d", "W", 10, 3, at),
    ]
}

fn names(v: &[RankEntry]) -> Vec<&str> {
    v.iter().map(|e| e.base_name.as_str()).collect()
}

#[test]
fn rank_by_volume_groups_base_and_breaks_ties_by_name() {
    // units: a=3 b=10 c=1 d=3
    let got = rank(&rank_fixture(), RankBy::Volume, 10, None);
    assert_eq!(names(&got), ["b", "a", "d", "c"]);
    assert_eq!(
        got[1],
        RankEntry {
            base_name: "a".into(),
            display_sample: "a 표시".into(),
            units: 3,
            value: 500,
            trades: 2
        }
    );
}

#[test]
fn rank_by_value_ties_and_top() {
    // value: a=500 b=500 c=500 d=30
    let got = rank(&rank_fixture(), RankBy::Value, 2, None);
    assert_eq!(names(&got), ["a", "b"]);
}

#[test]
fn rank_category_filter() {
    let got = rank(&rank_fixture(), RankBy::Volume, 10, Some("W"));
    assert_eq!(names(&got), ["a", "d", "c"]);
    assert!(rank(&rank_fixture(), RankBy::Volume, 10, Some("none")).is_empty());
    assert!(rank(&rank_fixture(), RankBy::Volume, 0, None).is_empty());
}

// ---------- watch ----------
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::{ItemOption, OptionRegistry};
use mabi_ah_domain::watch::{WatchRule, evaluate};

fn opt(t: &str, v: &str) -> ItemOption {
    ItemOption {
        option_type: t.into(),
        sub_type: None,
        value: Some(v.into()),
        value2: None,
        desc: None,
    }
}

fn listing(raw: &str, cat: &str, price: u64, options: Vec<ItemOption>) -> Listing {
    Listing {
        name: ItemName::new(raw, format!("{} 표시", raw.trim_start_matches('@'))),
        count: 1,
        category: cat.into(),
        unit_price: price,
        expires_at: datetime!(2026-11-12 00:00 UTC),
        options,
    }
}

fn rule() -> WatchRule {
    WatchRule {
        id: "r1".into(),
        base_name: "a".into(),
        max_unit_price: Some(150),
        category: Some("W".into()),
        filters: vec!["x~foo".into()],
    }
}

#[test]
fn watch_matches_only_listings_satisfying_all_conditions() {
    let reg = OptionRegistry::default();
    let ls = vec![
        listing("@a", "W", 100, vec![opt("x", "foobar")]), // ok (base a, <=150)
        listing("a", "W", 150, vec![opt("x", "foo")]),     // ok (boundary)
        listing("b", "W", 100, vec![opt("x", "foo")]),     // wrong base
        listing("a", "W", 151, vec![opt("x", "foo")]),     // too pricey
        listing("a", "A", 100, vec![opt("x", "foo")]),     // wrong category
        listing("a", "W", 100, vec![opt("x", "bar")]),     // filter fails
        listing("a", "W", 100, vec![]),                    // no options
    ];
    let got = evaluate(&rule(), &ls, &reg).unwrap();
    let prices: Vec<u64> = got.iter().map(|a| a.listing.unit_price).collect();
    assert_eq!(prices, [100, 150]);
    assert!(got.iter().all(|a| a.rule_id == "r1"));
}

#[test]
fn watch_dedupe_key_is_deterministic_and_discriminating() {
    let reg = OptionRegistry::default();
    let l = listing("@a", "W", 100, vec![opt("x", "foo")]);
    let r = WatchRule {
        filters: vec![],
        ..rule()
    };
    let a = evaluate(&r, std::slice::from_ref(&l), &reg).unwrap();
    assert_eq!(a[0].dedupe_key, "r1|a 표시|100|1794441600|x//foo//");
    let again = evaluate(&r, std::slice::from_ref(&l), &reg).unwrap();
    assert_eq!(a[0].dedupe_key, again[0].dedupe_key);
    let l2 = listing("@a", "W", 101, vec![opt("x", "foo")]);
    let b = evaluate(&r, &[l2], &reg).unwrap();
    assert_ne!(a[0].dedupe_key, b[0].dedupe_key);
}

#[test]
fn watch_no_constraints_matches_by_base_only_and_bad_filter_errors() {
    let reg = OptionRegistry::default();
    let bare = WatchRule {
        id: "r2".into(),
        base_name: "a".into(),
        max_unit_price: None,
        category: None,
        filters: vec![],
    };
    assert_eq!(
        evaluate(&bare, &[listing("a", "Z", 9_999, vec![])], &reg)
            .unwrap()
            .len(),
        1
    );
    let bad = WatchRule {
        filters: vec!["nonsense".into()],
        ..bare
    };
    assert!(evaluate(&bad, &[listing("a", "Z", 1, vec![])], &reg).is_err());
}

// ---------- staleness ----------
use mabi_ah_domain::staleness::{DEFAULT_STALE_THRESHOLD, Staleness, assess};
use time::Duration;

#[test]
fn staleness_default_threshold_is_15_minutes() {
    assert_eq!(DEFAULT_STALE_THRESHOLD, Duration::minutes(15));
}

#[test]
fn staleness_table() {
    let now = datetime!(2026-11-10 10:10 UTC);
    let th = DEFAULT_STALE_THRESHOLD;
    let cases = [
        (Some(datetime!(2026-11-10 10:00 UTC)), Some(600), false),
        (Some(datetime!(2026-11-10 09:55 UTC)), Some(900), false), // exactly threshold
        (Some(datetime!(2026-11-10 09:54:59 UTC)), Some(901), true),
        (Some(datetime!(2026-11-10 10:11 UTC)), Some(0), false), // future clamps to 0
    ];
    for (newest, lag, stale) in cases {
        assert_eq!(
            assess(newest, now, th),
            Staleness {
                newest_trade_at: newest,
                lag_seconds: lag,
                stale
            }
        );
    }
}

#[test]
fn staleness_no_trades_is_stale() {
    let s = assess(
        None,
        datetime!(2026-11-10 10:10 UTC),
        DEFAULT_STALE_THRESHOLD,
    );
    assert_eq!(
        s,
        Staleness {
            newest_trade_at: None,
            lag_seconds: None,
            stale: true
        }
    );
}
