//! SQLite implementation of the app store ports.
//!
//! Timestamps are stored as INTEGER unix milliseconds (UTC); budget days as
//! `YYYY-MM-DD` UTC text. Schema versions live in `PRAGMA user_version`.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{BudgetLedger, Outbox, TradeStore, WatchStore};
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::ItemOption;
use mabi_ah_domain::trade::Trade;
use mabi_ah_domain::watch::{Alert, WatchRule};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use time::{Date, OffsetDateTime};

/// Each entry is one migration; `user_version` = number applied.
const MIGRATIONS: &[&str] = &[
    "
CREATE TABLE trades (
    trade_id TEXT PRIMARY KEY,
    base_name TEXT NOT NULL,
    raw_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    category TEXT NOT NULL,
    count INTEGER NOT NULL,
    unit_price INTEGER NOT NULL,
    traded_at INTEGER NOT NULL,
    options_json TEXT NOT NULL,
    fetched_at INTEGER NOT NULL
);
CREATE INDEX trades_traded_at ON trades(traded_at);
CREATE INDEX trades_base_name ON trades(base_name, traded_at);
CREATE TABLE gaps (start INTEGER NOT NULL, end INTEGER NOT NULL, UNIQUE(start, end));
CREATE TABLE watch_rules (
    id TEXT PRIMARY KEY,
    base_name TEXT NOT NULL,
    max_unit_price INTEGER,
    category TEXT,
    filters_json TEXT NOT NULL
);
CREATE TABLE alerts_outbox (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    dedupe_key TEXT NOT NULL UNIQUE,
    rule_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    delivered_at INTEGER
);
CREATE TABLE budget_ledger (day TEXT PRIMARY KEY, calls INTEGER NOT NULL);
CREATE TABLE categories_discovered (name TEXT PRIMARY KEY);
",
    "
CREATE TABLE catalog_enchant (
    slot TEXT NOT NULL,
    name TEXT NOT NULL,
    rank TEXT NOT NULL,
    effects_json TEXT NOT NULL DEFAULT '[]',
    observed_count INTEGER NOT NULL,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (slot, name, rank)
);
CREATE TABLE catalog_reforge (
    option_name TEXT NOT NULL,
    slot TEXT NOT NULL,
    category TEXT NOT NULL,
    min_level INTEGER NOT NULL,
    max_level INTEGER NOT NULL,
    observed_count INTEGER NOT NULL,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (option_name, slot, category)
);
CREATE TABLE catalog_option_value (
    kind TEXT NOT NULL,
    option_type TEXT NOT NULL,
    sub_type TEXT NOT NULL DEFAULT '',
    value TEXT NOT NULL,
    category TEXT NOT NULL,
    min_level INTEGER,
    max_level INTEGER,
    extra_json TEXT NOT NULL DEFAULT '',
    observed_count INTEGER NOT NULL,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (kind, option_type, sub_type, value, category)
);
CREATE INDEX catalog_enchant_last_seen ON catalog_enchant(last_seen);
CREATE INDEX catalog_reforge_last_seen ON catalog_reforge(last_seen);
CREATE INDEX catalog_option_value_last_seen ON catalog_option_value(last_seen);
",
];

pub(crate) fn store_err(e: impl std::fmt::Display) -> AppError {
    AppError::Store(e.to_string())
}

pub(crate) fn ms(t: OffsetDateTime) -> i64 {
    i64::try_from(t.unix_timestamp_nanos() / 1_000_000).unwrap_or(i64::MAX)
}

pub(crate) fn from_ms(v: i64) -> Result<OffsetDateTime, AppError> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(v) * 1_000_000).map_err(store_err)
}

pub(crate) fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

