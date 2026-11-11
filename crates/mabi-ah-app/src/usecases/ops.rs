//! Store-backed analytics, watches, NPC shop, horn, doctor.
use mabi_ah_domain::option::OptionRegistry;
use mabi_ah_domain::rank::{RankBy, RankEntry};
use mabi_ah_domain::staleness::Staleness;
use mabi_ah_domain::trend::{Bucket, TrendPoint};
use mabi_ah_domain::watch::{Alert, WatchRule};
use time::OffsetDateTime;

use crate::error::AppError;
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
    _store: &dyn TradeStore,
    _bucket: Bucket,
    _from: OffsetDateTime,
    _to: OffsetDateTime,
    _base_name: Option<&str>,
) -> Result<Vec<TrendPoint>, AppError> {
    todo!("node A green: compute_trend")
}

pub fn compute_rank(
    _store: &dyn TradeStore,
    _from: OffsetDateTime,
    _to: OffsetDateTime,
    _by: RankBy,
    _top: usize,
    _category: Option<&str>,
) -> Result<Vec<RankEntry>, AppError> {
    todo!("node A green: compute_rank")
}

pub fn add_watch(_store: &mut dyn WatchStore, _rule: &WatchRule) -> Result<(), AppError> {
    todo!("node A green: add_watch")
}

pub fn list_watches(_store: &dyn WatchStore) -> Result<Vec<WatchRule>, AppError> {
    todo!("node A green: list_watches")
}

pub fn remove_watch(_store: &mut dyn WatchStore, _id: &str) -> Result<(), AppError> {
    todo!("node A green: remove_watch")
}

/// Returns only alerts that were new to the outbox.
pub fn check_watch(
    _gw: &mut dyn AuctionGateway,
    _store: &dyn WatchStore,
    _outbox: &mut dyn Outbox,
    _opts: &OptionRegistry,
) -> Result<Vec<Alert>, AppError> {
    todo!("node A green: check_watch")
}

pub fn query_npc_shop(
    _gw: &mut dyn NpcShopGateway,
    _npc: &str,
    _server: &str,
    _channel: u32,
) -> Result<NpcShop, AppError> {
    todo!("node A green: query_npc_shop")
}

/// With `redact_names`, every character name becomes `***`.
pub fn query_horn(
    _gw: &mut dyn HornGateway,
    _server: &str,
    _redact_names: bool,
) -> Result<Vec<HornMessage>, AppError> {
    todo!("node A green: query_horn")
}

pub fn doctor(
    _clock: &dyn Clock,
    _store: &dyn TradeStore,
    _ledger: &dyn BudgetLedger,
    _limit: u64,
    _key_present: bool,
) -> Result<DoctorReport, AppError> {
    todo!("node A green: doctor")
}
