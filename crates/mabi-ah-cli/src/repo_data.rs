//! Repository metadata (`data/`) embedded into the binary at build time.
//!
//! Facts are hand-written (`data/facts/*.json`, source `repo_fact`). Observed
//! catalog files (`data/observed/*.json`, source `repo_observed`) are listed in
//! [`OBSERVED_FILES`]; each file is ONE line there, its kind is read from the
//! file itself. Expired observed files (past `expires_at`) are ignored.
use mabi_ah_app::error::AppError;
use mabi_ah_domain::repo_data::{
    DataFile, EnchantEntry, EnchantRankFact, EnchantSlotFact, EquipmentTypeFact, OptionFamilyFact,
    OptionValueEntry, ReforgeEntry, load_facts, load_observed,
};
use serde::de::DeserializeOwned;
use time::Date;

const FACT_OPTION_FAMILIES: &str = include_str!("../../../data/facts/option_families.json");
const FACT_ENCHANT_SLOTS: &str = include_str!("../../../data/facts/enchant_slots.json");
const FACT_ENCHANT_RANKS: &str = include_str!("../../../data/facts/enchant_ranks.json");
const FACT_EQUIPMENT_TYPES: &str = include_str!("../../../data/facts/equipment_types.json");

/// Observed files written by `catalog export` into `data/observed/`.
/// To embed a new file add one line, e.g.
/// `include_str!("../../../data/observed/enchants.json"),`
pub const OBSERVED_FILES: &[&str] = &[];

/// Embedded repo data, parsed once per run.
#[derive(Debug, Clone, Default)]
pub struct RepoData {
    pub option_families: Vec<OptionFamilyFact>,
    pub enchant_slots: Vec<EnchantSlotFact>,
    pub enchant_ranks: Vec<EnchantRankFact>,
    pub equipment_types: Vec<EquipmentTypeFact>,
    pub enchants: Vec<EnchantEntry>,
    pub reforges: Vec<ReforgeEntry>,
    pub option_values: Vec<OptionValueEntry>,
}

fn bad(e: impl std::fmt::Display) -> AppError {
    AppError::Internal(format!("embedded repo data is invalid: {e}"))
}

fn facts<T: mabi_ah_domain::repo_data::DataEntry + DeserializeOwned>(
    json: &str,
) -> Result<Vec<T>, AppError> {
    Ok(load_facts::<T>(json).map_err(bad)?.entries)
}

fn fresh<T>(file: DataFile<T>, today: Date) -> Vec<T> {
    if file.is_expired(today) {
        Vec::new()
    } else {
        file.entries
    }
}

impl RepoData {
    /// Facts plus the observed files that are not expired on `today`.
    pub fn load(today: Date) -> Result<Self, AppError> {
        Self::load_from(OBSERVED_FILES, today)
    }

    pub fn load_from(observed: &[&str], today: Date) -> Result<Self, AppError> {
        let mut d = Self {
            option_families: facts(FACT_OPTION_FAMILIES)?,
            enchant_slots: facts(FACT_ENCHANT_SLOTS)?,
            enchant_ranks: facts(FACT_ENCHANT_RANKS)?,
            equipment_types: facts(FACT_EQUIPMENT_TYPES)?,
            ..Self::default()
        };
        for json in observed {
            let kind = serde_json::from_str::<serde_json::Value>(json)
                .map_err(bad)?
                .get("kind")
                .and_then(|k| k.as_str().map(str::to_owned))
                .unwrap_or_default();
            match kind.as_str() {
                "observed_enchant" => d
                    .enchants
                    .extend(fresh(load_observed(json).map_err(bad)?, today)),
                "observed_reforge" => d
                    .reforges
                    .extend(fresh(load_observed(json).map_err(bad)?, today)),
                "observed_option_value" => d
                    .option_values
                    .extend(fresh(load_observed(json).map_err(bad)?, today)),
                other => return Err(bad(format!("unknown observed kind '{other}'"))),
            }
        }
        Ok(d)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn today() -> Date {
        Date::from_calendar_date(2026, time::Month::November, 11).unwrap()
    }

    #[test]
    fn embedded_facts_and_observed_files_load() {
        let d = RepoData::load(today()).unwrap();
        assert!(
            d.option_families
                .iter()
                .any(|f| f.option_type == "세공 옵션")
        );
        assert!(!d.enchant_ranks.is_empty());
        assert!(!d.enchant_slots.is_empty());
        assert!(!d.equipment_types.is_empty());
    }

    fn observed(to: &str, exp: &str) -> String {
        serde_json::json!({
            "schema_version": 1, "kind": "observed_enchant",
            "attribution": "Data based on NEXON Open API",
            "observed_from": "2026-09-25", "observed_to": to, "expires_at": exp,
            "entries": [{"slot": "접두", "name": "날카로운", "rank": "E", "effects": [],
                         "observed_count": 3, "first_seen": "2026-09-25", "last_seen": to}]
        })
        .to_string()
    }

    #[test]
    fn observed_files_are_dispatched_by_kind_and_expired_ones_skipped() {
        let ok = observed("2026-11-10", "2026-12-10");
        let d = RepoData::load_from(&[ok.as_str()], today()).unwrap();
        assert_eq!(d.enchants.len(), 1);
        let old = observed("2026-10-01", "2026-10-31");
        let d = RepoData::load_from(&[old.as_str()], today()).unwrap();
        assert!(d.enchants.is_empty());
    }
}
