//! Port traits (node P): everything the use cases need from the outside world.
//!
//! Adapters (`mabi-ah-nexon`, `mabi-ah-store`) implement these; use cases only
//! see `&dyn`/`&mut dyn` ports. Every port returns [`AppError`], so adapter
//! errors (`NexonError`, `StoreError`) are converted at this boundary.
//!
//! Receivers: the program is single-threaded. Reads take `&self`, writes take
//! `&mut self`. Gateways take `&mut self` because one logical call may retry
//! and advance a token bucket.
//!
//! # Budget counting
//! One unit of Budget is one HTTP request sent to the API, retries included.
//! A gateway is built with a shared [`CallMeter`] and calls
//! [`CallMeter::before_request`] immediately before every request it sends.
//! If that returns an error (normally [`AppError::BudgetExceeded`]) the gateway
//! sends nothing and returns the error unchanged. The meter is the only writer
//! of the [`BudgetLedger`]: it compares `used_on(today)` with the tier limit and
//! then calls `record_call(now)`. Use cases never count calls themselves; they
//! read `used_on` before and after to fill `meta.calls_used` and `meta.budget`.
//! `CallMeter` takes `&self` so the gateway and the composition root can share
//! one meter without `Arc<Mutex>`; an implementation that needs to mutate keeps
//! its ledger behind a `RefCell`.

use time::{Date, OffsetDateTime};

use mabi_ah_domain::catalog::CatalogObservation;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::ItemOption;
use mabi_ah_domain::repo_data::{EnchantEntry, OptionValueEntry, ReforgeEntry, Sourced};
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::watch::{Alert, WatchRule};

use crate::error::AppError;

/// One page of results. `next_cursor` is `None` on the last page.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// Filter for auction listings and trades. The API needs `name` (exact raw
/// item name) or `category`; `cursor` continues a previous page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListingQuery {
    pub name: Option<String>,
    pub category: Option<String>,
    pub cursor: Option<String>,
}

/// NPC shop contents for one NPC on one server channel.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcShop {
    pub npc: String,
    pub server: String,
    pub channel: u32,
    pub fetched_at: OffsetDateTime,
    pub next_update_at: Option<OffsetDateTime>,
    pub tabs: Vec<NpcShopTab>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcShopTab {
    pub name: String,
    pub items: Vec<NpcShopItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcShopItem {
    pub display_name: String,
    pub count: u64,
    pub prices: Vec<NpcShopPrice>,
    pub options: Vec<ItemOption>,
    pub limit_type: Option<String>,
    pub limit_value: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopPrice {
    pub price_type: String,
    pub value: u64,
}

/// One horn-of-bugle world shout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HornMessage {
    pub character_name: String,
    pub message: String,
    pub sent_at: OffsetDateTime,
}

/// Auction house reads. One method call fetches one page.
pub trait AuctionGateway {
    /// Current listings (`/auction/list`).
    fn listings(&mut self, query: &ListingQuery) -> Result<Page<Listing>, AppError>;
    /// Completed trades (`/auction/history`), newest first.
    fn trades(&mut self, query: &ListingQuery) -> Result<Page<Trade>, AppError>;
    /// Listings whose name contains the keywords (`/auction/keyword-search`).
    fn search(
        &mut self,
        keywords: &[String],
        cursor: Option<&str>,
    ) -> Result<Page<Listing>, AppError>;
}

pub trait NpcShopGateway {
    fn npc_shop(&mut self, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError>;
}

pub trait HornGateway {
    fn horn(&mut self, server: &str) -> Result<Vec<HornMessage>, AppError>;
}

/// Collected trades and the gaps the collector could not cover.
pub trait TradeStore {
    /// Inserts trades not yet stored (key: `trade_id`); returns how many were new.
    fn insert_trades(&mut self, trades: &[Trade]) -> Result<usize, AppError>;
    /// Stored trade with the latest `traded_at`, optionally within one category
    /// (the Watermark source).
    fn newest_trade(&self, category: Option<&str>) -> Result<Option<Trade>, AppError>;
    /// Trades with `from <= traded_at < to`, oldest first, optionally for one
    /// `ItemName::base`.
    fn trades_between(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
        base_name: Option<&str>,
    ) -> Result<Vec<Trade>, AppError>;
    /// Records that trades in `[from, to)` may be missing.
    fn record_gap(&mut self, from: OffsetDateTime, to: OffsetDateTime) -> Result<(), AppError>;
    /// Gaps overlapping `[from, to)`, in the shape `domain::trend::trend` takes.
    fn gaps_between(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Vec<(OffsetDateTime, OffsetDateTime)>, AppError>;
    /// Deletes trades with `traded_at < cutoff`; returns how many were deleted.
    fn purge_older_than(&mut self, cutoff: OffsetDateTime) -> Result<usize, AppError>;
}

pub trait WatchStore {
    /// Saves a rule; a rule with the same `id` is replaced.
    fn add(&mut self, rule: &WatchRule) -> Result<(), AppError>;
    fn list(&self) -> Result<Vec<WatchRule>, AppError>;
    /// Returns `false` when no rule had that id.
    fn remove(&mut self, id: &str) -> Result<bool, AppError>;
}

/// Alerts waiting for delivery. `dedupe_key` makes each alert fire once.
pub trait Outbox {
    /// Queues the alert unless its `dedupe_key` was ever queued; `true` if queued.
    fn enqueue_if_new(&mut self, alert: &Alert) -> Result<bool, AppError>;
    /// Returns the undelivered alerts, oldest first, and marks them delivered.
    fn drain(&mut self) -> Result<Vec<Alert>, AppError>;
}

/// Durable count of API requests per UTC calendar day.
pub trait BudgetLedger {
    fn record_call(&mut self, at: OffsetDateTime) -> Result<(), AppError>;
    /// Requests recorded on `date` (UTC).
    fn used_on(&self, date: Date) -> Result<u64, AppError>;
}

/// Invoked by gateways once per HTTP request; see the module docs.
pub trait CallMeter {
    /// Counts one request about to be sent, or refuses it.
    fn before_request(&self) -> Result<(), AppError>;
}

pub trait Clock {
    fn now(&self) -> OffsetDateTime;
}

/// Filters for catalog queries; `None` means no filter. `text` is a substring match
/// (enchant name, reforge option, option value); `category` and `slot` are exact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogQuery {
    pub text: Option<String>,
    pub category: Option<String>,
    pub slot: Option<String>,
    pub limit: Option<usize>,
}

/// The user's local catalog of observed option names (no prices, no people).
pub trait CatalogStore {
    /// Records observations seen at `seen_at`; returns how many were written.
    fn record_catalog(
        &mut self,
        observations: &[CatalogObservation],
        seen_at: OffsetDateTime,
    ) -> Result<usize, AppError>;
    fn catalog_enchants(&self, q: &CatalogQuery) -> Result<Vec<Sourced<EnchantEntry>>, AppError>;
    fn catalog_reforges(&self, q: &CatalogQuery) -> Result<Vec<Sourced<ReforgeEntry>>, AppError>;
    /// Values of one option type (all types when `None`).
    fn catalog_option_values(
        &self,
        option_type: Option<&str>,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<OptionValueEntry>>, AppError>;
    /// Option types with value rows and their row counts, sorted by type.
    fn catalog_option_types(&self) -> Result<Vec<(String, u64)>, AppError>;
    /// Deletes observations older than 30 days before `now`; returns rows removed.
    fn purge_catalog(&mut self, now: OffsetDateTime) -> Result<usize, AppError>;
}