pub(crate) fn to_u64(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

fn day_key(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

fn connect(path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(store_err)?;
    }
    let mut conn = Connection::open(path).map_err(store_err)?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(store_err)?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(store_err)?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(store_err)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn migrate(conn: &mut Connection) -> Result<(), AppError> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(store_err)?;
    let current: usize = tx
        .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
        .map_err(store_err)
        .map(|v| usize::try_from(v).unwrap_or(0))?;
    if current > MIGRATIONS.len() {
        return Err(AppError::Store(format!(
            "database schema v{current} is newer than this build (v{})",
            MIGRATIONS.len()
        )));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        tx.execute_batch(sql).map_err(store_err)?;
        tx.pragma_update(
            None,
            "user_version",
            i64::try_from(i + 1).unwrap_or(i64::MAX),
        )
        .map_err(store_err)?;
    }
    tx.commit().map_err(store_err)
}

/// `$XDG_DATA_HOME/mabi-ah`, else `$HOME/.local/share/mabi-ah`, else a
/// relative `.local/share/mabi-ah`.
pub fn data_dir() -> PathBuf {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    if let Some(x) = var("XDG_DATA_HOME") {
        return PathBuf::from(x).join("mabi-ah");
    }
    let home = var("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".local/share/mabi-ah")
}

/// Trades, gaps, watch rules and the alert outbox.
pub struct SqliteStore {
    pub(crate) conn: Connection,
}

/// Budget ledger on its own connection, separate from [`SqliteStore`].
pub struct SqliteBudget {
    conn: Connection,
}

/// Exclusive collect lock: a `<db>.collect.lock` file (created with
/// `create_new`, removed on drop). Acquiring also probes the database with
/// `BEGIN IMMEDIATE` (no wait) and rolls back at once, so a busy writer fails
/// fast without blocking the holder's own inserts afterwards.
#[derive(Debug)]
pub struct CollectLock {
    path: PathBuf,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            conn: connect(path)?,
        })
    }

    pub fn discovered_categories(&self) -> Result<Vec<String>, AppError> {
        let mut st = self
            .conn
            .prepare("SELECT name FROM categories_discovered ORDER BY name")
            .map_err(store_err)?;
        st.query_map([], |r| r.get(0))
            .map_err(store_err)?
            .collect::<Result<_, _>>()
            .map_err(store_err)
    }
}

impl SqliteBudget {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            conn: connect(path)?,
        })
    }
}

/// A lock older than this is stale even if its holder pid looks alive
/// (pid reuse, hung process).
const LOCK_MAX_AGE: Duration = Duration::from_secs(2 * 3600);

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `kill -0` succeeds for a live pid; EPERM means it exists but is not ours.
/// libc is not a normal dependency of this crate, so shell out.
fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| {
            o.status.success()
                || String::from_utf8_lossy(&o.stderr)
                    .to_lowercase()
                    .contains("not permitted")
        })
        .unwrap_or(true) // cannot tell: treat as alive, never steal
}

/// Stale = holder pid dead, or lock older than `LOCK_MAX_AGE`. Content that
/// does not parse (legacy/partial file) falls back to the file mtime.
fn lock_is_stale(path: &Path) -> bool {
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut it = text.split_whitespace();
    let pid = it.next().and_then(|p| p.parse::<u32>().ok());
    let created = it.next().and_then(|t| t.parse::<u64>().ok());
    let (Some(pid), Some(created)) = (pid, created) else {
        let age = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| m.elapsed().ok());
        return age.is_some_and(|a| a > LOCK_MAX_AGE);
    };
    unix_now().saturating_sub(created) > LOCK_MAX_AGE.as_secs() || !pid_alive(pid)
}

fn create_lock_file(path: &Path) -> std::io::Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    writeln!(f, "{} {}", std::process::id(), unix_now())
}

impl CollectLock {
    pub fn acquire(db_path: &Path) -> Result<Self, AppError> {
        let mut name = db_path.as_os_str().to_owned();
        name.push(".collect.lock");
        let path = PathBuf::from(name);
        let mut created = create_lock_file(&path);
        if created.is_err() && lock_is_stale(&path) {
            eprintln!("mabi-ah: taking over stale collect lock {}", path.display());
            let _ = fs::remove_file(&path);
            created = create_lock_file(&path);
        }
        created.map_err(|e| {
            AppError::Store(format!(
                "another collect holds the lock ({e}); if none is running remove {}",
                path.display()
            ))
        })?;
        let lock = Self { path };
        let probe = connect(db_path).and_then(|c| {
            c.busy_timeout(Duration::ZERO).map_err(store_err)?;
            c.execute_batch("BEGIN IMMEDIATE; ROLLBACK;")
                .map_err(store_err)
        });
        probe.map(|()| lock) // on Err, `lock` drops and removes the file
    }
}

impl Drop for CollectLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn trade_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Trade, AppError>> {
    let options_json: String = r.get(7)?;
    let traded: i64 = r.get(6)?;
    let trade = Trade {
        trade_id: r.get(0)?,
        name: ItemName {
            raw: r.get(1)?,
            base: r.get(2)?,
            display: r.get(3)?,
        },
        category: r.get(4)?,
        count: to_u64(r.get(5)?),
        unit_price: to_u64(r.get(8)?),
        traded_at: OffsetDateTime::UNIX_EPOCH,
        options: Vec::new(),
    };
    Ok(from_ms(traded).and_then(|traded_at| {
        Ok(Trade {
            traded_at,
            options: serde_json::from_str(&options_json).map_err(store_err)?,
            ..trade
        })
    }))
}

const TRADE_COLS: &str = "trade_id, raw_name, base_name, display_name, category, count, \
                          traded_at, options_json, unit_price";

