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
        todo!()
    }

    /// Tie-break rank: user_import > local_observed > repo_observed > repo_fact.
    pub fn priority(self) -> u8 {
        todo!()
    }
}

impl FromStr for CatalogSource {
    type Err = RepoDataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
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
        todo!()
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
    todo!()
}

/// Format a date as `YYYY-MM-DD`.
pub fn format_date(d: Date) -> String {
    todo!()
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
        todo!()
    }
}

impl ObservedEntry for EnchantEntry {
    fn key(&self) -> String {
        todo!()
    }
    fn last_seen(&self) -> &str {
        todo!()
    }
}

impl DataEntry for ReforgeEntry {
    const KIND: &'static str = "observed_reforge";
    fn validate(&self) -> Result<(), RepoDataError> {
        todo!()
    }
}

impl ObservedEntry for ReforgeEntry {
    fn key(&self) -> String {
        todo!()
    }
    fn last_seen(&self) -> &str {
        todo!()
    }
}

impl DataEntry for OptionValueEntry {
    const KIND: &'static str = "observed_option_value";
    fn validate(&self) -> Result<(), RepoDataError> {
        todo!()
    }
}

impl ObservedEntry for OptionValueEntry {
    fn key(&self) -> String {
        todo!()
    }
    fn last_seen(&self) -> &str {
        todo!()
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
        todo!()
    }
}

impl DataEntry for EnchantRankFact {
    const KIND: &'static str = "enchant_rank";
    fn validate(&self) -> Result<(), RepoDataError> {
        todo!()
    }
}

impl DataEntry for OptionFamilyFact {
    const KIND: &'static str = "option_family";
    fn validate(&self) -> Result<(), RepoDataError> {
        todo!()
    }
}

impl DataEntry for EquipmentTypeFact {
    const KIND: &'static str = "equipment_type";
    fn validate(&self) -> Result<(), RepoDataError> {
        todo!()
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
    todo!()
}

/// Load an observed file: all three dates required, `observed_from <= observed_to <
/// expires_at <= observed_to + 30 days`.
pub fn load_observed<T: ObservedEntry + DeserializeOwned>(
    json: &str,
) -> Result<DataFile<T>, RepoDataError> {
    todo!()
}

// ---------- merge ----------

/// Merge entries from several sources: per key the newest `last_seen` wins; on a tie the
/// higher [`CatalogSource::priority`]. Output is sorted by key.
pub fn merge<T: ObservedEntry>(items: impl IntoIterator<Item = Sourced<T>>) -> Vec<Sourced<T>> {
    todo!()
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
    todo!()
}

/// Every object key (any depth) of `json` containing a [`FORBIDDEN_KEY_FRAGMENTS`] fragment.
pub fn find_forbidden_keys(json: &str) -> Result<Vec<String>, RepoDataError> {
    todo!()
}
