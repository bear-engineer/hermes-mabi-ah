//! Local catalog tables (`catalog_enchant`, `catalog_reforge`, `catalog_option_value`).
//!
//! Observations from the user's own API calls accumulate here (source
//! `local_observed`); `user_import` rows come from import files in the repo data
//! format. Rows older than 30 days (by `last_seen`) are purged. Unparsed options are
//! kept raw in `catalog_option_value` with `kind = 'raw'`. No prices or people are stored.
//!
//! Upsert rule: counts add up for live observations and take the max for imports (so
//! re-importing a file is idempotent); first/last seen and level ranges widen; the row's
//! source and enchant effects follow the newest `last_seen` (ties go to the new write).

use mabi_ah_app::error::AppError;
use mabi_ah_domain::catalog::CatalogObservation;
use mabi_ah_domain::option::ItemOption;
use mabi_ah_domain::option::enchant_rank::EnchantRank;
use mabi_ah_domain::repo_data::{
    CatalogSource, DataEntry, EnchantEntry, OptionValueEntry, ReforgeEntry, Sourced, format_date,
    load_observed, parse_date,
};
use rusqlite::{Transaction, params};
use time::{Date, Duration, OffsetDateTime};

use crate::store::{SqliteStore, from_ms, ms, store_err, to_i64, to_u64};

/// Rows older than this many days (by `last_seen`) are purged.
pub const PURGE_DAYS: i64 = 30;

/// Filters for catalog queries; `None` means no filter. `text` is a substring match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogQuery {
    pub text: Option<String>,
    pub category: Option<String>,
    pub slot: Option<String>,
    pub limit: Option<usize>,
}

/// How many rows an import wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub enchants: usize,
    pub reforges: usize,
    pub option_values: usize,
}

/// An option kept raw because nothing could interpret it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnparsedRow {
    pub category: String,
    pub raw: ItemOption,
    pub observed_count: u64,
    pub last_seen: String,
}

fn day_ms(date: &str) -> Result<i64, AppError> {
    let d = parse_date(date).map_err(|e| AppError::Usage(e.to_string()))?;
    Ok(ms(d.midnight().assume_utc()))
}

fn day_text(v: i64) -> Result<String, AppError> {
    Ok(format_date(from_ms(v)?.date()))
}

fn source_of(s: &str) -> Result<CatalogSource, AppError> {
    s.parse().map_err(store_err)
}

fn opt_u32(v: Option<i64>) -> Option<u32> {
    v.and_then(|n| u32::try_from(n).ok())
}

/// `MIN`/`MAX` over nullable levels: NULL means "no level seen".
const LEVEL_MIN: &str = "CASE WHEN min_level IS NULL THEN excluded.min_level \
    WHEN excluded.min_level IS NULL THEN min_level ELSE MIN(min_level, excluded.min_level) END";
const LEVEL_MAX: &str = "CASE WHEN max_level IS NULL THEN excluded.max_level \
    WHEN excluded.max_level IS NULL THEN max_level ELSE MAX(max_level, excluded.max_level) END";

struct Stamp {
    count: i64,
    first: i64,
    last: i64,
    source: CatalogSource,
    additive: bool,
}

fn upsert_enchant(
    tx: &Transaction<'_>,
    slot: &str,
    name: &str,
    rank: &str,
    effects: &[String],
    s: &Stamp,
) -> Result<(), AppError> {
    let effects_json = serde_json::to_string(effects).map_err(store_err)?;
    tx.execute(
        "INSERT INTO catalog_enchant(slot,name,rank,effects_json,observed_count,first_seen,last_seen,source) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8) \
         ON CONFLICT(slot,name,rank) DO UPDATE SET \
           effects_json = CASE WHEN excluded.effects_json <> '[]' AND excluded.last_seen >= last_seen \
                               THEN excluded.effects_json ELSE effects_json END, \
           observed_count = CASE WHEN ?9 THEN observed_count + excluded.observed_count \
                                 ELSE MAX(observed_count, excluded.observed_count) END, \
           first_seen = MIN(first_seen, excluded.first_seen), \
           source = CASE WHEN excluded.last_seen >= last_seen THEN excluded.source ELSE source END, \
           last_seen = MAX(last_seen, excluded.last_seen)",
        params![slot, name, rank, effects_json, s.count, s.first, s.last, s.source.as_str(), s.additive],
    )
    .map_err(store_err)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn upsert_reforge(
    tx: &Transaction<'_>,
    option: &str,
    slot: &str,
    category: &str,
    min_level: i64,
    max_level: i64,
    s: &Stamp,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO catalog_reforge(option_name,slot,category,min_level,max_level,observed_count,first_seen,last_seen,source) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) \
         ON CONFLICT(option_name,slot,category) DO UPDATE SET \
           min_level = MIN(min_level, excluded.min_level), \
           max_level = MAX(max_level, excluded.max_level), \
           observed_count = CASE WHEN ?10 THEN observed_count + excluded.observed_count \
                                 ELSE MAX(observed_count, excluded.observed_count) END, \
           first_seen = MIN(first_seen, excluded.first_seen), \
           source = CASE WHEN excluded.last_seen >= last_seen THEN excluded.source ELSE source END, \
           last_seen = MAX(last_seen, excluded.last_seen)",
        params![option, slot, category, min_level, max_level, s.count, s.first, s.last, s.source.as_str(), s.additive],
    )
    .map_err(store_err)?;
    Ok(())
}

