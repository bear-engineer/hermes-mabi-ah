#![allow(clippy::unwrap_used)]

use mabi_ah_app::error::AppError;
use mabi_ah_domain::catalog::{CatalogObservation as Obs, extract};
use mabi_ah_domain::option::enchant_rank::EnchantRank;
use mabi_ah_domain::option::{ItemOption, OptionRegistry};
use mabi_ah_domain::repo_data::CatalogSource;
use mabi_ah_store::SqliteStore;
use mabi_ah_store::catalog::CatalogQuery;
use rusqlite::Connection;
use time::OffsetDateTime;
use time::macros::{date, datetime};

const NOW: OffsetDateTime = datetime!(2026-11-11 12:00 UTC);

fn open() -> (tempfile::TempDir, SqliteStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("t.db")).unwrap();
    (dir, store)
}

fn enchant(name: &str, rank: EnchantRank, effects: &[&str]) -> Obs {
    Obs::Enchant {
        slot: "접두".into(),
        name: name.into(),
        rank,
        effects: effects.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn reforge(option: &str, level: u32, slot: &str, category: &str) -> Obs {
    Obs::Reforge {
        option: option.into(),
        level,
        slot: slot.into(),
        category: category.into(),
    }
}

fn q() -> CatalogQuery {
    CatalogQuery::default()
}

#[test]
fn migration_bumps_user_version_and_creates_tables() {
    let (dir, store) = open();
    drop(store);
    let c = Connection::open(dir.path().join("t.db")).unwrap();
    let v: i64 = c
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 2);
    for t in ["catalog_enchant", "catalog_reforge", "catalog_option_value"] {
        let n: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                [t],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "{t}");
    }
}

#[test]
fn enchant_observations_accumulate_and_keep_effects() {
    let (_d, mut s) = open();
    let scroll = enchant("날카로운", EnchantRank::E, &[]);
    let worn = enchant("날카로운", EnchantRank::E, &["크리티컬 2 증가"]);
    s.record_catalog(&[worn.clone()], datetime!(2026-11-01 00:00 UTC))
        .unwrap();
    s.record_catalog(&[scroll], datetime!(2026-11-05 00:00 UTC))
        .unwrap();
    s.record_catalog(&[worn], datetime!(2026-11-09 00:00 UTC))
        .unwrap();
    let got = s.catalog_enchants(&q()).unwrap();
    assert_eq!(got.len(), 1);
    let e = &got[0];
    assert_eq!(e.source, CatalogSource::LocalObserved);
    assert_eq!(e.entry.observed_count, 3);
    assert_eq!(e.entry.first_seen, "2026-11-01");
    assert_eq!(e.entry.last_seen, "2026-11-09");
    assert_eq!(e.entry.effects, vec!["크리티컬 2 증가".to_owned()]);
    assert_eq!(e.entry.rank, "E");
}

#[test]
fn enchant_query_by_text_slot_limit_and_rank_order() {
    let (_d, mut s) = open();
    let mut obs = vec![
        enchant("날카로운", EnchantRank::N9, &[]),
        enchant("날카로운", EnchantRank::E, &[]),
        enchant("회오리", EnchantRank::N6, &[]),
    ];
    obs.push(Obs::Enchant {
        slot: "접미".into(),
        name: "날카로운 척".into(),
        rank: EnchantRank::A,
        effects: vec![],
    });
    s.record_catalog(&obs, NOW).unwrap();
    let by_text = s
        .catalog_enchants(&CatalogQuery {
            text: Some("날카".into()),
            ..q()
        })
        .unwrap();
    let ranks: Vec<_> = by_text
        .iter()
        .map(|e| (e.entry.name.as_str(), e.entry.rank.as_str()))
        .collect();
    assert_eq!(
        ranks,
        vec![("날카로운", "E"), ("날카로운", "9"), ("날카로운 척", "A")]
    );
    let suffix = s
        .catalog_enchants(&CatalogQuery {
            slot: Some("접미".into()),
            ..q()
        })
        .unwrap();
    assert_eq!(suffix.len(), 1);
    let limited = s
        .catalog_enchants(&CatalogQuery {
            limit: Some(2),
            ..q()
        })
        .unwrap();
    assert_eq!(limited.len(), 2);
    // `%` and `_` are plain characters, not wildcards.
    assert!(
        s.catalog_enchants(&CatalogQuery {
            text: Some("%".into()),
            ..q()
        })
        .unwrap()
        .is_empty()
    );
}

