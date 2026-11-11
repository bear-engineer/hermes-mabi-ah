//! Contract tests for the application boundary: every port is implementable by
//! a plain in-memory fake, and `AppError` maps to the documented exit codes.
#![allow(clippy::unwrap_used)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use time::macros::datetime;
use time::{Date, OffsetDateTime};

use mabi_ah_app::error::{AppError, UpstreamKind};
use mabi_ah_app::ports::{
    AuctionGateway, BudgetLedger, CallMeter, Clock, HornGateway, HornMessage, ListingQuery,
    NpcShop, NpcShopGateway, Outbox, Page, TradeStore, WatchStore,
};
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::watch::{Alert, WatchRule};

const T0: OffsetDateTime = datetime!(2026-01-01 00:00 UTC);

fn listing(price: u64) -> Listing {
    Listing {
        name: ItemName::new("테스트 검", "테스트 검"),
        count: 1,
        category: "검".to_owned(),
        unit_price: price,
        expires_at: T0 + time::Duration::hours(1),
        options: vec![],
    }
}

fn trade(id: &str, at: OffsetDateTime) -> Trade {
    Trade {
        trade_id: id.to_owned(),
        name: ItemName::new("테스트 검", "테스트 검"),
        count: 1,
        category: "검".to_owned(),
        unit_price: 100,
        traded_at: at,
        options: vec![],
    }
}

fn all_errors() -> Vec<AppError> {
    let upstream = |kind| AppError::Upstream {
        kind,
        nexon_code: None,
    };
    vec![
        AppError::NotFound {
            what: "item".to_owned(),
            hint: Some("suggest".to_owned()),
        },
        AppError::Usage("bad flag".to_owned()),
        AppError::Auth("no key".to_owned()),
        AppError::RateLimited {
            retry_after: Some(Duration::from_secs(1)),
        },
        AppError::BudgetExceeded {
            used: 1000,
            limit: 1000,
        },
        upstream(UpstreamKind::GameMaintenance),
        upstream(UpstreamKind::ApiMaintenance),
        upstream(UpstreamKind::DataPreparing),
        upstream(UpstreamKind::Internal),
        AppError::Store("locked".to_owned()),
        AppError::Internal("bug".to_owned()),
    ]
}

#[test]
fn exit_codes_match_brief_table() {
    let codes: Vec<u8> = all_errors().iter().map(AppError::exit_code).collect();
    // 1 internal, 2 usage, 3 not found, 4 key/permission, 5 rate/budget,
    // 6 nexon maintenance/outage, 7 store.
    assert_eq!(codes, vec![3, 2, 4, 5, 5, 6, 6, 6, 6, 7, 1]);
}

#[test]
fn machine_codes_are_stable_and_unique() {
    let codes: Vec<&str> = all_errors().iter().map(AppError::code).collect();
    assert_eq!(
        codes,
        vec![
            "not_found",
            "usage",
            "auth",
            "rate_limited",
            "budget_exceeded",
            "upstream_game_maintenance",
            "upstream_api_maintenance",
            "upstream_data_preparing",
            "upstream_internal",
            "store",
            "internal",
        ]
    );
}

#[test]
fn only_rate_limit_and_upstream_are_retryable() {
    let flags: Vec<bool> = all_errors().iter().map(AppError::retryable).collect();
    assert_eq!(
        flags,
        vec![
            false, false, false, true, false, true, true, true, true, false, false
        ]
    );
}

// --- fakes -----------------------------------------------------------------

struct FixedClock(OffsetDateTime);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
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

/// The meter described in the ports module docs: check the limit, then record.
struct LedgerMeter<L, C> {
    ledger: RefCell<L>,
    clock: C,
    limit: u64,
}
impl<L: BudgetLedger, C: Clock> CallMeter for LedgerMeter<L, C> {
    fn before_request(&self) -> Result<(), AppError> {
        let now = self.clock.now();
        let used = self.ledger.borrow().used_on(now.date())?;
        if used >= self.limit {
            return Err(AppError::BudgetExceeded {
                used,
                limit: self.limit,
            });
        }
        self.ledger.borrow_mut().record_call(now)
    }
}

