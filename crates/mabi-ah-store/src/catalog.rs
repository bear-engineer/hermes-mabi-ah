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
        let t = ms(seen_at);
        let stamp = Stamp {
            count: 1,
            first: t,
            last: t,
            source: CatalogSource::LocalObserved,
            additive: true,
        };
        let tx = self.conn.transaction().map_err(store_err)?;
        for o in observations {
            match o {
                CatalogObservation::Enchant {
                    slot,
                    name,
                    rank,
                    effects,
                } => upsert_enchant(&tx, slot, name, &rank.to_string(), effects, &stamp)?,
                CatalogObservation::Reforge {
                    option,
                    level,
                    slot,
                    category,
                } => {
                    let l = i64::from(*level);
                    upsert_reforge(&tx, option, slot, category, l, l, &stamp)?;
                }
                CatalogObservation::OptionValue {
                    option_type,
                    sub_type,
                    value,
                    level,
                    category,
                } => {
                    let l = level.map(i64::from);
                    upsert_value(
                        &tx,
                        &ValueRow {
                            kind: "value",
                            option_type,
                            sub_type: sub_type.as_deref().unwrap_or(""),
                            value,
                            category,
                            min_level: l,
                            max_level: l,
                            extra_json: "",
                        },
                        &stamp,
                    )?;
                }
                CatalogObservation::Unparsed { category, raw } => {
                    let extra = serde_json::to_string(raw).map_err(store_err)?;
                    upsert_value(
                        &tx,
                        &ValueRow {
                            kind: "raw",
                            option_type: &raw.option_type,
                            sub_type: raw.sub_type.as_deref().unwrap_or(""),
                            value: raw.value.as_deref().unwrap_or(""),
                            category,
                            min_level: None,
                            max_level: None,
                            extra_json: &extra,
                        },
                        &stamp,
                    )?;
                }
            }
        }
        tx.commit().map_err(store_err)?;
        Ok(observations.len())
    }

    /// Import repo-format enchant entries (idempotent: counts take the max).
    pub fn import_enchants(
        &mut self,
        entries: &[EnchantEntry],
        source: CatalogSource,
    ) -> Result<usize, AppError> {
        let tx = self.conn.transaction().map_err(store_err)?;
        for e in entries {
            e.validate().map_err(|x| AppError::Usage(x.to_string()))?;
            let s = Stamp {
                count: to_i64(e.observed_count),
                first: day_ms(&e.first_seen)?,
                last: day_ms(&e.last_seen)?,
                source,
                additive: false,
            };
            upsert_enchant(&tx, &e.slot, &e.name, &e.rank, &e.effects, &s)?;
        }
        tx.commit().map_err(store_err)?;
        Ok(entries.len())
    }

    /// Import repo-format reforge entries.
    pub fn import_reforges(
        &mut self,
        entries: &[ReforgeEntry],
        source: CatalogSource,
    ) -> Result<usize, AppError> {
        let tx = self.conn.transaction().map_err(store_err)?;
        for e in entries {
            e.validate().map_err(|x| AppError::Usage(x.to_string()))?;
            let s = Stamp {
                count: to_i64(e.observed_count),
                first: day_ms(&e.first_seen)?,
                last: day_ms(&e.last_seen)?,
                source,
                additive: false,
            };
            upsert_reforge(
                &tx,
                &e.option,
                &e.slot,
                &e.category,
                i64::from(e.min_level),
                i64::from(e.max_level),
                &s,
            )?;
        }
        tx.commit().map_err(store_err)?;
        Ok(entries.len())
    }

    /// Import repo-format option value entries.
    pub fn import_option_values(
        &mut self,
        entries: &[OptionValueEntry],
        source: CatalogSource,
    ) -> Result<usize, AppError> {
        let tx = self.conn.transaction().map_err(store_err)?;
        for e in entries {
            e.validate().map_err(|x| AppError::Usage(x.to_string()))?;
            let s = Stamp {
                count: to_i64(e.observed_count),
                first: day_ms(&e.first_seen)?,
                last: day_ms(&e.last_seen)?,
                source,
                additive: false,
            };
            upsert_value(
                &tx,
                &ValueRow {
                    kind: "value",
                    option_type: &e.option_type,
                    sub_type: e.sub_type.as_deref().unwrap_or(""),
                    value: &e.value,
                    category: &e.category,
                    min_level: e.min_level.map(i64::from),
                    max_level: e.max_level.map(i64::from),
                    extra_json: "",
                },
                &s,
            )?;
        }
        tx.commit().map_err(store_err)?;
        Ok(entries.len())
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
        let kind = serde_json::from_str::<serde_json::Value>(json)
            .map_err(|e| AppError::Usage(format!("import file: {e}")))?
            .get("kind")
            .and_then(|k| k.as_str().map(str::to_owned))
            .ok_or_else(|| AppError::Usage("import file: missing kind".into()))?;
        let usage = |e: mabi_ah_domain::repo_data::RepoDataError| AppError::Usage(e.to_string());
        let expired = || AppError::Usage("import file is expired (expires_at passed)".into());
        let mut report = ImportReport::default();
        match kind.as_str() {
            k if k == EnchantEntry::KIND => {
                let f = load_observed::<EnchantEntry>(json).map_err(usage)?;
                if f.is_expired(today) {
                    return Err(expired());
                }
                report.enchants = self.import_enchants(&f.entries, CatalogSource::UserImport)?;
            }
            k if k == ReforgeEntry::KIND => {
                let f = load_observed::<ReforgeEntry>(json).map_err(usage)?;
                if f.is_expired(today) {
                    return Err(expired());
                }
                report.reforges = self.import_reforges(&f.entries, CatalogSource::UserImport)?;
            }
            k if k == OptionValueEntry::KIND => {
                let f = load_observed::<OptionValueEntry>(json).map_err(usage)?;
                if f.is_expired(today) {
                    return Err(expired());
                }
                report.option_values =
                    self.import_option_values(&f.entries, CatalogSource::UserImport)?;
            }
            other => {
                return Err(AppError::Usage(format!(
                    "import file: unknown kind {other:?}"
                )));
            }
        }
        Ok(report)
    }

    /// Enchants by name substring and/or slot, sorted by name, slot, then rank (weakest first).
    pub fn catalog_enchants(
        &self,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<EnchantEntry>>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT slot,name,rank,effects_json,observed_count,first_seen,last_seen,source \
                 FROM catalog_enchant \
                 WHERE (?1 IS NULL OR instr(name, ?1) > 0) AND (?2 IS NULL OR slot = ?2) \
                 ORDER BY name, slot",
            )
            .map_err(store_err)?;
        let rows = st
            .query_map(params![q.text, q.slot], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })
            .map_err(store_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (slot, name, rank, effects, count, first, last, source) = row.map_err(store_err)?;
            out.push(Sourced {
                source: source_of(&source)?,
                entry: EnchantEntry {
                    slot,
                    name,
                    rank,
                    effects: serde_json::from_str(&effects).map_err(store_err)?,
                    observed_count: to_u64(count),
                    first_seen: day_text(first)?,
                    last_seen: day_text(last)?,
                },
            });
        }
        let ord = |e: &EnchantEntry| e.rank.parse::<EnchantRank>().map(EnchantRank::ordinal).ok();
        out.sort_by(|a, b| {
            (&a.entry.name, &a.entry.slot, ord(&a.entry)).cmp(&(
                &b.entry.name,
                &b.entry.slot,
                ord(&b.entry),
            ))
        });
        if let Some(n) = q.limit {
            out.truncate(n);
        }
        Ok(out)
    }

    /// Reforge options by option-name substring and/or exact equipment category.
    pub fn catalog_reforges(
        &self,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<ReforgeEntry>>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT option_name,slot,category,min_level,max_level,observed_count,first_seen,last_seen,source \
                 FROM catalog_reforge \
                 WHERE (?1 IS NULL OR instr(option_name, ?1) > 0) \
                   AND (?2 IS NULL OR category = ?2) AND (?3 IS NULL OR slot = ?3) \
                 ORDER BY option_name, category, slot LIMIT ?4",
            )
            .map_err(store_err)?;
        let limit = q.limit.map_or(-1, to_i64_usize);
        let rows = st
            .query_map(params![q.text, q.category, q.slot, limit], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, String>(8)?,
                ))
            })
            .map_err(store_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (option, slot, category, min, max, count, first, last, source) =
                row.map_err(store_err)?;
            out.push(Sourced {
                source: source_of(&source)?,
                entry: ReforgeEntry {
                    option,
                    slot,
                    category,
                    min_level: opt_u32(Some(min)).unwrap_or(0),
                    max_level: opt_u32(Some(max)).unwrap_or(0),
                    observed_count: to_u64(count),
                    first_seen: day_text(first)?,
                    last_seen: day_text(last)?,
                },
            });
        }
        Ok(out)
    }

    /// Option values (not raw rows) of one `option_type` (or all when `None`), filtered by
    /// value substring and exact category.
    pub fn catalog_option_values(
        &self,
        option_type: Option<&str>,
        q: &CatalogQuery,
    ) -> Result<Vec<Sourced<OptionValueEntry>>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT option_type,sub_type,value,category,min_level,max_level,observed_count,first_seen,last_seen,source \
                 FROM catalog_option_value \
                 WHERE kind = 'value' AND (?1 IS NULL OR option_type = ?1) \
                   AND (?2 IS NULL OR instr(value, ?2) > 0) AND (?3 IS NULL OR category = ?3) \
                 ORDER BY option_type, value, sub_type, category LIMIT ?4",
            )
            .map_err(store_err)?;
        let limit = q.limit.map_or(-1, to_i64_usize);
        let rows = st
            .query_map(params![option_type, q.text, q.category, limit], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, i64>(8)?,
                    r.get::<_, String>(9)?,
                ))
            })
            .map_err(store_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (ty, sub, value, category, min, max, count, first, last, source) =
                row.map_err(store_err)?;
            out.push(Sourced {
                source: source_of(&source)?,
                entry: OptionValueEntry {
                    option_type: ty,
                    sub_type: Some(sub).filter(|s| !s.is_empty()),
                    value,
                    category,
                    min_level: opt_u32(min),
                    max_level: opt_u32(max),
                    observed_count: to_u64(count),
                    first_seen: day_text(first)?,
                    last_seen: day_text(last)?,
                },
            });
        }
        Ok(out)
    }

    /// Distinct option types with value rows and their row counts, sorted by type.
    pub fn catalog_option_types(&self) -> Result<Vec<(String, u64)>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT option_type, COUNT(*) FROM catalog_option_value \
                 WHERE kind = 'value' GROUP BY option_type ORDER BY option_type",
            )
            .map_err(store_err)?;
        st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, to_u64(r.get::<_, i64>(1)?)))
        })
        .map_err(store_err)?
        .collect::<Result<_, _>>()
        .map_err(store_err)
    }

    /// Raw options kept because they could not be interpreted, most recent first.
    pub fn catalog_unparsed(&self, limit: Option<usize>) -> Result<Vec<UnparsedRow>, AppError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT category, extra_json, observed_count, last_seen FROM catalog_option_value \
                 WHERE kind = 'raw' ORDER BY last_seen DESC, option_type, value LIMIT ?1",
            )
            .map_err(store_err)?;
        let rows = st
            .query_map(params![limit.map_or(-1, to_i64_usize)], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(store_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (category, raw, count, last) = row.map_err(store_err)?;
            out.push(UnparsedRow {
                category,
                raw: serde_json::from_str(&raw).map_err(store_err)?,
                observed_count: to_u64(count),
                last_seen: day_text(last)?,
            });
        }
        Ok(out)
    }

    /// Delete rows whose `last_seen` is older than [`PURGE_DAYS`] before `now` (all sources).
    /// Returns the number of rows removed.
    pub fn purge_catalog(&mut self, now: OffsetDateTime) -> Result<usize, AppError> {
        let cutoff = ms(now - Duration::days(PURGE_DAYS));
        let tx = self.conn.transaction().map_err(store_err)?;
        let mut n = 0;
        for t in ["catalog_enchant", "catalog_reforge", "catalog_option_value"] {
            n += tx
                .execute(&format!("DELETE FROM {t} WHERE last_seen < ?1"), [cutoff])
                .map_err(store_err)?;
        }
        tx.commit().map_err(store_err)?;
        Ok(n)
    }
}

fn to_i64_usize(v: usize) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}
