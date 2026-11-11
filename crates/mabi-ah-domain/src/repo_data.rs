//! Repo-managed catalog data: schema, loaders, merge rule and the export whitelist.
//!
//! Files live under `data/facts/*.json` (hand-authored facts) and
//! `data/observed/*.json` (API-observed catalog). Both share one envelope
//! ([`DataFile`]): `schema_version`, `kind`, `attribution`, `observed_from`,
//! `observed_to`, `expires_at`, `entries`. Observed files must carry the three
//! dates (expiry at most 30 days after `observed_to`); facts files may omit them.
//!
//! Entry structs reject unknown fields, so a file with `price` or a seller
//! field fails to load. Pure functions over `&str`; no I/O.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::macros::format_description;
use time::{Date, Duration};

use crate::attribution::ATTRIBUTION;
use crate::option::enchant_rank::EnchantRank;

/// Current envelope version.
pub const SCHEMA_VERSION: u32 = 1;
/// Observed data may be valid at most this many days after `observed_to` (NEXON 30-day rule).
pub const MAX_VALIDITY_DAYS: i64 = 30;
/// Key fragments that must never appear in repo data (checked case-insensitively).
pub const FORBIDDEN_KEY_FRAGMENTS: [&str; 12] = [
    "price",
    "seller",
    "buyer",
    "character",
    "horn",
    "token",
    "api_key",
    "email",
    "nickname",
    "trade_id",
    "user",
    "cost",
];

/// Why a data file was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepoDataError {
    #[error("json: {0}")]
    Json(String),
    #[error("unsupported schema_version {0}")]
    SchemaVersion(u32),
    #[error("kind mismatch: expected {expected}, found {found}")]
    Kind {
        expected: &'static str,
        found: String,
    },
    #[error("missing field: {0}")]
    Missing(&'static str),
    #[error("invalid date: {0}")]
    Date(String),
    #[error("invalid date window: {0}")]
    Window(String),
    #[error("invalid entry: {0}")]
    Entry(String),
}

/// Where a catalog entry came from. Ties on observation date go to the higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSource {
    RepoFact,
    RepoObserved,
    LocalObserved,
    UserImport,
}

impl CatalogSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepoFact => "repo_fact",
            Self::RepoObserved => "repo_observed",
            Self::LocalObserved => "local_observed",
            Self::UserImport => "user_import",
        }
    }

    /// Tie-break rank: user_import > local_observed > repo_observed > repo_fact.
    pub fn priority(self) -> u8 {
        match self {
            Self::RepoFact => 0,
            Self::RepoObserved => 1,
            Self::LocalObserved => 2,
            Self::UserImport => 3,
        }
    }
}

impl FromStr for CatalogSource {
    type Err = RepoDataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "repo_fact" => Ok(Self::RepoFact),
            "repo_observed" => Ok(Self::RepoObserved),
            "local_observed" => Ok(Self::LocalObserved),
            "user_import" => Ok(Self::UserImport),
            other => Err(RepoDataError::Entry(format!("unknown source {other:?}"))),
        }
    }
}

/// An entry with the source it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sourced<T> {
    pub source: CatalogSource,
    pub entry: T,
}

/// Common envelope of every data file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataFile<T> {
    pub schema_version: u32,
    pub kind: String,
    pub attribution: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub entries: Vec<T>,
}

impl<T> DataFile<T> {
    /// `true` when `expires_at` is set and earlier than `today`.
    pub fn is_expired(&self, today: Date) -> bool {
        self.expires_at
            .as_deref()
            .and_then(|d| parse_date(d).ok())
            .is_some_and(|e| e < today)
    }
}

/// Entry types a data file can hold.
pub trait DataEntry {
    /// Value of the file's `kind`.
    const KIND: &'static str;
    fn validate(&self) -> Result<(), RepoDataError>;
}

/// Observed entries: keyed, dated, mergeable.
pub trait ObservedEntry: DataEntry {
    /// Identity used for merging and sorting.
    fn key(&self) -> String;
    /// `YYYY-MM-DD`.
    fn last_seen(&self) -> &str;
}

/// Parse `YYYY-MM-DD`.
pub fn parse_date(s: &str) -> Result<Date, RepoDataError> {
    Date::parse(s, format_description!("[year]-[month]-[day]"))
        .map_err(|_| RepoDataError::Date(s.to_owned()))
}

