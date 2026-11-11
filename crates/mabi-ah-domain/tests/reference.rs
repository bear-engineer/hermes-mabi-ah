#![allow(clippy::unwrap_used, clippy::expect_used)]
//! v11: Labanyu reference files (data/reference/labanyu) load strictly, carry their
//! source tag, never expire, and rank between repo_observed and repo_fact on ties.
use mabi_ah_domain::repo_data::{
    AuctionCategoryRef, CatalogSource, DataEntry, EchostoneAbilityRef, EnchantEntry,
    EnchantScrollRef, LABANYU_ATTRIBUTION, LABANYU_SOURCE, MuriasRelicEffectRef,
    ReforgeEquipmentTypeRef, ReforgeLevelRangeRef, ReforgeOptionNameRef, SetBonusNameRef, Sourced,
    TotemAbilityRef, find_forbidden_keys, load_reference, merge,
};
use serde::de::DeserializeOwned;

const CATEGORIES: &str =
    include_str!("../../../data/reference/labanyu/auction_item_categories.json");
const EQUIP: &str = include_str!("../../../data/reference/labanyu/reforge_equipment_types.json");
const OPTIONS: &str = include_str!("../../../data/reference/labanyu/reforge_option_names.json");
const RANGES: &str = include_str!("../../../data/reference/labanyu/reforge_level_ranges.json");
const TOTEMS: &str = include_str!("../../../data/reference/labanyu/totem_abilities.json");
const ECHO: &str =
    include_str!("../../../data/reference/labanyu/echostone_awakening_abilities.json");
const MURIAS: &str = include_str!("../../../data/reference/labanyu/murias_relic_effects.json");
const SETS: &str = include_str!("../../../data/reference/labanyu/set_bonus_names.json");
const ENCHANTS: &str = include_str!("../../../data/reference/labanyu/enchant_scrolls.json");

const ALL: [&str; 9] = [
    CATEGORIES, EQUIP, OPTIONS, RANGES, TOTEMS, ECHO, MURIAS, SETS, ENCHANTS,
];

fn count<T: DataEntry + DeserializeOwned>(json: &str) -> usize {
    let f = load_reference::<T>(json).unwrap();
    assert_eq!(f.source, LABANYU_SOURCE);
    assert_eq!(f.attribution, LABANYU_ATTRIBUTION);
    assert_eq!(f.kind, T::KIND);
    assert!(f.source_url.starts_with("https://mabi.labanyu.com/"));
    assert_eq!(f.fetched_date().len(), 10);
    f.entries.len()
}

#[test]
fn every_reference_file_loads_with_expected_counts() {
    assert_eq!(count::<AuctionCategoryRef>(CATEGORIES), 84);
    assert_eq!(count::<ReforgeEquipmentTypeRef>(EQUIP), 60);
    assert_eq!(count::<ReforgeOptionNameRef>(OPTIONS), 527);
    assert_eq!(count::<ReforgeLevelRangeRef>(RANGES), 2805);
    assert_eq!(count::<TotemAbilityRef>(TOTEMS), 61);
    assert_eq!(count::<EchostoneAbilityRef>(ECHO), 580);
    assert_eq!(count::<MuriasRelicEffectRef>(MURIAS), 30);
    assert_eq!(count::<SetBonusNameRef>(SETS), 86);
    assert_eq!(count::<EnchantScrollRef>(ENCHANTS), 1185);
}

#[test]
fn reference_files_hold_no_forbidden_keys_and_no_duplicated_ranges() {
    for json in ALL {
        assert_eq!(find_forbidden_keys(json).unwrap(), Vec::<String>::new());
    }
    // SOT: ranges live only in reforge_level_ranges; rank order only in EnchantRank.
    assert!(!OPTIONS.contains("by_equipment_type"));
    assert!(!ENCHANTS.contains("rank_order"));
    assert!(!ENCHANTS.contains("rank_label"));
    assert!(!ENCHANTS.contains("slot_en"));
    assert!(!ENCHANTS.contains("labanyu_id"));
}

