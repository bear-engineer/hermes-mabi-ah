//! Use case tests over in-test fakes.
#![allow(clippy::unwrap_used)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use time::macros::datetime;
use time::{Date, Duration, OffsetDateTime};

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{
    AuctionGateway, BudgetLedger, CallMeter, Clock, HornGateway, HornMessage, ListingQuery,
    NpcShop, NpcShopGateway, Outbox, Page, TradeStore, WatchStore,
};
use mabi_ah_app::usecases::*;
use mabi_ah_domain::category::CategoryRegistry;
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::{ItemOption, OptionFilter, OptionRegistry};
use mabi_ah_domain::rank::RankBy;
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::trend::Bucket;
use mabi_ah_domain::watch::{Alert, WatchRule};

const T0: OffsetDateTime = datetime!(2026-01-01 12:00 UTC);

fn listing(name: &str, category: &str, price: u64) -> Listing {
    Listing {
        name: ItemName::new(name, name),
        count: 1,
        category: category.to_owned(),
        unit_price: price,
        expires_at: T0 + Duration::hours(1),
        options: vec![],
    }
}

fn trade(id: &str, mins_ago: i64) -> Trade {
    Trade {
        trade_id: id.to_owned(),
        name: ItemName::new("테스트 검", "테스트 검"),
        count: 1,
        category: "검".to_owned(),
        unit_price: 100,
        traded_at: T0 - Duration::minutes(mins_ago),
        options: vec![],
    }
}

fn opt(t: &str, v: &str) -> ItemOption {
    ItemOption {
        option_type: t.to_owned(),
        sub_type: None,
        value: Some(v.to_owned()),
        value2: None,
        desc: None,
    }
}

// --- fakes -----------------------------------------------------------------

#[derive(Default)]
struct FakeGw {
    listing_pages: Vec<Vec<Listing>>,
    trade_pages: Vec<Vec<Trade>>,
    search_items: Vec<Listing>,
    listings_err: Option<AppError>,
    search_calls: Vec<Vec<String>>,
    calls: u32,
}
fn paginate<T: Clone>(pages: &[Vec<T>], cursor: Option<&str>) -> Page<T> {
    let i: usize = cursor.map_or(0, |c| c.parse().unwrap());
    Page {
        items: pages.get(i).cloned().unwrap_or_default(),
        next_cursor: (i + 1 < pages.len()).then(|| (i + 1).to_string()),
    }
}
impl AuctionGateway for FakeGw {
    fn listings(&mut self, q: &ListingQuery) -> Result<Page<Listing>, AppError> {
        self.calls += 1;
        if let Some(e) = &self.listings_err {
            return Err(e.clone());
        }
        Ok(paginate(&self.listing_pages, q.cursor.as_deref()))
    }
    fn trades(&mut self, q: &ListingQuery) -> Result<Page<Trade>, AppError> {
        self.calls += 1;
        Ok(paginate(&self.trade_pages, q.cursor.as_deref()))
    }
    fn search(&mut self, kw: &[String], _c: Option<&str>) -> Result<Page<Listing>, AppError> {
        self.calls += 1;
        self.search_calls.push(kw.to_vec());
        Ok(Page {
            items: self.search_items.clone(),
            next_cursor: None,
        })
    }
}

