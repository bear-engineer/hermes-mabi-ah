#![allow(clippy::unwrap_used)]

use mabi_ah_domain::attribution::ATTRIBUTION;
use mabi_ah_domain::catalog::{CatalogObservation as Obs, extract};
use mabi_ah_domain::category::SPEC_CATEGORIES;
use mabi_ah_domain::option::enchant_rank::EnchantRank;
use mabi_ah_domain::option::{ItemOption, OptionRegistry};
use mabi_ah_domain::repo_data::{
    CatalogSource, DataFile, EnchantEntry, EnchantRankFact, EnchantSlotFact, EquipmentTypeFact,
    ExportWindow, OptionFamilyFact, OptionValueEntry, ReforgeEntry, RepoDataError, Sourced,
    find_forbidden_keys, load_facts, load_observed, merge, parse_date, to_observed_json,
};
use time::macros::date;

fn opt(
    ty: &str,
    sub: Option<&str>,
    v: Option<&str>,
    v2: Option<&str>,
    d: Option<&str>,
) -> ItemOption {
    ItemOption {
        option_type: ty.into(),
        sub_type: sub.map(Into::into),
        value: v.map(Into::into),
        value2: v2.map(Into::into),
        desc: d.map(Into::into),
    }
}

fn ex(cat: &str, o: Vec<ItemOption>) -> Vec<Obs> {
    extract(&OptionRegistry::default(), cat, &o)
}

#[test]
fn enchant_with_letter_rank_and_effects() {
    let got = ex(
        "장갑",
        vec![opt(
            "인챈트",
            Some("접두"),
            Some("날카로운 (랭크 E)"),
            None,
            Some("크리티컬 2 증가,최대대미지 3 감소"),
        )],
    );
    assert_eq!(
        got,
        vec![Obs::Enchant {
            slot: "접두".into(),
            name: "날카로운".into(),
            rank: EnchantRank::E,
            effects: vec!["크리티컬 2 증가".into(), "최대대미지 3 감소".into()],
        }]
    );
}

#[test]
fn enchant_scroll_digit_rank_has_no_effects() {
    let got = ex(
        "인챈트 스크롤",
        vec![opt(
            "인챈트 종류",
            Some("접미"),
            Some("스텔라 (랭크 4)"),
            None,
            None,
        )],
    );
    assert_eq!(
        got,
        vec![Obs::Enchant {
            slot: "접미".into(),
            name: "스텔라".into(),
            rank: EnchantRank::N4,
            effects: vec![],
        }]
    );
}

#[test]
fn reforge_both_value_formats_keep_slot_and_category() {
    let got = ex(
        "검",
        vec![
            opt(
                "세공 옵션",
                Some("1"),
                Some("최소 공격력 11 레벨"),
                None,
                None,
            ),
            opt(
                "세공 옵션",
                Some("3"),
                Some("듀얼건 최대 대미지(20레벨:100 증가)"),
                None,
                None,
            ),
        ],
    );
    assert_eq!(
        got,
        vec![
            Obs::Reforge {
                option: "최소 공격력".into(),
                level: 11,
                slot: "1".into(),
                category: "검".into()
            },
            Obs::Reforge {
                option: "듀얼건 최대 대미지".into(),
                level: 20,
                slot: "3".into(),
                category: "검".into()
            },
        ]
    );
}

#[test]
fn reforge_without_slot_is_kept_raw() {
    let o = opt("세공 옵션", None, Some("최소 공격력 11 레벨"), None, None);
    assert_eq!(
        ex("검", vec![o.clone()]),
        vec![Obs::Unparsed {
            category: "검".into(),
            raw: o
        }]
    );
}

#[test]
fn reforge_rank_is_never_interpreted() {
    let o = opt("세공 랭크", None, Some("3"), None, None);
    assert_eq!(
        ex("검", vec![o.clone()]),
        vec![Obs::Unparsed {
            category: "검".into(),
            raw: o
        }]
    );
}

#[test]
fn unreadable_and_unknown_options_keep_raw() {
    let bad = opt("인챈트", Some("접두"), Some("이상한 값"), None, None);
    let unk = opt("새로운 옵션", None, Some("x"), None, Some("d"));
    let got = ex("검", vec![bad.clone(), unk.clone()]);
    assert_eq!(
        got,
        vec![
            Obs::Unparsed {
                category: "검".into(),
                raw: bad
            },
            Obs::Unparsed {
                category: "검".into(),
                raw: unk
            },
        ]
    );
}

#[test]
fn duplicate_options_are_reported_once() {
    let o = opt(
        "인챈트 종류",
        Some("접두"),
        Some("스텔라 (랭크 4)"),
        None,
        None,
    );
    assert_eq!(ex("인챈트 스크롤", vec![o.clone(), o]).len(), 1);
}