#[test]
fn reforge_ranges_per_option_slot_category() {
    let (_d, mut s) = open();
    s.record_catalog(
        &[
            reforge("최소 공격력", 11, "1", "검"),
            reforge("최소 공격력", 20, "1", "검"),
            reforge("최소 공격력", 5, "2", "검"),
            reforge("최소 공격력", 7, "1", "도끼"),
        ],
        NOW,
    )
    .unwrap();
    let sword = s
        .catalog_reforges(&CatalogQuery {
            category: Some("검".into()),
            ..q()
        })
        .unwrap();
    assert_eq!(sword.len(), 2);
    let slot1 = sword.iter().find(|r| r.entry.slot == "1").unwrap();
    assert_eq!(
        (
            slot1.entry.min_level,
            slot1.entry.max_level,
            slot1.entry.observed_count
        ),
        (11, 20, 2)
    );
    let by_text = s
        .catalog_reforges(&CatalogQuery {
            text: Some("공격".into()),
            ..q()
        })
        .unwrap();
    assert_eq!(by_text.len(), 3);
}

#[test]
fn option_values_types_and_filters() {
    let (_d, mut s) = open();
    let ov = |ty: &str, sub: Option<&str>, v: &str, lv: Option<u32>, cat: &str| Obs::OptionValue {
        option_type: ty.into(),
        sub_type: sub.map(Into::into),
        value: v.into(),
        level: lv,
        category: cat.into(),
    };
    s.record_catalog(
        &[
            ov("에르그", Some("S"), "S", Some(10), "검"),
            ov("에르그", Some("S"), "S", Some(40), "검"),
            ov("세트 효과", None, "테스트 세트", None, "장갑"),
        ],
        NOW,
    )
    .unwrap();
    assert_eq!(
        s.catalog_option_types().unwrap(),
        vec![("세트 효과".to_owned(), 1), ("에르그".to_owned(), 1)]
    );
    let erg = s.catalog_option_values(Some("에르그"), &q()).unwrap();
    assert_eq!(erg.len(), 1);
    assert_eq!(
        (
            erg[0].entry.min_level,
            erg[0].entry.max_level,
            erg[0].entry.sub_type.as_deref()
        ),
        (Some(10), Some(40), Some("S"))
    );
    let set = s
        .catalog_option_values(
            None,
            &CatalogQuery {
                category: Some("장갑".into()),
                ..q()
            },
        )
        .unwrap();
    assert_eq!(set.len(), 1);
    assert_eq!(set[0].entry.min_level, None);
}

#[test]
fn unparsed_options_are_kept_raw_not_interpreted() {
    let (_d, mut s) = open();
    let rank = ItemOption {
        option_type: "세공 랭크".into(),
        sub_type: None,
        value: Some("3".into()),
        value2: None,
        desc: Some("임의 설명".into()),
    };
    let obs = extract(
        &OptionRegistry::default(),
        "검",
        std::slice::from_ref(&rank),
    );
    s.record_catalog(&obs, NOW).unwrap();
    s.record_catalog(&obs, NOW).unwrap();
    let rows = s.catalog_unparsed(None).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].raw, rank);
    assert_eq!(rows[0].category, "검");
    assert_eq!(rows[0].observed_count, 2);
    assert!(s.catalog_option_types().unwrap().is_empty());
}

