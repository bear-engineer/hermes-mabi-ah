use mabi_ah_app::ports::{BudgetLedger, Outbox, TradeStore, WatchStore};
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::ItemOption;
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::watch::{Alert, WatchRule};
use mabi_ah_store::{CollectLock, SqliteBudget, SqliteStore, data_dir};
use time::OffsetDateTime;
use time::macros::{date, datetime};

fn trade(id: &str, name: &str, cat: &str, at: OffsetDateTime) -> Trade {
    Trade {
        trade_id: id.into(),
        name: ItemName::new(name, name.trim_start_matches('@')),
        count: 2,
        category: cat.into(),
        unit_price: 1000,
        traded_at: at,
        options: vec![ItemOption {
            option_type: "enchant".into(),
            sub_type: Some("접두".into()),
            value: Some("1".into()),
            value2: None,
            desc: None,
        }],
    }
}

fn alert(rule: &str, key: &str) -> Alert {
    Alert {
        rule_id: rule.into(),
        dedupe_key: key.into(),
        listing: Listing {
            name: ItemName::new("@테스트검", "테스트검"),
            count: 1,
            category: "검".into(),
            unit_price: 500,
            expires_at: datetime!(2026-11-12 00:00 UTC),
            options: vec![],
        },
    }
}

fn open() -> (tempfile::TempDir, std::path::PathBuf, SqliteStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nested/deeper/ah.db");
    let store = SqliteStore::open(&path).expect("open");
    (dir, path, store)
}

#[test]
fn open_creates_parent_dirs_and_reopens() {
    let (_d, path, store) = open();
    assert!(path.exists());
    drop(store);
    SqliteStore::open(&path).expect("reopen migrates idempotently");
}

#[test]
fn overlapping_batches_store_unique_trade_ids() {
    let (_d, _p, mut s) = open();
    let t0 = datetime!(2026-11-11 00:00 UTC);
    let a = vec![
        trade("1", "@검", "검", t0),
        trade("2", "@검", "검", t0 + time::Duration::minutes(1)),
    ];
    let b = vec![
        trade("2", "@검", "검", t0 + time::Duration::minutes(1)),
        trade("3", "@검", "검", t0 + time::Duration::minutes(2)),
        trade("3", "@검", "검", t0 + time::Duration::minutes(2)),
    ];
    assert_eq!(s.insert_trades(&a).unwrap(), 2);
    assert_eq!(s.insert_trades(&b).unwrap(), 1);
    let all = s
        .trades_between(t0, t0 + time::Duration::hours(1), None)
        .unwrap();
    let ids: Vec<_> = all.iter().map(|t| t.trade_id.as_str()).collect();
    assert_eq!(ids, ["1", "2", "3"]);
    assert_eq!(all[0], a[0], "trade roundtrips including options");
}

#[test]
fn trades_between_is_half_open_and_filters_base_name() {
    let (_d, _p, mut s) = open();
    let t0 = datetime!(2026-11-11 00:00 UTC);
    s.insert_trades(&[
        trade("1", "@검", "검", t0),
        trade("2", "방패", "방패", t0 + time::Duration::minutes(5)),
    ])
    .unwrap();
    assert_eq!(s.trades_between(t0, t0 + time::Duration::minutes(5), None).unwrap().len(), 1);
    let only = s.trades_between(t0, t0 + time::Duration::hours(1), Some("검")).unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].name.raw, "@검");
}

#[test]
fn newest_trade_by_category_and_discovered_categories() {
    let (_d, _p, mut s) = open();
    let t0 = datetime!(2026-11-11 00:00 UTC);
    assert!(s.newest_trade(None).unwrap().is_none());
    s.insert_trades(&[
        trade("1", "@검", "검", t0),
        trade("2", "@검", "검", t0 + time::Duration::minutes(3)),
        trade("3", "방패", "방패", t0 + time::Duration::minutes(1)),
    ])
    .unwrap();
    assert_eq!(s.newest_trade(None).unwrap().unwrap().trade_id, "2");
    assert_eq!(s.newest_trade(Some("방패")).unwrap().unwrap().trade_id, "3");
    assert!(s.newest_trade(Some("없음")).unwrap().is_none());
    let mut cats = s.discovered_categories().unwrap();
    cats.sort();
    assert_eq!(cats, ["검", "방패"]);
}

