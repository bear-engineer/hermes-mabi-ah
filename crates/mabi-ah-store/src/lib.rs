//! SQLite adapter (implements app store ports).
//!
//! `SqliteStore` implements `TradeStore`, `WatchStore` and `Outbox`.
//! `SqliteBudget` is a separate handle with its own connection and implements
//! `BudgetLedger`, so a call meter can own it while a use case holds the store.

pub mod catalog;
mod store;

pub use store::{CollectLock, SqliteBudget, SqliteStore, data_dir};
