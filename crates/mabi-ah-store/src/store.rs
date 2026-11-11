//! Red stub: every behaviour is `todo!()`.
use std::path::{Path, PathBuf};

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{BudgetLedger, Outbox, TradeStore, WatchStore};
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::watch::{Alert, WatchRule};
use time::{Date, OffsetDateTime};

pub fn data_dir() -> PathBuf {
    todo!("node S green")
}

pub struct SqliteStore;
pub struct SqliteBudget;
pub struct CollectLock;

impl SqliteStore {
    pub fn open(_path: &Path) -> Result<Self, AppError> {
        todo!("node S green")
    }
    pub fn discovered_categories(&self) -> Result<Vec<String>, AppError> {
        todo!("node S green")
    }
}

impl SqliteBudget {
    pub fn open(_path: &Path) -> Result<Self, AppError> {
        todo!("node S green")
    }
}

impl CollectLock {
    pub fn acquire(_db_path: &Path) -> Result<Self, AppError> {
        todo!("node S green")
    }
}

impl TradeStore for SqliteStore {
    fn insert_trades(&mut self, _t: &[Trade]) -> Result<usize, AppError> {
        todo!("node S green")
    }
    fn newest_trade(&self, _c: Option<&str>) -> Result<Option<Trade>, AppError> {
        todo!("node S green")
    }
    fn trades_between(
        &self,
        _from: OffsetDateTime,
        _to: OffsetDateTime,
        _base: Option<&str>,
    ) -> Result<Vec<Trade>, AppError> {
        todo!("node S green")
    }
    fn record_gap(&mut self, _from: OffsetDateTime, _to: OffsetDateTime) -> Result<(), AppError> {
        todo!("node S green")
    }
    fn gaps_between(
        &self,
        _from: OffsetDateTime,
        _to: OffsetDateTime,
    ) -> Result<Vec<(OffsetDateTime, OffsetDateTime)>, AppError> {
        todo!("node S green")
    }
    fn purge_older_than(&mut self, _cutoff: OffsetDateTime) -> Result<usize, AppError> {
        todo!("node S green")
    }
}

impl WatchStore for SqliteStore {
    fn add(&mut self, _r: &WatchRule) -> Result<(), AppError> {
        todo!("node S green")
    }
    fn list(&self) -> Result<Vec<WatchRule>, AppError> {
        todo!("node S green")
    }
    fn remove(&mut self, _id: &str) -> Result<bool, AppError> {
        todo!("node S green")
    }
}

impl Outbox for SqliteStore {
    fn enqueue_if_new(&mut self, _a: &Alert) -> Result<bool, AppError> {
        todo!("node S green")
    }
    fn drain(&mut self) -> Result<Vec<Alert>, AppError> {
        todo!("node S green")
    }
}

impl BudgetLedger for SqliteBudget {
    fn record_call(&mut self, _at: OffsetDateTime) -> Result<(), AppError> {
        todo!("node S green")
    }
    fn used_on(&self, _d: Date) -> Result<u64, AppError> {
        todo!("node S green")
    }
}