#[derive(Default)]
struct MemStore {
    trades: Vec<Trade>,
    gaps: Vec<(OffsetDateTime, OffsetDateTime)>,
}
impl TradeStore for MemStore {
    fn insert_trades(&mut self, ts: &[Trade]) -> Result<usize, AppError> {
        let mut n = 0;
        for t in ts {
            if !self.trades.iter().any(|s| s.trade_id == t.trade_id) {
                self.trades.push(t.clone());
                n += 1;
            }
        }
        Ok(n)
    }
    fn newest_trade(&self, category: Option<&str>) -> Result<Option<Trade>, AppError> {
        Ok(self
            .trades
            .iter()
            .filter(|t| category.is_none_or(|c| t.category == c))
            .max_by_key(|t| t.traded_at)
            .cloned())
    }
    fn trades_between(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
        base: Option<&str>,
    ) -> Result<Vec<Trade>, AppError> {
        let mut v: Vec<Trade> = self
            .trades
            .iter()
            .filter(|t| t.traded_at >= from && t.traded_at < to)
            .filter(|t| base.is_none_or(|b| t.name.base == b))
            .cloned()
            .collect();
        v.sort_by_key(|t| t.traded_at);
        Ok(v)
    }
    fn record_gap(&mut self, from: OffsetDateTime, to: OffsetDateTime) -> Result<(), AppError> {
        self.gaps.push((from, to));
        Ok(())
    }
    fn gaps_between(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Vec<(OffsetDateTime, OffsetDateTime)>, AppError> {
        Ok(self
            .gaps
            .iter()
            .copied()
            .filter(|&(s, e)| s < to && from < e)
            .collect())
    }
    fn purge_older_than(&mut self, _cutoff: OffsetDateTime) -> Result<usize, AppError> {
        Ok(0)
    }
}

#[derive(Default)]
struct MemWatch(Vec<WatchRule>);
impl WatchStore for MemWatch {
    fn add(&mut self, rule: &WatchRule) -> Result<(), AppError> {
        self.0.retain(|r| r.id != rule.id);
        self.0.push(rule.clone());
        Ok(())
    }
    fn list(&self) -> Result<Vec<WatchRule>, AppError> {
        Ok(self.0.clone())
    }
    fn remove(&mut self, id: &str) -> Result<bool, AppError> {
        let n = self.0.len();
        self.0.retain(|r| r.id != id);
        Ok(self.0.len() < n)
    }
}

#[derive(Default)]
struct MemOutbox {
    seen: Vec<String>,
    pending: Vec<Alert>,
}
impl Outbox for MemOutbox {
    fn enqueue_if_new(&mut self, a: &Alert) -> Result<bool, AppError> {
        if self.seen.contains(&a.dedupe_key) {
            return Ok(false);
        }
        self.seen.push(a.dedupe_key.clone());
        self.pending.push(a.clone());
        Ok(true)
    }
    fn drain(&mut self) -> Result<Vec<Alert>, AppError> {
        Ok(std::mem::take(&mut self.pending))
    }
}

#[derive(Default)]
struct MemLedger(BTreeMap<Date, u64>);
impl BudgetLedger for MemLedger {
    fn record_call(&mut self, at: OffsetDateTime) -> Result<(), AppError> {
        *self.0.entry(at.date()).or_default() += 1;
        Ok(())
    }
    fn used_on(&self, date: Date) -> Result<u64, AppError> {
        Ok(self.0.get(&date).copied().unwrap_or(0))
    }
}

struct FixedClock(OffsetDateTime);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

struct FakeShop;
impl NpcShopGateway for FakeShop {
    fn npc_shop(&mut self, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError> {
        Ok(NpcShop {
            npc: npc.to_owned(),
            server: server.to_owned(),
            channel,
            fetched_at: T0,
            next_update_at: None,
            tabs: vec![],
        })
    }
}

struct FakeHorn;
impl HornGateway for FakeHorn {
    fn horn(&mut self, _server: &str) -> Result<Vec<HornMessage>, AppError> {
        Ok(vec![HornMessage {
            character_name: "테스트캐릭터".to_owned(),
            message: "안녕".to_owned(),
            sent_at: T0,
        }])
    }
}

fn sorted_paged_gw() -> FakeGw {
    FakeGw {
        listing_pages: vec![
            vec![
                listing("테스트 검", "검", 300),
                listing("테스트 검", "신기한 무기", 100),
            ],
            vec![listing("테스트 검", "검", 200)],
            vec![listing("테스트 검", "검", 400)],
        ],
        ..FakeGw::default()
    }
}

// --- query -----------------------------------------------------------------

#[test]
fn query_first_page_only_is_truncated_with_stats() {
    let mut gw = sorted_paged_gw();
    let mut cats = CategoryRegistry::new();
    let r = query_listings(
        &mut gw,
        &ListingQuery::default(),
        &[],
        &OptionRegistry::default(),
        &mut cats,
        false,
        5,
    )
    .unwrap();
    assert_eq!((r.pages, r.truncated, r.listings.len()), (1, true, 2));
    let s = r.stats.unwrap();
    assert_eq!((s.min, s.max), (100, 300));
    assert!(cats.is_known("신기한 무기"), "discovered category recorded");
}

#[test]
fn query_all_pages_respects_max_pages() {
    let mut gw = sorted_paged_gw();
    let mut cats = CategoryRegistry::new();
    let all = |gw: &mut FakeGw, max| {
        query_listings(
            gw,
            &ListingQuery::default(),
            &[],
            &OptionRegistry::default(),
            &mut CategoryRegistry::new(),
            true,
            max,
        )
        .unwrap()
    };
    let capped = all(&mut gw, 2);
    assert_eq!(
        (capped.pages, capped.truncated, capped.listings.len()),
        (2, true, 3)
    );
    let full = all(&mut gw, 10);
    assert_eq!(
        (full.pages, full.truncated, full.listings.len()),
        (3, false, 4)
    );
    let _ = &mut cats;
}

#[test]
fn query_applies_option_filter_before_stats() {
    let mut l1 = listing("테스트 검", "검", 100);
    l1.options = vec![opt("세공 랭크", "3")];
    let l2 = listing("테스트 검", "검", 900);
    let mut gw = FakeGw {
        listing_pages: vec![vec![l1, l2]],
        ..FakeGw::default()
    };
    let f: OptionFilter = "세공 랭크>=3".parse().unwrap();
    let r = query_listings(
        &mut gw,
        &ListingQuery::default(),
        &[f],
        &OptionRegistry::default(),
        &mut CategoryRegistry::new(),
        false,
        1,
    )
    .unwrap();
    assert_eq!(r.listings.len(), 1);
    assert_eq!(r.stats.unwrap().max, 100);
}

#[test]
fn query_trades_same_shape() {
    let mut gw = FakeGw {
        trade_pages: vec![vec![trade("a", 1)], vec![trade("b", 2)]],
        ..FakeGw::default()
    };
    let r = query_trades(
        &mut gw,
        &ListingQuery::default(),
        &[],
        &OptionRegistry::default(),
        &mut CategoryRegistry::new(),
        true,
        9,
    )
    .unwrap();
    assert_eq!((r.pages, r.truncated, r.trades.len()), (2, false, 2));
    assert_eq!(r.stats.unwrap().units, 2);
}

#[test]
fn search_rejects_more_than_ten_keywords_and_filters_category() {
    let mut gw = FakeGw {
        search_items: vec![listing("a", "검", 1), listing("b", "활", 2)],
        ..FakeGw::default()
    };
    let eleven: Vec<String> = (0..11).map(|i| i.to_string()).collect();
    let run = |gw: &mut FakeGw, kw: &[String], cat| {
        search_listings(
            gw,
            kw,
            cat,
            &[],
            &OptionRegistry::default(),
            &mut CategoryRegistry::new(),
        )
    };
    let err = run(&mut gw, &eleven, None).unwrap_err();
    assert!(matches!(err, AppError::Usage(_)));
    assert_eq!(gw.calls, 0);
    let ok = run(&mut gw, &eleven[..10], Some("활")).unwrap();
    assert_eq!(ok.listings.len(), 1);
    assert_eq!(ok.listings[0].category, "활");
    assert!(matches!(run(&mut gw, &[], None), Err(AppError::Usage(_))));
}

#[test]
fn not_found_listing_gets_name_suggestions_in_hint() {
    let mut gw = FakeGw {
        listings_err: Some(AppError::NotFound {
            what: "item".to_owned(),
            hint: None,
        }),
        search_items: vec![
            listing("@테스트 검", "검", 1),
            listing("테스트 검", "검", 2),
            listing("테스트 활", "활", 3),
        ],
        ..FakeGw::default()
    };
    let q = ListingQuery {
        name: Some("테스트 검X".to_owned()),
        ..ListingQuery::default()
    };
    let err = query_listings(
        &mut gw,
        &q,
        &[],
        &OptionRegistry::default(),
        &mut CategoryRegistry::new(),
        false,
        1,
    )
    .unwrap_err();
    let AppError::NotFound { hint, .. } = err else {
        panic!("expected NotFound");
    };
    let hint = hint.unwrap();
    assert!(
        hint.contains("테스트 검") && hint.contains("테스트 활"),
        "{hint}"
    );
    assert_eq!(
        hint.matches("테스트 검").count(),
        1,
        "distinct by base name"
    );
    assert_eq!(
        gw.search_calls[0],
        vec!["테스트".to_owned(), "검X".to_owned()]
    );
}

#[test]
fn suggest_names_is_distinct_and_capped_at_ten() {
    let mut gw = FakeGw {
        search_items: (0..15)
            .map(|i| listing(&format!("n{i}"), "검", 1))
            .collect(),
        ..FakeGw::default()
    };
    let names = suggest_names(&mut gw, "n").unwrap();
    assert_eq!(names.len(), 10);
}

// --- collect ---------------------------------------------------------------

#[test]
fn collect_stops_at_watermark_overlap() {
    let mut store = MemStore::default();
    store
        .insert_trades(&[trade("old1", 30), trade("old2", 40)])
        .unwrap();
    let mut gw = FakeGw {
        trade_pages: vec![
            vec![trade("n1", 1), trade("n2", 2), trade("old1", 30)],
            vec![trade("never", 50)],
        ],
        ..FakeGw::default()
    };
    let r = collect_trades(&mut gw, &mut store, &FixedClock(T0), 10).unwrap();
    assert_eq!((r.pages, r.fetched, r.inserted), (1, 3, 2));
    assert!(r.reached_watermark);
    assert_eq!(r.gap, None);
    assert!(store.gaps.is_empty());
}

#[test]
fn collect_records_gap_without_overlap() {
    let mut store = MemStore::default();
    store.insert_trades(&[trade("old", 300)]).unwrap();
    let mut gw = FakeGw {
        trade_pages: vec![vec![trade("n1", 1), trade("n2", 60)], vec![trade("n3", 90)]],
        ..FakeGw::default()
    };
    let r = collect_trades(&mut gw, &mut store, &FixedClock(T0), 10).unwrap();
    assert!(!r.reached_watermark);
    assert_eq!((r.pages, r.inserted), (2, 3));
    let expect = (T0 - Duration::minutes(300), T0 - Duration::minutes(90));
    assert_eq!(r.gap, Some(expect));
    assert_eq!(store.gaps, vec![expect]);
}

#[test]
fn collect_first_run_has_no_gap_and_respects_max_calls() {
    let mut store = MemStore::default();
    let mut gw = FakeGw {
        trade_pages: vec![
            vec![trade("a", 1)],
            vec![trade("b", 2)],
            vec![trade("c", 3)],
        ],
        ..FakeGw::default()
    };
    let r = collect_trades(&mut gw, &mut store, &FixedClock(T0), 2).unwrap();
    assert_eq!((r.pages, r.inserted, gw.calls), (2, 2, 2));
    assert_eq!(r.gap, None);
}

// --- store-backed ----------------------------------------------------------

#[test]
fn trend_and_rank_read_from_store() {
    let mut store = MemStore::default();
    store
        .insert_trades(&[trade("a", 10), trade("b", 20), trade("c", 90)])
        .unwrap();
    store
        .record_gap(T0 - Duration::minutes(80), T0 - Duration::minutes(70))
        .unwrap();
    let from = datetime!(2026-01-01 10:00 UTC);
    let pts = compute_trend(
        &store,
        Bucket::Hour,
        from,
        T0 + Duration::hours(1),
        Some("테스트 검"),
    )
    .unwrap();
    assert_eq!(pts.len(), 3);
    assert_eq!(pts.iter().map(|p| p.trades).sum::<u64>(), 3);
    assert!(pts.iter().any(|p| p.has_gap));
    let ranks = compute_rank(
        &store,
        from,
        T0 + Duration::hours(1),
        RankBy::Volume,
        5,
        None,
    )
    .unwrap();
    assert_eq!((ranks.len(), ranks[0].units), (1, 3));
}

// --- watch -----------------------------------------------------------------

fn rule(id: &str, max: Option<u64>) -> WatchRule {
    WatchRule {
        id: id.to_owned(),
        base_name: "테스트 검".to_owned(),
        max_unit_price: max,
        category: None,
        filters: vec![],
    }
}

#[test]
fn watch_add_list_remove() {
    let mut w = MemWatch::default();
    add_watch(&mut w, &rule("r1", None)).unwrap();
    assert_eq!(list_watches(&w).unwrap().len(), 1);
    let mut bad = rule("r2", None);
    bad.filters = vec!["no operator".to_owned()];
    assert!(matches!(add_watch(&mut w, &bad), Err(AppError::Usage(_))));
    remove_watch(&mut w, "r1").unwrap();
    assert!(matches!(
        remove_watch(&mut w, "r1"),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn check_watch_dedupes_across_runs() {
    let mut w = MemWatch::default();
    add_watch(&mut w, &rule("r1", Some(250))).unwrap();
    let mut gw = FakeGw {
        listing_pages: vec![vec![
            listing("테스트 검", "검", 200),
            listing("테스트 검", "검", 900),
        ]],
        ..FakeGw::default()
    };
    let mut ob = MemOutbox::default();
    let reg = OptionRegistry::default();
    let first = check_watch(&mut gw, &w, &mut ob, &reg).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].listing.unit_price, 200);
    let second = check_watch(&mut gw, &w, &mut ob, &reg).unwrap();
    assert!(second.is_empty(), "same listing must not alert twice");
    assert_eq!(ob.drain().unwrap().len(), 1);
}

// --- shop / horn / doctor / meter -------------------------------------------

#[test]
fn horn_redacts_character_names() {
    let masked = query_horn(&mut FakeHorn, "류트", true).unwrap();
    assert_eq!(masked[0].character_name, "***");
    assert_eq!(masked[0].message, "안녕");
    let raw = query_horn(&mut FakeHorn, "류트", false).unwrap();
    assert_eq!(raw[0].character_name, "테스트캐릭터");
}

#[test]
fn npc_shop_passes_through() {
    let s = query_npc_shop(&mut FakeShop, "테스트NPC", "류트", 3).unwrap();
    assert_eq!((s.npc.as_str(), s.channel), ("테스트NPC", 3));
}

#[test]
fn doctor_reports_staleness_budget_and_key() {
    let mut store = MemStore::default();
    store.insert_trades(&[trade("a", 60)]).unwrap();
    let mut ledger = MemLedger::default();
    ledger.record_call(T0).unwrap();
    let r = doctor(&FixedClock(T0), &store, &ledger, 1000, true).unwrap();
    assert!(r.staleness.stale);
    assert_eq!(r.staleness.lag_seconds, Some(3600));
    assert_eq!((r.used_today, r.limit, r.key_present), (1, 1000, true));
    let empty = doctor(&FixedClock(T0), &MemStore::default(), &ledger, 5, false).unwrap();
    assert!(empty.staleness.stale && !empty.key_present);
}

#[test]
fn ledger_meter_counts_then_refuses_at_cap() {
    let meter = LedgerMeter::new(MemLedger::default(), FixedClock(T0), 2);
    meter.before_request().unwrap();
    meter.before_request().unwrap();
    let err = meter.before_request().unwrap_err();
    assert_eq!(err, AppError::BudgetExceeded { used: 2, limit: 2 });
}

#[test]
fn ledger_meter_is_usable_as_shared_dyn_meter() {
    let meter: Rc<dyn CallMeter> =
        Rc::new(LedgerMeter::new(MemLedger::default(), FixedClock(T0), 1));
    let sent = Cell::new(0);
    let send = || meter.before_request().map(|()| sent.set(sent.get() + 1));
    assert!(send().is_ok());
    assert!(send().is_err());
    assert_eq!(sent.get(), 1);
    let _ = RefCell::new(0);
}