/// Format a date as `YYYY-MM-DD`.
pub fn format_date(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

// ---------- observed entries (whitelisted fields only) ----------

/// Observed enchant: name, slot (접두/접미), rank, effect text, counts, dates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnchantEntry {
    pub slot: String,
    pub name: String,
    pub rank: String,
    #[serde(default)]
    pub effects: Vec<String>,
    pub observed_count: u64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Observed reforge option per (option name, slot, equipment category).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReforgeEntry {
    pub option: String,
    pub slot: String,
    pub category: String,
    pub min_level: u32,
    pub max_level: u32,
    pub observed_count: u64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Observed option value (e.g. totem effect, erg grade) per category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionValueEntry {
    pub option_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
    pub value: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_level: Option<u32>,
    pub observed_count: u64,
    pub first_seen: String,
    pub last_seen: String,
}

fn check_dates(first: &str, last: &str) -> Result<(), RepoDataError> {
    let (f, l) = (parse_date(first)?, parse_date(last)?);
    if f > l {
        return Err(RepoDataError::Entry(format!(
            "first_seen {first} after last_seen {last}"
        )));
    }
    Ok(())
}

fn non_empty(field: &str, v: &str) -> Result<(), RepoDataError> {
    if v.trim().is_empty() {
        return Err(RepoDataError::Entry(format!("empty {field}")));
    }
    Ok(())
}

impl DataEntry for EnchantEntry {
    const KIND: &'static str = "observed_enchant";
    fn validate(&self) -> Result<(), RepoDataError> {
        non_empty("name", &self.name)?;
        non_empty("slot", &self.slot)?;
        EnchantRank::from_str(&self.rank).map_err(|e| RepoDataError::Entry(e.to_string()))?;
        check_dates(&self.first_seen, &self.last_seen)
    }
}

impl ObservedEntry for EnchantEntry {
    fn key(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}", self.slot, self.name, self.rank)
    }
    fn last_seen(&self) -> &str {
        &self.last_seen
    }
}

impl DataEntry for ReforgeEntry {
    const KIND: &'static str = "observed_reforge";
    fn validate(&self) -> Result<(), RepoDataError> {
        non_empty("option", &self.option)?;
        non_empty("category", &self.category)?;
        if self.min_level > self.max_level {
            return Err(RepoDataError::Entry("min_level above max_level".into()));
        }
        check_dates(&self.first_seen, &self.last_seen)
    }
}

impl ObservedEntry for ReforgeEntry {
    fn key(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}", self.option, self.slot, self.category)
    }
    fn last_seen(&self) -> &str {
        &self.last_seen
    }
}

impl DataEntry for OptionValueEntry {
    const KIND: &'static str = "observed_option_value";
    fn validate(&self) -> Result<(), RepoDataError> {
        non_empty("option_type", &self.option_type)?;
        non_empty("value", &self.value)?;
        if let (Some(a), Some(b)) = (self.min_level, self.max_level)
            && a > b
        {
            return Err(RepoDataError::Entry("min_level above max_level".into()));
        }
        check_dates(&self.first_seen, &self.last_seen)
    }
}

impl ObservedEntry for OptionValueEntry {
    fn key(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.option_type,
            self.sub_type.as_deref().unwrap_or(""),
            self.value,
            self.category
        )
    }
    fn last_seen(&self) -> &str {
        &self.last_seen
    }
}

// ---------- facts entries (hand-authored) ----------

/// `enchant_slot` fact: an enchant slot name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnchantSlotFact {
    pub slot: String,
    pub note: String,
}

/// `enchant_rank` fact: rank text and its position (0 = weakest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnchantRankFact {
    pub rank: String,
    pub order: u8,
}

/// `option_family` fact: how one `option_type` is shaped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionFamilyFact {
    pub option_type: String,
    pub sub_type: String,
    pub value_format: String,
    pub value2_format: String,
    /// `observed` or `unobserved` (format not seen in live data; meaning not interpreted).
    pub status: String,
}

/// `equipment_type` fact: an auction category that can carry enchants/reforges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentTypeFact {
    pub category: String,
    pub group: String,
}

impl DataEntry for EnchantSlotFact {
    const KIND: &'static str = "enchant_slot";
    fn validate(&self) -> Result<(), RepoDataError> {
        non_empty("slot", &self.slot)
    }
}

impl DataEntry for EnchantRankFact {
    const KIND: &'static str = "enchant_rank";
    fn validate(&self) -> Result<(), RepoDataError> {
        EnchantRank::from_str(&self.rank)
            .map(|_| ())
            .map_err(|e| RepoDataError::Entry(e.to_string()))
    }
}

impl DataEntry for OptionFamilyFact {
    const KIND: &'static str = "option_family";
    fn validate(&self) -> Result<(), RepoDataError> {
        non_empty("option_type", &self.option_type)?;
        match self.status.as_str() {
            "observed" | "unobserved" => Ok(()),
            other => Err(RepoDataError::Entry(format!("unknown status {other:?}"))),
        }
    }
}

impl DataEntry for EquipmentTypeFact {
    const KIND: &'static str = "equipment_type";
    fn validate(&self) -> Result<(), RepoDataError> {
        non_empty("category", &self.category)
    }
}

// ---------- loading ----------