fn collect_trades(
    rows: impl Iterator<Item = rusqlite::Result<Result<Trade, AppError>>>,
) -> Result<Vec<Trade>, AppError> {
    rows.map(|r| r.map_err(store_err)?).collect()
}

impl TradeStore for SqliteStore {
    fn insert_trades(&mut self, trades: &[Trade]) -> Result<usize, AppError> {
        let now = ms(OffsetDateTime::now_utc());
        let tx = self.conn.transaction().map_err(store_err)?;
        let mut inserted = 0;
        {
            let mut ins = tx
                .prepare(
                    "INSERT OR IGNORE INTO trades (trade_id, base_name, raw_name, display_name, \
                     category, count, unit_price, traded_at, options_json, fetched_at) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                )
                .map_err(store_err)?;
            let mut cat = tx
                .prepare("INSERT OR IGNORE INTO categories_discovered (name) VALUES (?1)")
                .map_err(store_err)?;
            for t in trades {
                let opts = serde_json::to_string(&t.options).map_err(store_err)?;
                inserted += ins
                    .execute(params![
                        t.trade_id,
                        t.name.base,
                        t.name.raw,
                        t.name.display,
                        t.category,
                        to_i64(t.count),
                        to_i64(t.unit_price),
                        ms(t.traded_at),
                        opts,
                        now
                    ])
                    .map_err(store_err)?;
                cat.execute(params![t.category]).map_err(store_err)?;
            }
        }
        tx.commit().map_err(store_err)?;
        Ok(inserted)
    }

    fn newest_trade(&self, category: Option<&str>) -> Result<Option<Trade>, AppError> {
        let sql = format!(
            "SELECT {TRADE_COLS} FROM trades WHERE (?1 IS NULL OR category = ?1) \
             ORDER BY traded_at DESC, trade_id DESC LIMIT 1"
        );
        let mut st = self.conn.prepare(&sql).map_err(store_err)?;
        let rows = st
            .query_map(params![category], trade_from_row)
            .map_err(store_err)?;
        Ok(collect_trades(rows)?.into_iter().next())
    }

    fn trades_between(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
        base_name: Option<&str>,
    ) -> Result<Vec<Trade>, AppError> {
        let sql = format!(
            "SELECT {TRADE_COLS} FROM trades WHERE traded_at >= ?1 AND traded_at < ?2 \
             AND (?3 IS NULL OR base_name = ?3) ORDER BY traded_at, trade_id"
        );
        let mut st = self.conn.prepare(&sql).map_err(store_err)?;
        let rows = st
            .query_map(params![ms(from), ms(to), base_name], trade_from_row)
            .map_err(store_err)?;
        collect_trades(rows)
    }

    fn record_gap(&mut self, from: OffsetDateTime, to: OffsetDateTime) -> Result<(), AppError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO gaps (start, end) VALUES (?1, ?2)",
                params![ms(from), ms(to)],
            )
            .map(|_| ())
            .map_err(store_err)
    }

    fn gaps_between(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Vec<(OffsetDateTime, OffsetDateTime)>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT start, end FROM gaps WHERE start < ?2 AND ?1 < end ORDER BY start, end",
            )
            .map_err(store_err)?;
        let rows: Vec<(i64, i64)> = st
            .query_map(params![ms(from), ms(to)], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(store_err)?
            .collect::<Result<_, _>>()
            .map_err(store_err)?;
        rows.into_iter()
            .map(|(a, b)| Ok((from_ms(a)?, from_ms(b)?)))
            .collect()
    }

    fn purge_older_than(&mut self, cutoff: OffsetDateTime) -> Result<usize, AppError> {
        let tx = self.conn.transaction().map_err(store_err)?;
        let n = tx
            .execute(
                "DELETE FROM trades WHERE traded_at < ?1",
                params![ms(cutoff)],
            )
            .map_err(store_err)?;
        tx.execute("DELETE FROM gaps WHERE end < ?1", params![ms(cutoff)])
            .map_err(store_err)?;
        tx.commit().map_err(store_err)?;
        Ok(n)
    }
}