#[test]
fn option_values_use_sub_type_name_and_level() {
    let got = ex(
        "장갑",
        vec![
            opt("에르그", Some("S"), Some("40"), Some("50"), None),
            opt(
                "에코스톤 각성 능력",
                None,
                Some("테스트효과 6 레벨"),
                None,
                None,
            ),
            opt("세트 효과", None, Some("테스트 세트"), None, None),
        ],
    );
    assert_eq!(
        got,
        vec![
            Obs::OptionValue {
                option_type: "에르그".into(),
                sub_type: Some("S".into()),
                value: "S".into(),
                level: Some(40),
                category: "장갑".into()
            },
            Obs::OptionValue {
                option_type: "에코스톤 각성 능력".into(),
                sub_type: None,
                value: "테스트효과".into(),
                level: Some(6),
                category: "장갑".into()
            },
            Obs::OptionValue {
                option_type: "세트 효과".into(),
                sub_type: None,
                value: "테스트 세트".into(),
                level: None,
                category: "장갑".into()
            },
        ]
    );
}

#[test]
fn volatile_options_yield_nothing() {
    let got = ex(
        "장갑",
        vec![
            opt("색상", None, Some("1,2,3"), None, None),
            opt("공격", None, Some("10"), Some("20"), None),
            opt("인챈트 불가능", None, Some("true"), None, None),
        ],
    );
    assert!(got.is_empty(), "{got:?}");
}

// ---------- repo data ----------

const OBS_ENCHANT: &str = r#"{
  "schema_version": 1, "kind": "observed_enchant", "attribution": "Data based on NEXON Open API",
  "observed_from": "2026-11-01", "observed_to": "2026-11-10", "expires_at": "2026-12-10",
  "entries": [
    {"slot":"접두","name":"날카로운","rank":"E","effects":["크리티컬 2 증가"],
     "observed_count":3,"first_seen":"2026-11-02","last_seen":"2026-11-09"}
  ]}"#;

fn enchant(name: &str, last: &str) -> EnchantEntry {
    EnchantEntry {
        slot: "접두".into(),
        name: name.into(),
        rank: "E".into(),
        effects: vec![],
        observed_count: 1,
        first_seen: "2026-11-01".into(),
        last_seen: last.into(),
    }
}

#[test]
fn observed_file_loads() {
    let f = load_observed::<EnchantEntry>(OBS_ENCHANT).unwrap();
    assert_eq!(f.entries.len(), 1);
    assert_eq!(f.entries[0].name, "날카로운");
    assert!(!f.is_expired(date!(2026 - 12 - 10)));
    assert!(f.is_expired(date!(2026 - 12 - 11)));
}

#[test]
fn observed_file_rejections() {
    let swap = |from: &str, to: &str| OBS_ENCHANT.replace(from, to);
    let e = |s: String| load_observed::<EnchantEntry>(&s).unwrap_err();
    assert!(matches!(
        e(swap("\"schema_version\": 1", "\"schema_version\": 2")),
        RepoDataError::SchemaVersion(2)
    ));
    assert!(matches!(
        e(swap("observed_enchant", "observed_reforge")),
        RepoDataError::Kind { .. }
    ));
    assert!(matches!(
        e(swap("\"expires_at\": \"2026-12-10\",", "")),
        RepoDataError::Missing("expires_at")
    ));
    assert!(matches!(
        e(swap("2026-12-10", "2026-12-11")),
        RepoDataError::Window(_)
    ));
    assert!(matches!(
        e(swap("\"rank\":\"E\"", "\"rank\":\"Z\"")),
        RepoDataError::Entry(_)
    ));
    assert!(matches!(
        e(swap("\"observed_count\"", "\"price\":5,\"observed_count\"")),
        RepoDataError::Json(_)
    ));
    assert!(matches!(
        e(swap("2026-11-09", "bad")),
        RepoDataError::Date(_)
    ));
}

#[test]
fn merge_newest_observation_wins_then_source_priority() {
    let s = |source, name: &str, last: &str| Sourced {
        source,
        entry: enchant(name, last),
    };
    let mut old = s(CatalogSource::UserImport, "a", "2026-11-01");
    old.entry.observed_count = 9;
    let merged = merge(vec![
        old,
        s(CatalogSource::RepoObserved, "a", "2026-11-05"),
        s(CatalogSource::RepoObserved, "b", "2026-11-05"),
        s(CatalogSource::LocalObserved, "b", "2026-11-05"),
        s(CatalogSource::RepoFact, "b", "2026-11-05"),
    ]);
    let got: Vec<_> = merged
        .iter()
        .map(|m| (m.entry.name.as_str(), m.source))
        .collect();
    assert_eq!(
        got,
        vec![
            ("a", CatalogSource::RepoObserved),
            ("b", CatalogSource::LocalObserved)
        ]
    );
    assert_eq!(CatalogSource::UserImport.as_str(), "user_import");
    assert_eq!(
        "repo_fact".parse::<CatalogSource>().unwrap(),
        CatalogSource::RepoFact
    );
}