fn parse<T: DataEntry + DeserializeOwned>(json: &str) -> Result<DataFile<T>, RepoDataError> {
    let file: DataFile<T> =
        serde_json::from_str(json).map_err(|e| RepoDataError::Json(e.to_string()))?;
    if file.schema_version != SCHEMA_VERSION {
        return Err(RepoDataError::SchemaVersion(file.schema_version));
    }
    if file.kind != T::KIND {
        return Err(RepoDataError::Kind {
            expected: T::KIND,
            found: file.kind,
        });
    }
    non_empty("attribution", &file.attribution)?;
    for e in &file.entries {
        e.validate()?;
    }
    Ok(file)
}

/// Load a facts file; the date fields are optional.
pub fn load_facts<T: DataEntry + DeserializeOwned>(
    json: &str,
) -> Result<DataFile<T>, RepoDataError> {
    parse(json)
}

/// Load an observed file: all three dates required, `observed_from <= observed_to <
/// expires_at <= observed_to + 30 days`.
pub fn load_observed<T: ObservedEntry + DeserializeOwned>(
    json: &str,
) -> Result<DataFile<T>, RepoDataError> {
    let file = parse::<T>(json)?;
    let from = parse_date(
        file.observed_from
            .as_deref()
            .ok_or(RepoDataError::Missing("observed_from"))?,
    )?;
    let to = parse_date(
        file.observed_to
            .as_deref()
            .ok_or(RepoDataError::Missing("observed_to"))?,
    )?;
    let exp = parse_date(
        file.expires_at
            .as_deref()
            .ok_or(RepoDataError::Missing("expires_at"))?,
    )?;
    if from > to {
        return Err(RepoDataError::Window(
            "observed_from after observed_to".into(),
        ));
    }
    if exp <= to {
        return Err(RepoDataError::Window(
            "expires_at not after observed_to".into(),
        ));
    }
    if exp - to > Duration::days(MAX_VALIDITY_DAYS) {
        return Err(RepoDataError::Window(format!(
            "expires_at more than {MAX_VALIDITY_DAYS} days after observed_to"
        )));
    }
    Ok(file)
}

// ---------- merge ----------

/// Merge entries from several sources: per key the newest `last_seen` wins; on a tie the
/// higher [`CatalogSource::priority`]. Output is sorted by key.
pub fn merge<T: ObservedEntry>(items: impl IntoIterator<Item = Sourced<T>>) -> Vec<Sourced<T>> {
    let mut best: BTreeMap<String, Sourced<T>> = BTreeMap::new();
    for item in items {
        let key = item.entry.key();
        let better = match best.get(&key) {
            None => true,
            Some(cur) => {
                (item.entry.last_seen(), item.source.priority())
                    > (cur.entry.last_seen(), cur.source.priority())
            }
        };
        if better {
            best.insert(key, item);
        }
    }
    best.into_values().collect()
}

// ---------- export whitelist ----------

/// Observation window of an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportWindow {
    pub observed_from: Date,
    pub observed_to: Date,
}

/// Serialize observed entries to the repo file format: sorted by key, pretty JSON,
/// trailing newline, `expires_at = observed_to + 30 days`. Only whitelisted entry
/// fields exist on the entry types, so prices and person data cannot be written.
pub fn to_observed_json<T: ObservedEntry + Serialize + Clone>(
    window: ExportWindow,
    entries: &[T],
) -> Result<String, RepoDataError> {
    let mut sorted: Vec<T> = entries.to_vec();
    sorted.sort_by_key(ObservedEntry::key);
    for e in &sorted {
        e.validate()?;
    }
    let file = DataFile {
        schema_version: SCHEMA_VERSION,
        kind: T::KIND.to_owned(),
        attribution: ATTRIBUTION.to_owned(),
        observed_from: Some(format_date(window.observed_from)),
        observed_to: Some(format_date(window.observed_to)),
        expires_at: Some(format_date(
            window.observed_to + Duration::days(MAX_VALIDITY_DAYS),
        )),
        entries: sorted,
    };
    let mut out =
        serde_json::to_string_pretty(&file).map_err(|e| RepoDataError::Json(e.to_string()))?;
    out.push('\n');
    Ok(out)
}

/// Every object key (any depth) of `json` containing a [`FORBIDDEN_KEY_FRAGMENTS`] fragment.
pub fn find_forbidden_keys(json: &str) -> Result<Vec<String>, RepoDataError> {
    fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    let lower = k.to_lowercase();
                    if FORBIDDEN_KEY_FRAGMENTS.iter().any(|f| lower.contains(f)) {
                        out.push(k.clone());
                    }
                    walk(v, out);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| RepoDataError::Json(e.to_string()))?;
    let mut out = Vec::new();
    walk(&v, &mut out);
    out.sort();
    out.dedup();
    Ok(out)
}