#[test]
fn gaps_roundtrip_and_overlap() {
    let (_d, _p, mut s) = open();
    let t = datetime!(2026-11-11 00:00 UTC);
    let m = |n| t + time::Duration::minutes(n);
    s.record_gap(m(10), m(20)).unwrap();
    s.record_gap(m(30), m(40)).unwrap();
    assert_eq!(s.gaps_between(m(0), m(60)).unwrap(), vec![(m(10), m(20)), (m(30), m(40))]);
    assert_eq!(s.gaps_between(m(15), m(25)).unwrap(), vec![(m(10), m(20))]);
    assert!(s.gaps_between(m(20), m(30)).unwrap().is_empty());
}

#[test]
fn watch_rules_add_replace_list_remove() {
    let (_d, _p, mut s) = open();
    let mut r = WatchRule {
        id: "r1".into(),
        base_name: "검".into(),
        max_unit_price: Some(100),
        category: None,
        filters: vec!["enchant=1".into()],
    };
    s.add(&r).unwrap();
    r.max_unit_price = Some(200);
    r.category = Some("검".into());
    s.add(&r).unwrap();
    assert_eq!(s.list().unwrap(), vec![r]);
    assert!(s.remove("r1").unwrap());
    assert!(!s.remove("r1").unwrap());
    assert!(s.list().unwrap().is_empty());
}

#[test]
fn outbox_same_key_twice_enqueues_once() {
    let (_d, _p, mut s) = open();
    assert!(s.enqueue_if_new(&alert("r1", "k1")).unwrap());
    assert!(!s.enqueue_if_new(&alert("r1", "k1")).unwrap());
    assert!(s.enqueue_if_new(&alert("r1", "k2")).unwrap());
}

#[test]
fn drain_returns_pending_in_order_then_marks_delivered() {
    let (_d, _p, mut s) = open();
    let (a, b) = (alert("r1", "k1"), alert("r1", "k2"));
    s.enqueue_if_new(&a).unwrap();
    s.enqueue_if_new(&b).unwrap();
    assert_eq!(s.drain().unwrap(), vec![a.clone(), b]);
    assert!(s.drain().unwrap().is_empty());
    assert!(!s.enqueue_if_new(&a).unwrap(), "delivered keys stay deduped");
}

#[test]
fn budget_counts_per_utc_day_on_separate_handle() {
    let (_d, path, store) = open();
    let mut b = SqliteBudget::open(&path).unwrap();
    b.record_call(datetime!(2026-11-11 23:59:59 UTC)).unwrap();
    b.record_call(datetime!(2026-11-11 00:00:00 UTC)).unwrap();
    b.record_call(datetime!(2026-11-12 00:00:00 UTC)).unwrap();
    assert_eq!(b.used_on(date!(2026-11-11)).unwrap(), 2);
    assert_eq!(b.used_on(date!(2026-11-12)).unwrap(), 1);
    assert_eq!(b.used_on(date!(2026-11-13)).unwrap(), 0);
    drop(store);
    let b2 = SqliteBudget::open(&path).unwrap();
    assert_eq!(b2.used_on(date!(2026-11-11)).unwrap(), 2, "persisted");
}

#[test]
fn purge_deletes_old_trades_and_gaps() {
    let (_d, _p, mut s) = open();
    let t = datetime!(2026-11-11 00:00 UTC);
    let m = |n| t + time::Duration::minutes(n);
    s.insert_trades(&[trade("old", "@검", "검", m(0)), trade("new", "@검", "검", m(10))])
        .unwrap();
    s.record_gap(m(0), m(2)).unwrap();
    s.record_gap(m(8), m(12)).unwrap();
    assert_eq!(s.purge_older_than(m(5)).unwrap(), 1);
    let left = s.trades_between(m(0), m(60), None).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].trade_id, "new");
    assert_eq!(s.gaps_between(m(0), m(60)).unwrap(), vec![(m(8), m(12))]);
}

#[test]
fn second_collect_lock_fails_and_releases_on_drop() {
    let (_d, path, _s) = open();
    let first = CollectLock::acquire(&path).unwrap();
    let err = CollectLock::acquire(&path).err().expect("second holder fails");
    assert_eq!(err.exit_code(), 7);
    drop(first);
    CollectLock::acquire(&path).expect("free after drop");
}

#[test]
fn data_dir_follows_xdg_then_home() {
    // Single test touching env to avoid races within this test binary.
    // set_var is unsafe in edition 2024, so this only reads the environment.
    let d = data_dir();
    assert!(d.ends_with("mabi-ah"));
    if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        if !x.is_empty() {
            assert_eq!(d, std::path::Path::new(&x).join("mabi-ah"));
        }
    } else if let Ok(h) = std::env::var("HOME") {
        assert_eq!(d, std::path::Path::new(&h).join(".local/share/mabi-ah"));
    }
}