#[test]
fn export_is_sorted_deterministic_whitelisted_and_roundtrips() {
    let win = ExportWindow {
        observed_from: date!(2026 - 11 - 01),
        observed_to: date!(2026 - 11 - 10),
    };
    let a = to_observed_json(
        win,
        &[enchant("나", "2026-11-05"), enchant("가", "2026-11-05")],
    )
    .unwrap();
    let b = to_observed_json(
        win,
        &[enchant("가", "2026-11-05"), enchant("나", "2026-11-05")],
    )
    .unwrap();
    assert_eq!(a, b);
    assert!(a.ends_with('\n'));
    assert!(a.find("\"가\"").unwrap() < a.find("\"나\"").unwrap());
    assert!(a.contains("\"expires_at\": \"2026-12-10\""));
    assert!(a.contains(ATTRIBUTION));
    assert!(find_forbidden_keys(&a).unwrap().is_empty());
    let back = load_observed::<EnchantEntry>(&a).unwrap();
    assert_eq!(back.entries.len(), 2);

    let rf = ReforgeEntry {
        option: "최소 공격력".into(),
        slot: "1".into(),
        category: "검".into(),
        min_level: 3,
        max_level: 20,
        observed_count: 4,
        first_seen: "2026-11-01".into(),
        last_seen: "2026-11-09".into(),
    };
    let ov = OptionValueEntry {
        option_type: "에르그".into(),
        sub_type: Some("S".into()),
        value: "S".into(),
        category: "장갑".into(),
        min_level: Some(1),
        max_level: Some(50),
        observed_count: 2,
        first_seen: "2026-11-01".into(),
        last_seen: "2026-11-09".into(),
    };
    for json in [
        to_observed_json(win, &[rf]).unwrap(),
        to_observed_json(win, &[ov]).unwrap(),
    ] {
        assert!(find_forbidden_keys(&json).unwrap().is_empty());
    }
}

#[test]
fn forbidden_keys_are_found_at_any_depth() {
    let json = r#"{"entries":[{"unit_price":1,"x":{"Seller_Name":"a","character":"b"}}]}"#;
    assert_eq!(
        find_forbidden_keys(json).unwrap(),
        vec![
            "Seller_Name".to_owned(),
            "character".to_owned(),
            "unit_price".to_owned()
        ]
    );
    assert!(parse_date("2026-02-30").is_err());
}

fn facts(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../data/facts/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn facts_enchant_slots_and_rank_order_match_the_domain_type() {
    let slots: DataFile<EnchantSlotFact> = load_facts(&facts("enchant_slots.json")).unwrap();
    let names: Vec<_> = slots.entries.iter().map(|s| s.slot.as_str()).collect();
    assert_eq!(names, vec!["접두", "접미"]);

    let ranks: DataFile<EnchantRankFact> = load_facts(&facts("enchant_ranks.json")).unwrap();
    let got: Vec<_> = ranks
        .entries
        .iter()
        .map(|r| (r.rank.clone(), r.order))
        .collect();
    let want: Vec<_> = EnchantRank::ALL
        .iter()
        .map(|r| (r.to_string(), r.ordinal()))
        .collect();
    assert_eq!(got, want);
}

#[test]
fn facts_equipment_types_are_spec_categories() {
    let f: DataFile<EquipmentTypeFact> = load_facts(&facts("equipment_types.json")).unwrap();
    assert!(f.entries.len() >= 20);
    for e in &f.entries {
        assert!(
            SPEC_CATEGORIES.contains(&e.category.as_str()),
            "{}",
            e.category
        );
    }
}

#[test]
fn facts_option_families_are_registry_types_and_mark_unobserved() {
    let f: DataFile<OptionFamilyFact> = load_facts(&facts("option_families.json")).unwrap();
    let registry = OptionRegistry::default().option_types();
    for e in &f.entries {
        assert!(
            registry.contains(&e.option_type.as_str()),
            "{}",
            e.option_type
        );
    }
    let rank = f
        .entries
        .iter()
        .find(|e| e.option_type == "세공 랭크")
        .unwrap();
    assert_eq!(rank.status, "unobserved");
    assert!(
        f.entries
            .iter()
            .any(|e| e.option_type == "세공 옵션" && e.status == "observed")
    );
    assert!(
        find_forbidden_keys(&facts("option_families.json"))
            .unwrap()
            .is_empty()
    );
}