/// Fake API: two pages of listings, metered once per page request.
struct FakeNexon {
    meter: Rc<dyn CallMeter>,
    requests_sent: Cell<u32>,
}
impl FakeNexon {
    fn request(&self) -> Result<(), AppError> {
        self.meter.before_request()?;
        self.requests_sent.set(self.requests_sent.get() + 1);
        Ok(())
    }
}
impl AuctionGateway for FakeNexon {
    fn listings(&mut self, query: &ListingQuery) -> Result<Page<Listing>, AppError> {
        self.request()?;
        Ok(match query.cursor.as_deref() {
            None => Page {
                items: vec![listing(100)],
                next_cursor: Some("c1".to_owned()),
            },
            Some(_) => Page {
                items: vec![listing(200)],
                next_cursor: None,
            },
        })
    }
    fn trades(&mut self, _query: &ListingQuery) -> Result<Page<Trade>, AppError> {
        self.request()?;
        Ok(Page {
            items: vec![trade("t1", T0)],
            next_cursor: None,
        })
    }
    fn search(
        &mut self,
        keywords: &[String],
        _cursor: Option<&str>,
    ) -> Result<Page<Listing>, AppError> {
        self.request()?;
        if keywords.is_empty() {
            return Err(AppError::Usage("keyword required".to_owned()));
        }
        Ok(Page {
            items: vec![listing(100)],
            next_cursor: None,
        })
    }
}
impl NpcShopGateway for FakeNexon {
    fn npc_shop(&mut self, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError> {
        self.request()?;
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
impl HornGateway for FakeNexon {
    fn horn(&mut self, _server: &str) -> Result<Vec<HornMessage>, AppError> {
        self.request()?;
        Ok(vec![HornMessage {
            character_name: "테스트캐릭터".to_owned(),
            message: "테스트".to_owned(),
            sent_at: T0,
        }])
    }
}

type Gap = (OffsetDateTime, OffsetDateTime);

#[derive(Default)]
struct MemStore {
    trades: Vec<Trade>,
    gaps: Vec<Gap>,
    rules: Vec<WatchRule>,
    seen_keys: Vec<String>,
    pending: Vec<Alert>,
}
impl TradeStore for MemStore {
    fn insert_trades(&mut self, trades: &[Trade]) -> Result<usize, AppError> {
        let before = self.trades.len();
        for t in trades {
            if !self.trades.iter().any(|s| s.trade_id == t.trade_id) {
                self.trades.push(t.clone());
            }
        }
        Ok(self.trades.len() - before)
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
        base_name: Option<&str>,
    ) -> Result<Vec<Trade>, AppError> {
        let mut out: Vec<Trade> = self
            .trades
            .iter()
            .filter(|t| from <= t.traded_at && t.traded_at < to)
            .filter(|t| base_name.is_none_or(|b| t.name.base == b))
            .cloned()
            .collect();
        out.sort_by_key(|t| t.traded_at);
        Ok(out)
    }
    fn record_gap(&mut self, from: OffsetDateTime, to: OffsetDateTime) -> Result<(), AppError> {
        self.gaps.push((from, to));
        Ok(())
    }
    fn gaps_between(&self, from: OffsetDateTime, to: OffsetDateTime) -> Result<Vec<Gap>, AppError> {
        Ok(self
            .gaps
            .iter()
            .filter(|(s, e)| *s < to && from < *e)
            .copied()
            .collect())
    }
    fn purge_older_than(&mut self, cutoff: OffsetDateTime) -> Result<usize, AppError> {
        let before = self.trades.len();
        self.trades.retain(|t| t.traded_at >= cutoff);
        Ok(before - self.trades.len())
    }
}
impl WatchStore for MemStore {
    fn add(&mut self, rule: &WatchRule) -> Result<(), AppError> {
        self.rules.retain(|r| r.id != rule.id);
        self.rules.push(rule.clone());
        Ok(())
    }
    fn list(&self) -> Result<Vec<WatchRule>, AppError> {
        Ok(self.rules.clone())
    }
    fn remove(&mut self, id: &str) -> Result<bool, AppError> {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        Ok(self.rules.len() != before)
    }
}
impl Outbox for MemStore {
    fn enqueue_if_new(&mut self, alert: &Alert) -> Result<bool, AppError> {
        if self.seen_keys.contains(&alert.dedupe_key) {
            return Ok(false);
        }
        self.seen_keys.push(alert.dedupe_key.clone());
        self.pending.push(alert.clone());
        Ok(true)
    }
    fn drain(&mut self) -> Result<Vec<Alert>, AppError> {
        Ok(std::mem::take(&mut self.pending))
    }
}

fn nexon_with_limit(limit: u64) -> (FakeNexon, Rc<LedgerMeter<MemLedger, FixedClock>>) {
    let meter = Rc::new(LedgerMeter {
        ledger: RefCell::new(MemLedger::default()),
        clock: FixedClock(T0),
        limit,
    });
    let nexon = FakeNexon {
        meter: meter.clone(),
        requests_sent: Cell::new(0),
    };
    (nexon, meter)
}

// --- contracts -------------------------------------------------------------

#[test]
fn gateways_work_as_trait_objects_and_follow_the_cursor() {
    let (mut nexon, _meter) = nexon_with_limit(10);
    let gw: &mut dyn AuctionGateway = &mut nexon;
    let first = gw.listings(&ListingQuery::default()).unwrap();
    let second = gw
        .listings(&ListingQuery {
            cursor: first.next_cursor.clone(),
            ..ListingQuery::default()
        })
        .unwrap();
    assert_eq!(first.items.len() + second.items.len(), 2);
    assert_eq!(second.next_cursor, None);
    assert_eq!(gw.trades(&ListingQuery::default()).unwrap().items.len(), 1);
    assert_eq!(gw.search(&[], None).unwrap_err().code(), "usage");

    let shop: &mut dyn NpcShopGateway = &mut nexon;
    assert_eq!(shop.npc_shop("상인", "류트", 3).unwrap().channel, 3);
    let horn: &mut dyn HornGateway = &mut nexon;
    assert_eq!(horn.horn("류트").unwrap().len(), 1);
}

#[test]
fn every_request_is_metered_and_refused_past_the_limit() {
    let (mut nexon, meter) = nexon_with_limit(2);
    nexon.listings(&ListingQuery::default()).unwrap();
    nexon.horn("류트").unwrap();
    let refused = nexon.trades(&ListingQuery::default()).unwrap_err();

    assert_eq!(refused, AppError::BudgetExceeded { used: 2, limit: 2 });
    assert_eq!(refused.exit_code(), 5);
    // The refused request was never sent and never counted.
    assert_eq!(nexon.requests_sent.get(), 2);
    assert_eq!(meter.ledger.borrow().used_on(T0.date()).unwrap(), 2);
    let tomorrow = T0.date().next_day().unwrap();
    assert_eq!(meter.ledger.borrow().used_on(tomorrow).unwrap(), 0);
}

#[test]
fn trade_store_dedupes_ranges_and_purges() {
    let mut store = MemStore::default();
    let hour = time::Duration::hours(1);
    let batch = [trade("a", T0), trade("b", T0 + hour), trade("a", T0)];
    let ts: &mut dyn TradeStore = &mut store;

    assert_eq!(ts.insert_trades(&batch).unwrap(), 2);
    assert_eq!(ts.insert_trades(&batch).unwrap(), 0);
    assert_eq!(ts.newest_trade(None).unwrap().unwrap().trade_id, "b");
    assert_eq!(ts.newest_trade(Some("없는분류")).unwrap(), None);
    // Half-open range: `to` is excluded.
    let in_range = ts.trades_between(T0, T0 + hour, Some("테스트 검")).unwrap();
    assert_eq!(in_range.len(), 1);

    ts.record_gap(T0, T0 + hour).unwrap();
    assert_eq!(ts.gaps_between(T0, T0 + hour).unwrap().len(), 1);
    assert!(
        ts.gaps_between(T0 + hour, T0 + hour * 2)
            .unwrap()
            .is_empty()
    );

    assert_eq!(ts.purge_older_than(T0 + hour).unwrap(), 1);
    assert_eq!(ts.newest_trade(None).unwrap().unwrap().trade_id, "b");
}

#[test]
fn watch_store_and_outbox_fire_each_alert_once() {
    let mut store = MemStore::default();
    let rule = WatchRule {
        id: "r1".to_owned(),
        base_name: "테스트 검".to_owned(),
        max_unit_price: Some(150),
        category: None,
        filters: vec![],
    };
    let ws: &mut dyn WatchStore = &mut store;
    ws.add(&rule).unwrap();
    ws.add(&rule).unwrap();
    assert_eq!(ws.list().unwrap(), vec![rule.clone()]);
    assert!(ws.remove("r1").unwrap());
    assert!(!ws.remove("r1").unwrap());

    let alert = Alert {
        rule_id: rule.id,
        dedupe_key: "r1|k".to_owned(),
        listing: listing(100),
    };
    let outbox: &mut dyn Outbox = &mut store;
    assert!(outbox.enqueue_if_new(&alert).unwrap());
    assert!(!outbox.enqueue_if_new(&alert).unwrap());
    assert_eq!(outbox.drain().unwrap(), vec![alert.clone()]);
    assert!(outbox.drain().unwrap().is_empty());
    // Delivered alerts stay deduped.
    assert!(!outbox.enqueue_if_new(&alert).unwrap());
}

#[test]
fn exit_code_table_values() {
    use mabi_ah_app::error::exit_code as x;
    let got = [
        x::OK,
        x::INTERNAL,
        x::USAGE,
        x::NOT_FOUND,
        x::AUTH,
        x::RATE_OR_BUDGET,
        x::UPSTREAM,
        x::STORE,
    ];
    assert_eq!(got, [0, 1, 2, 3, 4, 5, 6, 7]);
}