struct ValueRow<'a> {
    kind: &'a str,
    option_type: &'a str,
    sub_type: &'a str,
    value: &'a str,
    category: &'a str,
    min_level: Option<i64>,
    max_level: Option<i64>,
    extra_json: &'a str,
}

fn upsert_value(tx: &Transaction<'_>, r: &ValueRow<'_>, s: &Stamp) -> Result<(), AppError> {
    let sql = format!(
        "INSERT INTO catalog_option_value(kind,option_type,sub_type,value,category,min_level,max_level,extra_json,observed_count,first_seen,last_seen,source) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) \
         ON CONFLICT(kind,option_type,sub_type,value,category) DO UPDATE SET \
           min_level = {LEVEL_MIN}, max_level = {LEVEL_MAX}, \
           extra_json = CASE WHEN excluded.last_seen >= last_seen THEN excluded.extra_json ELSE extra_json END, \
           observed_count = CASE WHEN ?13 THEN observed_count + excluded.observed_count \
                                 ELSE MAX(observed_count, excluded.observed_count) END, \
           first_seen = MIN(first_seen, excluded.first_seen), \
           source = CASE WHEN excluded.last_seen >= last_seen THEN excluded.source ELSE source END, \
           last_seen = MAX(last_seen, excluded.last_seen)"
    );
    tx.execute(
        &sql,
        params![
            r.kind,
            r.option_type,
            r.sub_type,
            r.value,
            r.category,
            r.min_level,
            r.max_level,
            r.extra_json,
            s.count,
            s.first,
            s.last,
            s.source.as_str(),
            s.additive
        ],
    )
    .map_err(store_err)?;
    Ok(())
}

impl SqliteStore {
    /// Record observations seen at `seen_at` (source `local_observed`, count +1 each).
    /// Returns the number of observations written.
    pub fn record_catalog(
        &mut self,
        observations: &[CatalogObservation],
        seen_at: OffsetDateTime,
    ) -> Result<usize, AppError> {
        todo!()
    }

    /// Import repo-format enchant entries (idempotent: counts take the max).
    pub fn import_enchants(
        &mut self,
        entries: &[EnchantEntry],
        source: CatalogSource,
    ) -> Result<usize, AppError> {
        todo!()
    }

    /// Import repo-format reforge entries.
    pub fn import_reforges(
        &mut self,
        entries: &[ReforgeEntry],
        source: CatalogSource,
    ) -> Result<usize, AppError> {
        todo!()
    }

    /// Import repo-format option value entries.
    pub fn import_option_values(
        &mut self,
        entries: &[OptionValueEntry],
        source: CatalogSource,
    ) -> Result<usize, AppError> {
        todo!()
    }

    /// `catalog import`: read a repo-format observed file (`observed_enchant`,
    /// `observed_reforge` or `observed_option_value`) as source `user_import`.
    /// Files that fail validation (unknown or price/person fields, bad dates) or are
    /// already expired on `today` are rejected whole.
    pub fn import_catalog_json(
        &mut self,
        json: &str,
        today: Date,
    ) -> Result<ImportReport, AppError> {
        todo!()
    }

    /// Enchants by name substring and/or slot, sorted by name, slot, then rank (weakest first).
    pub fn catalog_enchants(
        &self,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<EnchantEntry>>, AppError> {
        todo!()
    }

    /// Reforge options by option-name substring and/or exact equipment category.
    pub fn catalog_reforges(
        &self,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<ReforgeEntry>>, AppError> {
        todo!()
    }

    /// Option values (not raw rows) of one `option_type` (or all when `None`), filtered by
    /// value substring and exact category.
    pub fn catalog_option_values(
        &self,
        option_type: Option<&str>,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<OptionValueEntry>>, AppError> {
        todo!()
    }

    /// Distinct option types with value rows and their row counts, sorted by type.
    pub fn catalog_option_types(&self) -> Result<Vec<(String, u64)>, AppError> {
        todo!()
    }

    /// Raw options kept because they could not be interpreted, most recent first.
    pub fn catalog_unparsed(&self, limit: Option<usize>) -> Result<Vec<UnparsedRow>, AppError> {
        todo!()
    }

    /// Delete rows whose `last_seen` is older than [`PURGE_DAYS`] before `now` (all sources).
    /// Returns the number of rows removed.
    pub fn purge_catalog(&mut self, now: OffsetDateTime) -> Result<usize, AppError> {
        todo!()
    }
}

fn to_i64_usize(v: usize) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}