#[test]
fn purge_removes_only_rows_older_than_30_days() {
    let (_d, mut s) = open();
    s.record_catalog(
        &[
            enchant("옛것", EnchantRank::F, &[]),
            reforge("옛 옵션", 1, "1", "검"),
        ],
        NOW - time::Duration::days(31),
    )
    .unwrap();
    s.record_catalog(
        &[
            enchant("새것", EnchantRank::F, &[]),
            reforge("새 옵션", 1, "1", "검"),
        ],
        NOW - time::Duration::days(29),
    )
    .unwrap();
    assert_eq!(s.purge_catalog(NOW).unwrap(), 2);
    let names: Vec<_> = s
        .catalog_enchants(&q())
        .unwrap()
        .into_iter()
        .map(|e| e.entry.name)
        .collect();
    assert_eq!(names, vec!["새것".to_owned()]);
    assert_eq!(s.catalog_reforges(&q()).unwrap().len(), 1);
    assert_eq!(s.purge_catalog(NOW).unwrap(), 0);
}

const IMPORT: &str = r#"{
  "schema_version": 1, "kind": "observed_enchant", "attribution": "Data based on NEXON Open API",
  "observed_from": "2026-11-01", "observed_to": "2026-11-10", "expires_at": "2026-12-10",
  "entries": [
    {"slot":"접두","name":"날카로운","rank":"E","effects":["크리티컬 2 증가"],
     "observed_count":5,"first_seen":"2026-11-02","last_seen":"2026-11-09"}
  ]}"#;

#[test]
fn user_import_is_idempotent_and_older_data_does_not_take_over() {
    let (_d, mut s) = open();
    let r = s
        .import_catalog_json(IMPORT, date!(2026 - 11 - 11))
        .unwrap();
    assert_eq!((r.enchants, r.reforges, r.option_values), (1, 0, 0));
    s.import_catalog_json(IMPORT, date!(2026 - 11 - 11))
        .unwrap();
    let got = s.catalog_enchants(&q()).unwrap();
    assert_eq!(got[0].source, CatalogSource::UserImport);
    assert_eq!(got[0].entry.observed_count, 5);

    // A newer local observation takes the source; a later re-import of old data does not take it back.
    s.record_catalog(
        &[enchant("날카로운", EnchantRank::E, &[])],
        datetime!(2026-11-10 09:00 UTC),
    )
    .unwrap();
    s.import_catalog_json(IMPORT, date!(2026 - 11 - 11))
        .unwrap();
    let got = s.catalog_enchants(&q()).unwrap();
    assert_eq!(got[0].source, CatalogSource::LocalObserved);
    assert_eq!(got[0].entry.last_seen, "2026-11-10");
    assert_eq!(got[0].entry.effects, vec!["크리티컬 2 증가".to_owned()]);
}

#[test]
fn import_rejects_expired_forbidden_and_unknown_files() {
    let (_d, mut s) = open();
    let usage = |r: Result<_, AppError>| matches!(r, Err(AppError::Usage(_)));
    assert!(usage(s.import_catalog_json(IMPORT, date!(2026 - 12 - 11))));
    let priced = IMPORT.replace("\"observed_count\"", "\"unit_price\":1,\"observed_count\"");
    assert!(usage(s.import_catalog_json(&priced, date!(2026 - 11 - 11))));
    assert!(usage(s.import_catalog_json(
        "{\"kind\":\"enchant_slot\"}",
        date!(2026 - 11 - 11)
    )));
    assert!(usage(
        s.import_catalog_json("not json", date!(2026 - 11 - 11))
    ));
    assert!(s.catalog_enchants(&q()).unwrap().is_empty());
}

#[test]
fn catalog_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut s = SqliteStore::open(&path).unwrap();
        s.record_catalog(&[reforge("최소 공격력", 3, "1", "검")], NOW)
            .unwrap();
    }
    let s = SqliteStore::open(&path).unwrap();
    assert_eq!(s.catalog_reforges(&q()).unwrap().len(), 1);
}