#[test]
fn reforge_range_carries_limit_break_cap() {
    let f = load_reference::<ReforgeLevelRangeRef>(RANGES).unwrap();
    let r = f
        .entries
        .iter()
        .find(|r| r.equipment_type == "한손 검" && r.option_name == "최대 공격력")
        .expect("한손 검 최대 공격력 range");
    assert!(r.min_level <= r.max_level);
    assert!(r.limit_break_max_level.is_some_and(|l| l >= r.max_level));
}

fn file(kind: &str, source: &str, entry: serde_json::Value) -> String {
    serde_json::json!({
        "schema_version": 1, "kind": kind, "source": source,
        "source_url": "https://mabi.labanyu.com/in-game/auction",
        "fetched_at": "2026-11-11T02:48:39Z", "attribution": "mabi.labanyu.com",
        "notes": "test", "entries": [entry]
    })
    .to_string()
}

#[test]
fn reference_loader_is_strict() {
    let ok = file(
        "set_bonus_names",
        "labanyu_reference",
        serde_json::json!({"name": "a"}),
    );
    assert!(load_reference::<SetBonusNameRef>(&ok).is_ok());
    let price = file(
        "set_bonus_names",
        "labanyu_reference",
        serde_json::json!({"name": "a", "min_price": 1}),
    );
    assert!(load_reference::<SetBonusNameRef>(&price).is_err());
    let wrong_source = file(
        "set_bonus_names",
        "repo_fact",
        serde_json::json!({"name": "a"}),
    );
    assert!(load_reference::<SetBonusNameRef>(&wrong_source).is_err());
    let wrong_kind = file(
        "totem_abilities",
        "labanyu_reference",
        serde_json::json!({"name": "a"}),
    );
    assert!(load_reference::<SetBonusNameRef>(&wrong_kind).is_err());
    let bad_rank = file(
        "enchant_scrolls",
        "labanyu_reference",
        serde_json::json!({"name": "a", "slot": "접두", "rank": "Z"}),
    );
    assert!(load_reference::<EnchantScrollRef>(&bad_rank).is_err());
    let mut v: serde_json::Value = serde_json::from_str(&ok).unwrap();
    v["fetched_at"] = serde_json::json!("yesterday");
    assert!(load_reference::<SetBonusNameRef>(&v.to_string()).is_err());
}

#[test]
fn enchant_scroll_becomes_catalog_enchant() {
    let f = load_reference::<EnchantScrollRef>(ENCHANTS).unwrap();
    let s = f
        .entries
        .iter()
        .find(|e| e.name == "날카로운" && e.rank == "E")
        .unwrap();
    let e = s.to_enchant_entry(f.fetched_date());
    assert_eq!(e.slot, "접두");
    assert_eq!(e.effects, s.effect_lines);
    assert_eq!(e.observed_count, 0);
    assert_eq!(e.first_seen, "2026-11-11");
    assert_eq!(e.last_seen, "2026-11-11");
    assert!(e.validate().is_ok());
}

fn sourced(source: CatalogSource, name: &str) -> Sourced<EnchantEntry> {
    Sourced {
        source,
        entry: EnchantEntry {
            slot: "접두".into(),
            name: name.into(),
            rank: "E".into(),
            effects: vec![],
            observed_count: 0,
            first_seen: "2026-11-11".into(),
            last_seen: "2026-11-11".into(),
        },
    }
}

#[test]
fn labanyu_reference_ranks_between_repo_observed_and_repo_fact() {
    use CatalogSource::*;
    let order = [
        RepoFact,
        LabanyuReference,
        RepoObserved,
        LocalObserved,
        UserImport,
    ];
    for w in order.windows(2) {
        assert!(w[0].priority() < w[1].priority(), "{w:?}");
    }
    assert_eq!(LabanyuReference.as_str(), "labanyu_reference");
    assert_eq!(
        "labanyu_reference".parse::<CatalogSource>().unwrap(),
        LabanyuReference
    );
    let got = merge([
        sourced(RepoFact, "a"),
        sourced(LabanyuReference, "a"),
        sourced(LabanyuReference, "b"),
        sourced(RepoObserved, "b"),
    ]);
    let got: Vec<_> = got
        .iter()
        .map(|s| (s.entry.name.as_str(), s.source))
        .collect();
    assert_eq!(got, vec![("a", LabanyuReference), ("b", RepoObserved)]);
}
