//! Repository metadata (`data/`) embedded into the binary at build time.
//!
//! Facts are hand-written (`data/facts/*.json`, source `repo_fact`). Observed
//! catalog files (`data/observed/*.json`, source `repo_observed`) are listed in
//! [`OBSERVED_FILES`]; each file is ONE line there, its kind is read from the
//! file itself. Expired observed files (past `expires_at`) are ignored.
//!
//! Labanyu reference files (`data/reference/labanyu/<kind>.json`, source
//! `labanyu_reference`) are listed in [`REFERENCE_FILES`], one line each; they have
//! `fetched_at` and never expire. Removing a line (takedown) leaves the CLI working
//! on observed data and facts only.
use mabi_ah_app::error::AppError;
use mabi_ah_domain::repo_data::{
    AuctionCategoryRef, DataEntry, DataFile, EchostoneAbilityRef, EnchantEntry, EnchantRankFact,
    EnchantScrollRef, EnchantSlotFact, EquipmentTypeFact, MuriasRelicEffectRef, OptionFamilyFact,
    OptionValueEntry, ReferenceFile, ReforgeEntry, ReforgeEquipmentTypeRef, ReforgeLevelRangeRef,
    ReforgeOptionNameRef, SetBonusNameRef, TotemAbilityRef, load_facts, load_observed,
    load_reference,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use time::Date;

const FACT_OPTION_FAMILIES: &str = include_str!("../../../data/facts/option_families.json");
const FACT_ENCHANT_SLOTS: &str = include_str!("../../../data/facts/enchant_slots.json");
const FACT_ENCHANT_RANKS: &str = include_str!("../../../data/facts/enchant_ranks.json");
const FACT_EQUIPMENT_TYPES: &str = include_str!("../../../data/facts/equipment_types.json");

/// Observed files written by `catalog export` into `data/observed/`.
/// To embed a new file add one line, e.g.
/// `include_str!("../../../data/observed/enchants.json"),`
pub const OBSERVED_FILES: &[&str] = &[];

/// Labanyu reference files (`scripts/labanyu/build.py` output); kind read from the file.
pub const REFERENCE_FILES: &[&str] = &[
    include_str!("../../../data/reference/labanyu/auction_item_categories.json"),
    include_str!("../../../data/reference/labanyu/reforge_equipment_types.json"),
    include_str!("../../../data/reference/labanyu/reforge_option_names.json"),
    include_str!("../../../data/reference/labanyu/reforge_level_ranges.json"),
    include_str!("../../../data/reference/labanyu/totem_abilities.json"),
    include_str!("../../../data/reference/labanyu/echostone_awakening_abilities.json"),
    include_str!("../../../data/reference/labanyu/murias_relic_effects.json"),
    include_str!("../../../data/reference/labanyu/set_bonus_names.json"),
    include_str!("../../../data/reference/labanyu/enchant_scrolls.json"),
];

/// Kinds listed by `catalog reference --kind` (enchants and ranges have their own commands).
pub const LIST_KINDS: &[&str] = &[
    AuctionCategoryRef::KIND,
    ReforgeEquipmentTypeRef::KIND,
    ReforgeOptionNameRef::KIND,
    TotemAbilityRef::KIND,
    EchostoneAbilityRef::KIND,
    MuriasRelicEffectRef::KIND,
    SetBonusNameRef::KIND,
];

/// Embedded Labanyu reference data; a kind whose file is not embedded is `None`.
#[derive(Debug, Clone, Default)]
pub struct Reference {
    pub categories: Option<ReferenceFile<AuctionCategoryRef>>,
    pub equipment_types: Option<ReferenceFile<ReforgeEquipmentTypeRef>>,
    pub option_names: Option<ReferenceFile<ReforgeOptionNameRef>>,
    pub ranges: Option<ReferenceFile<ReforgeLevelRangeRef>>,
    pub totems: Option<ReferenceFile<TotemAbilityRef>>,
    pub echostones: Option<ReferenceFile<EchostoneAbilityRef>>,
    pub murias: Option<ReferenceFile<MuriasRelicEffectRef>>,
    pub set_bonuses: Option<ReferenceFile<SetBonusNameRef>>,
    pub enchants: Option<ReferenceFile<EnchantScrollRef>>,
}

/// One listed reference kind: file envelope (without entries) and entry rows.
pub struct ReferenceRows {
    pub kind: &'static str,
    pub source_url: String,
    pub fetched_at: String,
    pub attribution: String,
    pub notes: String,
    /// `(name, entry JSON)`.
    pub rows: Vec<(String, Value)>,
}

fn reference_rows<T: Serialize>(
    kind: &'static str,
    f: &ReferenceFile<T>,
    name: impl Fn(&T) -> &str,
) -> ReferenceRows {
    ReferenceRows {
        kind,
        source_url: f.source_url.clone(),
        fetched_at: f.fetched_at.clone(),
        attribution: f.attribution.clone(),
        notes: f.notes.clone(),
        rows: f
            .entries
            .iter()
            .map(|e| {
                (
                    name(e).to_owned(),
                    serde_json::to_value(e).unwrap_or(Value::Null),
                )
            })
            .collect(),
    }
}

impl Reference {
    pub fn load() -> Result<Self, AppError> {
        Self::load_from(REFERENCE_FILES)
    }

    pub fn load_from(files: &[&str]) -> Result<Self, AppError> {
        fn put<T: DataEntry + serde::de::DeserializeOwned>(
            slot: &mut Option<ReferenceFile<T>>,
            json: &str,
        ) -> Result<(), AppError> {
            *slot = Some(load_reference(json).map_err(bad)?);
            Ok(())
        }
        let mut r = Self::default();
        for json in files {
            match kind_of(json)?.as_str() {
                AuctionCategoryRef::KIND => put(&mut r.categories, json)?,
                ReforgeEquipmentTypeRef::KIND => put(&mut r.equipment_types, json)?,
                ReforgeOptionNameRef::KIND => put(&mut r.option_names, json)?,
                ReforgeLevelRangeRef::KIND => put(&mut r.ranges, json)?,
                TotemAbilityRef::KIND => put(&mut r.totems, json)?,
                EchostoneAbilityRef::KIND => put(&mut r.echostones, json)?,
                MuriasRelicEffectRef::KIND => put(&mut r.murias, json)?,
                SetBonusNameRef::KIND => put(&mut r.set_bonuses, json)?,
                EnchantScrollRef::KIND => put(&mut r.enchants, json)?,
                other => return Err(bad(format!("unknown reference kind '{other}'"))),
            }
        }
        Ok(r)
    }

    /// Reference enchants as catalog entries (dated by `fetched_at`).
    pub fn enchant_entries(&self) -> Vec<EnchantEntry> {
        self.enchants
            .as_ref()
            .map(|f| {
                f.entries
                    .iter()
                    .map(|e| e.to_enchant_entry(f.fetched_date()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every known reforge option name (option list plus range rows), sorted, deduped.
    pub fn reforge_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .option_names
            .iter()
            .flat_map(|f| f.entries.iter().map(|e| e.name.clone()))
            .chain(
                self.ranges
                    .iter()
                    .flat_map(|f| f.entries.iter().map(|e| e.option_name.clone())),
            )
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Rows of one [`LIST_KINDS`] kind; `None` if unknown or not embedded.
    pub fn rows(&self, kind: &str) -> Option<ReferenceRows> {
        match kind {
            AuctionCategoryRef::KIND => self
                .categories
                .as_ref()
                .map(|f| reference_rows(AuctionCategoryRef::KIND, f, |e| &e.name)),
            ReforgeEquipmentTypeRef::KIND => self
                .equipment_types
                .as_ref()
                .map(|f| reference_rows(ReforgeEquipmentTypeRef::KIND, f, |e| &e.name)),
            ReforgeOptionNameRef::KIND => self
                .option_names
                .as_ref()
                .map(|f| reference_rows(ReforgeOptionNameRef::KIND, f, |e| &e.name)),
            TotemAbilityRef::KIND => self
                .totems
                .as_ref()
                .map(|f| reference_rows(TotemAbilityRef::KIND, f, |e| &e.name)),
            EchostoneAbilityRef::KIND => self
                .echostones
                .as_ref()
                .map(|f| reference_rows(EchostoneAbilityRef::KIND, f, |e| &e.name)),
            MuriasRelicEffectRef::KIND => self
                .murias
                .as_ref()
                .map(|f| reference_rows(MuriasRelicEffectRef::KIND, f, |e| &e.name)),
            SetBonusNameRef::KIND => self
                .set_bonuses
                .as_ref()
                .map(|f| reference_rows(SetBonusNameRef::KIND, f, |e| &e.name)),
            _ => None,
        }
    }
}

fn kind_of(json: &str) -> Result<String, AppError> {
    Ok(serde_json::from_str::<Value>(json)
        .map_err(bad)?
        .get("kind")
        .and_then(|k| k.as_str().map(str::to_owned))
        .unwrap_or_default())
}

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

fn facts<T: DataEntry + DeserializeOwned>(json: &str) -> Result<Vec<T>, AppError> {
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

    #[test]
    fn embedded_reference_files_load_and_list_every_kind() {
        let r = Reference::load().unwrap();
        assert!(r.reforge_names().iter().any(|n| n == "최대 공격력"));
        assert!(!r.enchant_entries().is_empty());
        for k in LIST_KINDS {
            assert!(!r.rows(k).unwrap().rows.is_empty(), "{k}");
        }
        let none = Reference::load_from(&[]).unwrap();
        assert!(none.reforge_names().is_empty() && none.rows(LIST_KINDS[0]).is_none());
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