impl WatchStore for SqliteStore {
    fn add(&mut self, rule: &WatchRule) -> Result<(), AppError> {
        let filters = serde_json::to_string(&rule.filters).map_err(store_err)?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO watch_rules (id, base_name, max_unit_price, category, \
                 filters_json) VALUES (?1,?2,?3,?4,?5)",
                params![
                    rule.id,
                    rule.base_name,
                    rule.max_unit_price.map(to_i64),
                    rule.category,
                    filters
                ],
            )
            .map(|_| ())
            .map_err(store_err)
    }

    fn list(&self) -> Result<Vec<WatchRule>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id, base_name, max_unit_price, category, filters_json \
                 FROM watch_rules ORDER BY id",
            )
            .map_err(store_err)?;
        let rows = st
            .query_map([], |r| {
                let filters: String = r.get(4)?;
                Ok(WatchRule {
                    id: r.get(0)?,
                    base_name: r.get(1)?,
                    max_unit_price: r.get::<_, Option<i64>>(2)?.map(to_u64),
                    category: r.get(3)?,
                    filters: serde_json::from_str(&filters).unwrap_or_default(),
                })
            })
            .map_err(store_err)?;
        rows.collect::<Result<_, _>>().map_err(store_err)
    }

    fn remove(&mut self, id: &str) -> Result<bool, AppError> {
        let n = self
            .conn
            .execute("DELETE FROM watch_rules WHERE id = ?1", params![id])
            .map_err(store_err)?;
        Ok(n > 0)
    }
}

fn alert_payload(a: &Alert) -> Result<String, AppError> {
    let l = &a.listing;
    let v = json!({
        "raw": l.name.raw,
        "display": l.name.display,
        "count": l.count,
        "category": l.category,
        "unit_price": l.unit_price,
        "expires_at_ms": ms(l.expires_at),
        "options": serde_json::to_value(&l.options).map_err(store_err)?,
    });
    serde_json::to_string(&v).map_err(store_err)
}

fn alert_from_payload(
    rule_id: String,
    dedupe_key: String,
    payload: &str,
) -> Result<Alert, AppError> {
    let v: Value = serde_json::from_str(payload).map_err(store_err)?;
    let text = |k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let num = |k: &str| v.get(k).and_then(Value::as_u64).unwrap_or(0);
    let expires = v.get("expires_at_ms").and_then(Value::as_i64).unwrap_or(0);
    let options: Vec<ItemOption> =
        serde_json::from_value(v.get("options").cloned().unwrap_or(Value::Null))
            .unwrap_or_default();
    Ok(Alert {
        rule_id,
        dedupe_key,
        listing: Listing {
            name: ItemName::new(text("raw"), text("display")),
            count: num("count"),
            category: text("category"),
            unit_price: num("unit_price"),
            expires_at: from_ms(expires)?,
            options,
        },
    })
}

impl Outbox for SqliteStore {
    fn enqueue_if_new(&mut self, alert: &Alert) -> Result<bool, AppError> {
        let payload = alert_payload(alert)?;
        let n = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO alerts_outbox (dedupe_key, rule_id, payload_json, created_at) \
                 VALUES (?1,?2,?3,?4)",
                params![alert.dedupe_key, alert.rule_id, payload, ms(OffsetDateTime::now_utc())],
            )
            .map_err(store_err)?;
        Ok(n > 0)
    }

    fn drain(&mut self) -> Result<Vec<Alert>, AppError> {
        let tx = self.conn.transaction().map_err(store_err)?;
        let pending: Vec<(i64, String, String, String)> = {
            let mut st = tx
                .prepare(
                    "SELECT seq, rule_id, dedupe_key, payload_json FROM alerts_outbox \
                     WHERE delivered_at IS NULL ORDER BY seq",
                )
                .map_err(store_err)?;
            st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
                .map_err(store_err)?
                .collect::<Result<_, _>>()
                .map_err(store_err)?
        };
        let now = ms(OffsetDateTime::now_utc());
        let mut out = Vec::with_capacity(pending.len());
        for (seq, rule, key, payload) in pending {
            out.push(alert_from_payload(rule, key, &payload)?);
            tx.execute(
                "UPDATE alerts_outbox SET delivered_at = ?1 WHERE seq = ?2",
                params![now, seq],
            )
            .map_err(store_err)?;
        }
        tx.commit().map_err(store_err)?;
        Ok(out)
    }
}

impl BudgetLedger for SqliteBudget {
    fn record_call(&mut self, at: OffsetDateTime) -> Result<(), AppError> {
        self.conn
            .execute(
                "INSERT INTO budget_ledger (day, calls) VALUES (?1, 1) \
                 ON CONFLICT(day) DO UPDATE SET calls = calls + 1",
                params![day_key(at.to_offset(time::UtcOffset::UTC).date())],
            )
            .map(|_| ())
            .map_err(store_err)
    }

    fn used_on(&self, date: Date) -> Result<u64, AppError> {
        let n: Option<i64> = self
            .conn
            .query_row(
                "SELECT calls FROM budget_ledger WHERE day = ?1",
                params![day_key(date)],
                |r| r.get(0),
            )
            .optional()
            .map_err(store_err)?;
        Ok(n.map_or(0, to_u64))
    }
}
