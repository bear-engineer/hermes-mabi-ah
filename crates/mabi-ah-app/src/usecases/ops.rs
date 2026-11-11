//! Store-backed analytics, watches, NPC shop, horn, doctor.
use mabi_ah_domain::option::{OptionFilter, OptionRegistry};
use mabi_ah_domain::rank::rank;
use mabi_ah_domain::rank::{RankBy, RankEntry};
use mabi_ah_domain::staleness::Staleness;
use mabi_ah_domain::staleness::{DEFAULT_STALE_THRESHOLD, assess};
use mabi_ah_domain::trend::trend;
use mabi_ah_domain::trend::{Bucket, TrendPoint};
use mabi_ah_domain::watch::{Alert, WatchRule, evaluate};
use time::OffsetDateTime;

use crate::error::AppError;
use crate::ports::ListingQuery;
use crate::ports::{
    AuctionGateway, BudgetLedger, Clock, HornGateway, HornMessage, NpcShop, NpcShopGateway, Outbox,
    TradeStore, WatchStore,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub staleness: Staleness,
    pub used_today: u64,
    pub limit: u64,
    pub key_present: bool,
}

pub fn compute_trend(
    store: &dyn TradeStore,
    bucket: Bucket,
    from: OffsetDateTime,
    to: OffsetDateTime,
    base_name: Option<&str>,
) -> Result<Vec<TrendPoint>, AppError> {
    let trades = store.trades_between(from, to, base_name)?;
    let gaps = store.gaps_between(from, to)?;
    Ok(trend(&trades, bucket, from, to, &gaps))
}

pub fn compute_rank(
    store: &dyn TradeStore,
    from: OffsetDateTime,
    to: OffsetDateTime,
    by: RankBy,
    top: usize,
    category: Option<&str>,
) -> Result<Vec<RankEntry>, AppError> {
    let trades = store.trades_between(from, to, None)?;
    Ok(rank(&trades, by, top, category))
}

pub fn add_watch(store: &mut dyn WatchStore, rule: &WatchRule) -> Result<(), AppError> {
    if rule.id.trim().is_empty() || rule.base_name.trim().is_empty() {
        return Err(AppError::Usage(
            "watch rule needs an id and an item name".to_owned(),
        ));
    }
    for f in &rule.filters {
        f.parse::<OptionFilter>()
            .map_err(|e| AppError::Usage(e.to_string()))?;
    }
    store.add(rule)
}

pub fn list_watches(store: &dyn WatchStore) -> Result<Vec<WatchRule>, AppError> {
    store.list()
}

pub fn remove_watch(store: &mut dyn WatchStore, id: &str) -> Result<(), AppError> {
    if store.remove(id)? {
        Ok(())
    } else {
        Err(AppError::NotFound {
            what: format!("watch rule {id}"),
            hint: None,
        })
    }
}

/// Outcome of one `check_watch` run.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchCheck {
    /// Every undelivered alert (now marked delivered); empty if any rule failed.
    pub alerts: Vec<Alert>,
    /// One error per rule whose API call (or evaluation) failed.
    pub errors: Vec<AppError>,
}

/// Evaluates every rule (a failing rule never stops the others), then drains
/// the outbox so alerts queued by earlier runs are delivered too. While any
/// rule failed the outbox is left untouched: the caller reports a failure and
/// the pending alerts are delivered by the next healthy run.
pub fn check_watch(
    gw: &mut dyn AuctionGateway,
    store: &dyn WatchStore,
    outbox: &mut dyn Outbox,
    opts: &OptionRegistry,
) -> Result<WatchCheck, AppError> {
    let mut errors = Vec::new();
    for rule in store.list()? {
        let query = ListingQuery {
            name: Some(rule.base_name.clone()),
            category: rule.category.clone(),
            cursor: None,
        };
        let page = match gw.listings(&query) {
            Ok(p) => p,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        match evaluate(&rule, &page.items, opts) {
            Ok(alerts) => {
                for alert in alerts {
                    outbox.enqueue_if_new(&alert)?;
                }
            }
            Err(e) => errors.push(AppError::Usage(e.to_string())),
        }
    }
    let alerts = if errors.is_empty() {
        outbox.drain()?
    } else {
        Vec::new()
    };
    Ok(WatchCheck { alerts, errors })
}

pub fn query_npc_shop(
    gw: &mut dyn NpcShopGateway,
    npc: &str,
    server: &str,
    channel: u32,
) -> Result<NpcShop, AppError> {
    gw.npc_shop(npc, server, channel)
}

/// With `redact_names`, every character name becomes `***`.
pub fn query_horn(
    gw: &mut dyn HornGateway,
    server: &str,
    redact_names: bool,
) -> Result<Vec<HornMessage>, AppError> {
    let mut messages = gw.horn(server)?;
    if redact_names {
        for m in &mut messages {
            m.character_name = "***".to_owned();
        }
    }
    Ok(messages)
}

pub fn doctor(
    clock: &dyn Clock,
    store: &dyn TradeStore,
    ledger: &dyn BudgetLedger,
    limit: u64,
    key_present: bool,
) -> Result<DoctorReport, AppError> {
    let now = clock.now();
    let newest = store.newest_trade(None)?.map(|t| t.traded_at);
    Ok(DoctorReport {
        staleness: assess(newest, now, DEFAULT_STALE_THRESHOLD),
        used_today: ledger.used_on(now.date())?,
        limit,
        key_present,
    })
}
