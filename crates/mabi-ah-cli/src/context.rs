//! Composition root: wires the Nexon client, SQLite store and call meter.
use std::rc::Rc;

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{BudgetLedger, CallMeter, Clock, TradeStore};
use mabi_ah_app::usecases::LedgerMeter;
use mabi_ah_domain::category::CategoryRegistry;
use mabi_ah_domain::option::OptionRegistry;
use mabi_ah_domain::staleness::{DEFAULT_STALE_THRESHOLD, assess};
use mabi_ah_nexon::client::{NexonClient, SystemClock as MonoClock, ThreadSleeper};
use mabi_ah_nexon::key::ApiKey;
use mabi_ah_nexon::limits::CLI_BUCKET_CAPACITY;
use mabi_ah_nexon::rate_limit::TokenBucket;
use mabi_ah_nexon::retry::RetryPolicy;
use mabi_ah_store::{SqliteBudget, SqliteStore};
use time::OffsetDateTime;

use crate::config::{Config, resolve_key_file};
use crate::output::{BudgetMeta, Meta, StalenessMeta, rfc3339};

/// Wall clock for the use cases and the call meter.
#[derive(Debug, Clone, Copy, Default)]
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

pub struct Context {
    pub cfg: Config,
    pub clock: WallClock,
    pub store: SqliteStore,
    /// Read handle on the budget ledger; the meter owns a second connection.
    pub ledger: SqliteBudget,
    pub meter: Rc<LedgerMeter<SqliteBudget, WallClock>>,
    pub options: OptionRegistry,
    pub categories: CategoryRegistry,
}

impl Context {
    pub fn open(cfg: Config) -> Result<Self, AppError> {
        let store = SqliteStore::open(&cfg.db_path)?;
        let ledger = SqliteBudget::open(&cfg.db_path)?;
        let meter = Rc::new(LedgerMeter::new(
            SqliteBudget::open(&cfg.db_path)?,
            WallClock,
            cfg.daily_cap(),
        ));
        let mut categories = CategoryRegistry::new();
        for c in store.discovered_categories()? {
            categories.record_discovered(&c);
        }
        Ok(Self {
            cfg,
            clock: WallClock,
            store,
            ledger,
            meter,
            options: OptionRegistry::default(),
            categories,
        })
    }

    /// Embedded repo data (facts + fresh observed files).
    pub fn repo(&self) -> Result<crate::repo_data::RepoData, AppError> {
        crate::repo_data::RepoData::load(self.clock.now().date())
    }

    pub fn used_today(&self) -> Result<u64, AppError> {
        self.ledger.used_on(self.clock.now().date())
    }

    /// API client for this run. Fails with `Auth` when no key is available.
    /// The token bucket does not burst (capacity 1) because the dev tier
    /// answers a burst of 5 calls with HTTP 429.
    pub fn gateway(&self) -> Result<NexonClient, AppError> {
        let key = ApiKey::load(
            resolve_key_file(self.cfg.key_file.clone()).as_deref(),
            std::env::var("NEXON_OPEN_API_KEY").ok(),
        )
        .map_err(|e| AppError::Auth(e.to_string()))?;
        let meter: Rc<dyn CallMeter> = self.meter.clone();
        let bucket = TokenBucket::new(
            self.cfg.tier.per_second(),
            CLI_BUCKET_CAPACITY,
            MonoClock::default(),
            ThreadSleeper,
        );
        let retry = RetryPolicy::default();
        let base = self
            .cfg
            .base_url
            .clone()
            .unwrap_or_else(|| mabi_ah_nexon::client::DEFAULT_BASE_URL.to_owned());
        Ok(NexonClient::with_parts(
            &base,
            key,
            bucket,
            retry,
            meter,
            Box::new(ThreadSleeper),
        ))
    }

    /// Envelope `meta` for a run that started with `used_before` calls today.
    pub fn meta(&self, used_before: u64) -> Meta {
        build_meta(Some(self), used_before)
    }
}

/// Builds `meta`; without a context (store failed to open) the numbers are
/// zero and staleness is "unknown/stale".
pub fn build_meta(ctx: Option<&Context>, used_before: u64) -> Meta {
    let now = WallClock.now();
    let cfg_tier = ctx.map(|c| c.cfg.tier_name()).unwrap_or("development");
    let (used, limit) = ctx.map_or((0, 0), |c| (c.used_today().unwrap_or(0), c.cfg.daily_cap()));
    let newest = ctx
        .and_then(|c| c.store.newest_trade(None).ok().flatten())
        .map(|t| t.traded_at);
    let st = assess(newest, now, DEFAULT_STALE_THRESHOLD);
    Meta {
        fetched_at: rfc3339(now),
        calls_used: used.saturating_sub(used_before),
        budget: BudgetMeta {
            tier: cfg_tier.to_owned(),
            used_today: used,
            limit_today: limit,
        },
        staleness: StalenessMeta {
            newest_trade_at: st.newest_trade_at.map(rfc3339),
            lag_seconds: st.lag_seconds,
            stale: st.stale,
        },
        attribution: mabi_ah_domain::attribution::ATTRIBUTION,
    }
}
