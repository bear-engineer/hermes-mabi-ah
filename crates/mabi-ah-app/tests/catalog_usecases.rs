//! V4 use cases: catalog port, observation recording, learn, option prices,
//! suggestions, predicate filtering, catalog merge.
#![allow(clippy::unwrap_used)]

use time::macros::datetime;
use time::{Duration, OffsetDateTime};

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{AuctionGateway, CatalogQuery, CatalogStore, ListingQuery, Page};
use mabi_ah_app::usecases::catalog::{
    catalog_enchants, catalog_option_types, catalog_option_values, catalog_reforges,
    check_filter_names, learn_catalog,
};
use mabi_ah_app::usecases::option_prices::{OptionPriceRow, option_prices};
use mabi_ah_app::usecases::query::{
    ItemFilter, QueryEnv, query_listings_in, query_trades_in, search_listings_in,
};
use mabi_ah_app::usecases::suggest::suggest_option_names;
use mabi_ah_domain::catalog::CatalogObservation;
use mabi_ah_domain::category::CategoryRegistry;
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::predicate::{EnchantPred, FilterExpr, OptionPredicate, ReforgePred};
use mabi_ah_domain::option::{ItemOption, OptionFilter, OptionRegistry};
use mabi_ah_domain::repo_data::{
    CatalogSource, EnchantEntry, OptionValueEntry, ReforgeEntry, Sourced,
};
use mabi_ah_domain::trade::Trade;

const T0: OffsetDateTime = datetime!(2026-11-10 12:00 UTC);

fn o(t: &str, sub: Option<&str>, v: &str) -> ItemOption {
    ItemOption {
        option_type: t.to_owned(),
        sub_type: sub.map(str::to_owned),
        value: Some(v.to_owned()),
        value2: None,
        desc: None,
    }
}
fn ench(slot: &str, v: &str) -> ItemOption {
    o("인챈트", Some(slot), v)
}
fn refo(slot: &str, v: &str) -> ItemOption {
    o("세공 옵션", Some(slot), v)
}
fn listing(name: &str, category: &str, price: u64, options: Vec<ItemOption>) -> Listing {
    Listing {
        name: ItemName::new(name, name),
        count: 1,
        category: category.to_owned(),
        unit_price: price,
        expires_at: T0 + Duration::hours(1),
        options,
    }
}
fn trade(id: &str, options: Vec<ItemOption>) -> Trade {
    Trade {
        trade_id: id.to_owned(),
        name: ItemName::new("테스트 검", "테스트 검"),
        count: 1,
        category: "한손 검".to_owned(),
        unit_price: 100,
        traded_at: T0 - Duration::minutes(1),
        options,
    }
}

// --- fakes -----------------------------------------------------------------

#[derive(Default)]
struct FakeGw {
    listing_pages: Vec<Vec<Listing>>,
    trade_pages: Vec<Vec<Trade>>,
    search_items: Vec<Listing>,
    calls: u32,
    queries: Vec<ListingQuery>,
}
fn paginate<T: Clone>(pages: &[Vec<T>], cursor: Option<&str>) -> Page<T> {
    let i: usize = cursor.map_or(0, |c| c.parse().unwrap());
    Page {
        items: pages.get(i).cloned().unwrap_or_default(),
        next_cursor: (i + 1 < pages.len()).then(|| (i + 1).to_string()),
    }
}
impl AuctionGateway for FakeGw {
    fn listings(&mut self, q: &ListingQuery) -> Result<Page<Listing>, AppError> {
        self.calls += 1;
        self.queries.push(q.clone());
        Ok(paginate(&self.listing_pages, q.cursor.as_deref()))
    }
    fn trades(&mut self, q: &ListingQuery) -> Result<Page<Trade>, AppError> {
        self.calls += 1;
        Ok(paginate(&self.trade_pages, q.cursor.as_deref()))
    }
    fn search(&mut self, _kw: &[String], _c: Option<&str>) -> Result<Page<Listing>, AppError> {
        self.calls += 1;
        Ok(Page {
            items: self.search_items.clone(),
            next_cursor: None,
        })
    }
}

#[derive(Default)]
struct MemCatalog {
    recorded: Vec<(CatalogObservation, OffsetDateTime)>,
    enchants: Vec<Sourced<EnchantEntry>>,
    reforges: Vec<Sourced<ReforgeEntry>>,
    values: Vec<Sourced<OptionValueEntry>>,
    purged_at: Option<OffsetDateTime>,
}
impl CatalogStore for MemCatalog {
    fn record_catalog(
        &mut self,
        obs: &[CatalogObservation],
        seen_at: OffsetDateTime,
    ) -> Result<usize, AppError> {
        self.recorded
            .extend(obs.iter().cloned().map(|x| (x, seen_at)));
        Ok(obs.len())
    }
    fn catalog_enchants(&self, q: &CatalogQuery) -> Result<Vec<Sourced<EnchantEntry>>, AppError> {
        let mut v: Vec<_> = self
            .enchants
            .iter()
            .filter(|e| {
                q.text
                    .as_ref()
                    .is_none_or(|t| e.entry.name.contains(t.as_str()))
            })
            .cloned()
            .collect();
        if let Some(n) = q.limit {
            v.truncate(n);
        }
        Ok(v)
    }
    fn catalog_reforges(&self, q: &CatalogQuery) -> Result<Vec<Sourced<ReforgeEntry>>, AppError> {
        Ok(self
            .reforges
            .iter()
            .filter(|e| q.category.as_ref().is_none_or(|c| &e.entry.category == c))
            .cloned()
            .collect())
    }
    fn catalog_option_values(
        &self,
        option_type: Option<&str>,
        _q: &CatalogQuery,
    ) -> Result<Vec<Sourced<OptionValueEntry>>, AppError> {
        Ok(self
            .values
            .iter()
            .filter(|e| option_type.is_none_or(|t| e.entry.option_type == t))
            .cloned()
            .collect())
    }
    fn catalog_option_types(&self) -> Result<Vec<(String, u64)>, AppError> {
        Ok(vec![])
    }
    fn purge_catalog(&mut self, now: OffsetDateTime) -> Result<usize, AppError> {
        self.purged_at = Some(now);
        Ok(0)
    }
}

fn enchant_entry(name: &str, rank: &str, last_seen: &str) -> EnchantEntry {
    EnchantEntry {
        slot: "접두".into(),
        name: name.into(),
        rank: rank.into(),
        effects: vec![],
        observed_count: 1,
        first_seen: "2026-11-01".into(),
        last_seen: last_seen.into(),
    }
}
fn sourced<T>(source: CatalogSource, entry: T) -> Sourced<T> {
    Sourced { source, entry }
}

// --- T5: observations are recorded from every fetched item --------------------

#[test]
fn query_listings_records_observations_from_all_fetched_items() {
    let mut gw = FakeGw {
        listing_pages: vec![
            vec![listing(
                "테스트 검",
                "한손 검",
                100,
                vec![ench("접두", "날카로운 (랭크 E)")],
            )],
            vec![listing(
                "테스트 검",
                "한손 검",
                200,
                vec![refo("1", "최소 공격력 11 레벨")],
            )],
        ],
        ..FakeGw::default()
    };
    let mut cat = MemCatalog::default();
    let opts = OptionRegistry::default();
    let mut cats = CategoryRegistry::new();
    // The filter keeps nothing: observations must still be recorded.
    let filter = ItemFilter {
        predicates: vec![OptionPredicate::Enchant(
            "접미:없는인챈트".parse::<EnchantPred>().unwrap(),
        )],
        ..ItemFilter::default()
    };
    let mut env = QueryEnv {
        opts: &opts,
        cats: &mut cats,
        catalog: Some((&mut cat, T0)),
    };
    let r = query_listings_in(
        &mut gw,
        &ListingQuery::default(),
        &filter,
        &mut env,
        true,
        5,
    )
    .unwrap();
    assert!(r.listings.is_empty());
    assert_eq!(cat.recorded.len(), 2);
    assert!(cat.recorded.iter().all(|(_, at)| *at == T0));
    assert!(cat.recorded.iter().any(|(x, _)| matches!(
        x,
        CatalogObservation::Enchant { name, .. } if name == "날카로운"
    )));
    assert!(cat.recorded.iter().any(|(x, _)| matches!(
        x,
        CatalogObservation::Reforge { category, .. } if category == "한손 검"
    )));
}

#[test]
fn query_trades_and_search_record_observations_too() {
    let mut gw = FakeGw {
        trade_pages: vec![vec![trade("a", vec![ench("접두", "날카로운 (랭크 E)")])]],
        search_items: vec![listing(
            "테스트 검",
            "한손 검",
            1,
            vec![ench("접미", "회오리 (랭크 6)")],
        )],
        ..FakeGw::default()
    };
    let mut cat = MemCatalog::default();
    let opts = OptionRegistry::default();
    let mut cats = CategoryRegistry::new();
    {
        let mut env = QueryEnv {
            opts: &opts,
            cats: &mut cats,
            catalog: Some((&mut cat, T0)),
        };
        query_trades_in(
            &mut gw,
            &ListingQuery::default(),
            &ItemFilter::default(),
            &mut env,
            false,
            1,
        )
        .unwrap();
        search_listings_in(
            &mut gw,
            &["검".to_owned()],
            None,
            &ItemFilter::default(),
            &mut env,
        )
        .unwrap();
    }
    assert_eq!(cat.recorded.len(), 2);
}

#[test]
fn query_without_catalog_records_nothing_and_keeps_option_filter_behavior() {
    let mut gw = FakeGw {
        listing_pages: vec![vec![
            listing("a", "검", 100, vec![o("세공 랭크", None, "3")]),
            listing("b", "검", 900, vec![]),
        ]],
        ..FakeGw::default()
    };
    let opts = OptionRegistry::default();
    let mut cats = CategoryRegistry::new();
    let filter = ItemFilter {
        options: vec!["세공 랭크>=3".parse::<OptionFilter>().unwrap()],
        ..ItemFilter::default()
    };
    let mut env = QueryEnv {
        opts: &opts,
        cats: &mut cats,
        catalog: None,
    };
    let r = query_listings_in(
        &mut gw,
        &ListingQuery::default(),
        &filter,
        &mut env,
        false,
        1,
    )
    .unwrap();
    assert_eq!(r.listings.len(), 1);
    assert_eq!(r.listings[0].name.base, "a");
}

// --- predicates and blueprint exclusion in the filter path ----------------------

#[test]
fn family_predicates_generic_expr_and_blueprint_exclusion_filter_listings() {
    let items = vec![
        listing(
            "검A",
            "한손 검",
            100,
            vec![
                ench("접두", "날카로운 (랭크 E)"),
                refo("1", "최소 공격력 11 레벨"),
                o("색상", Some("파트 A"), "255,0,0"),
            ],
        ),
        listing(
            "검B",
            "한손 검",
            200,
            vec![ench("접두", "날카로운 (랭크 E)")],
        ),
        listing(
            "검C 도면",
            "한손 검",
            50,
            vec![
                ench("접두", "날카로운 (랭크 E)"),
                refo("1", "최소 공격력 11 레벨"),
            ],
        ),
    ];
    let mut gw = FakeGw {
        listing_pages: vec![items],
        ..FakeGw::default()
    };
    let opts = OptionRegistry::default();
    let mut cats = CategoryRegistry::new();
    let filter = ItemFilter {
        options: vec![],
        predicates: vec![
            OptionPredicate::Enchant("접두:날카로운>=E".parse::<EnchantPred>().unwrap()),
            OptionPredicate::Reforge("최소 공격력>=10".parse::<ReforgePred>().unwrap()),
            OptionPredicate::Generic("!세공 랭크>=1".parse::<FilterExpr>().unwrap()),
        ],
        exclude_blueprints: true,
    };
    let mut env = QueryEnv {
        opts: &opts,
        cats: &mut cats,
        catalog: None,
    };
    let r = query_listings_in(
        &mut gw,
        &ListingQuery::default(),
        &filter,
        &mut env,
        false,
        1,
    )
    .unwrap();
    let names: Vec<_> = r.listings.iter().map(|l| l.name.base.as_str()).collect();
    assert_eq!(names, vec!["검A"]);
    assert_eq!(r.stats.unwrap().min, 100);
}

#[test]
fn item_filter_keep_is_and_of_all_parts() {
    let opts = OptionRegistry::default();
    let f = ItemFilter {
        exclude_blueprints: true,
        ..ItemFilter::default()
    };
    assert!(!f.keep(&ItemName::new("@천 옷본", "천 옷본"), &[], &opts));
    assert!(f.keep(&ItemName::new("천 옷", "천 옷"), &[], &opts));
    assert!(ItemFilter::default().keep(&ItemName::new("검 도면", "검 도면"), &[], &opts));
}

// --- T5: learn_catalog ---------------------------------------------------------

#[test]
fn learn_catalog_scans_category_within_max_calls_and_purges() {
    let page = |p| {
        vec![listing(
            "장갑",
            "장갑",
            p,
            vec![ench("접두", &format!("이름{p} (랭크 E)"))],
        )]
    };
    let mut gw = FakeGw {
        listing_pages: (1..=5).map(page).collect(),
        ..FakeGw::default()
    };
    let mut cat = MemCatalog::default();
    let r = learn_catalog(&mut gw, &mut cat, &OptionRegistry::default(), "장갑", 3, T0).unwrap();
    assert_eq!(gw.calls, 3, "at most max_calls requests");
    assert!(
        gw.queries
            .iter()
            .all(|q| q.category.as_deref() == Some("장갑") && q.name.is_none())
    );
    assert_eq!((r.calls, r.items, r.observations), (3, 3, 3));
    assert!(r.truncated);
    assert_eq!(cat.recorded.len(), 3);
    assert_eq!(cat.purged_at, Some(T0));

    let mut short = FakeGw {
        listing_pages: (1..=2).map(page).collect(),
        ..FakeGw::default()
    };
    let r = learn_catalog(
        &mut short,
        &mut MemCatalog::default(),
        &OptionRegistry::default(),
        "장갑",
        3,
        T0,
    )
    .unwrap();
    assert_eq!((r.calls, r.truncated), (2, false));
}

#[test]
fn learn_catalog_rejects_zero_calls_and_empty_category() {
    let mut gw = FakeGw::default();
    let mut cat = MemCatalog::default();
    let reg = OptionRegistry::default();
    let e = learn_catalog(&mut gw, &mut cat, &reg, "장갑", 0, T0).unwrap_err();
    assert!(matches!(e, AppError::Usage(_)));
    let e = learn_catalog(&mut gw, &mut cat, &reg, " ", 3, T0).unwrap_err();
    assert!(matches!(e, AppError::Usage(_)));
    assert_eq!(gw.calls, 0);
}

// --- T6: option_prices ------------------------------------------------------------

#[test]
fn option_prices_min_price_and_count_per_value_and_level() {
    let mut gw = FakeGw {
        listing_pages: vec![
            vec![
                listing(
                    "a",
                    "한손 검",
                    500,
                    vec![
                        refo("1", "최소 공격력 11 레벨"),
                        refo("2", "최대 공격력 10 레벨"),
                    ],
                ),
                listing("b", "한손 검", 300, vec![refo("1", "최소 공격력 11 레벨")]),
            ],
            vec![
                listing("c", "한손 검", 700, vec![refo("3", "최소 공격력 12 레벨")]),
                listing("d", "한손 검", 100, vec![ench("접두", "날카로운 (랭크 E)")]),
            ],
            vec![listing(
                "e",
                "한손 검",
                1,
                vec![refo("1", "최소 공격력 11 레벨")],
            )],
        ],
        ..FakeGw::default()
    };
    let r = option_prices(
        &mut gw,
        &OptionRegistry::default(),
        "한손 검",
        "세공 옵션",
        2,
    )
    .unwrap();
    assert_eq!(gw.calls, 2);
    assert!(r.truncated);
    assert_eq!(r.pages, 2);
    assert_eq!(r.listings, 4);
    let row = |v: &str, l: &str, min: u64, count: u64| OptionPriceRow {
        value: v.to_owned(),
        level: Some(l.to_owned()),
        min_price: min,
        count,
    };
    assert_eq!(
        r.rows,
        vec![
            row("최대 공격력", "10", 500, 1),
            row("최소 공격력", "11", 300, 2),
            row("최소 공격력", "12", 700, 1),
        ]
    );
}

#[test]
fn option_prices_enchant_family_uses_rank_as_level() {
    let mut gw = FakeGw {
        listing_pages: vec![vec![
            listing("a", "장갑", 10, vec![ench("접두", "날카로운 (랭크 E)")]),
            listing("b", "장갑", 30, vec![ench("접두", "날카로운 (랭크 E)")]),
            listing("c", "장갑", 20, vec![ench("접미", "회오리 (랭크 6)")]),
        ]],
        ..FakeGw::default()
    };
    let r = option_prices(&mut gw, &OptionRegistry::default(), "장갑", "인챈트", 3).unwrap();
    assert!(!r.truncated);
    let got: Vec<_> = r
        .rows
        .iter()
        .map(|x| (x.value.as_str(), x.level.as_deref(), x.min_price, x.count))
        .collect();
    assert_eq!(
        got,
        vec![("날카로운", Some("E"), 10, 2), ("회오리", Some("6"), 20, 1)]
    );
}

// --- T4: suggestions ------------------------------------------------------------

#[test]
fn suggest_uses_jamo_distance_and_substring_top_five() {
    let cands: Vec<String> = [
        "날카로운",
        "회오리",
        "세공 옵션",
        "세공 랭크",
        "인챈트",
        "날렵한",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    assert_eq!(suggest_option_names("날카론운", &cands)[0], "날카로운");
    assert_eq!(suggest_option_names("세공옵션", &cands)[0], "세공 옵션");
    assert!(suggest_option_names("날카", &cands).contains(&"날카로운".to_owned()));
    assert!(suggest_option_names("전혀다른말입니다", &cands).is_empty());
    let many: Vec<String> = (0..9).map(|i| format!("공격 {i}")).collect();
    assert_eq!(suggest_option_names("공격", &many).len(), 5);
    let dup = vec!["회오리".to_owned(), "회오리".to_owned()];
    assert_eq!(
        suggest_option_names("회오리", &dup),
        vec!["회오리".to_owned()]
    );
}

#[test]
fn unknown_enchant_name_is_not_found_with_suggestion() {
    let filter = ItemFilter {
        predicates: vec![OptionPredicate::Enchant(
            "접두:날카론운".parse::<EnchantPred>().unwrap(),
        )],
        ..ItemFilter::default()
    };
    let known_types: Vec<String> = vec!["인챈트".into()];
    let known_enchants: Vec<String> = vec!["날카로운".into(), "회오리".into()];
    let e = check_filter_names(&filter, &known_types, &known_enchants).unwrap_err();
    assert_eq!(e.exit_code(), 3);
    let AppError::NotFound { hint, .. } = e else {
        panic!("expected NotFound");
    };
    assert!(hint.unwrap().contains("날카로운"));

    let ok = ItemFilter {
        predicates: vec![OptionPredicate::Enchant(
            "접두:날카로운".parse::<EnchantPred>().unwrap(),
        )],
        ..ItemFilter::default()
    };
    check_filter_names(&ok, &known_types, &known_enchants).unwrap();
    // An empty catalog cannot judge enchant names.
    check_filter_names(&filter, &known_types, &[]).unwrap();
}

#[test]
fn unknown_option_type_is_usage_error_with_suggestion() {
    let filter = ItemFilter {
        options: vec!["세공옵션>=1".parse::<OptionFilter>().unwrap()],
        ..ItemFilter::default()
    };
    let known: Vec<String> = vec!["세공 옵션".into(), "세공 랭크".into()];
    let e = check_filter_names(&filter, &known, &[]).unwrap_err();
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("세공 옵션"), "{e}");

    let generic = ItemFilter {
        predicates: vec![OptionPredicate::Generic(
            "세공 랭크>=1|세공옵션>=2".parse::<FilterExpr>().unwrap(),
        )],
        ..ItemFilter::default()
    };
    assert!(matches!(
        check_filter_names(&generic, &known, &[]),
        Err(AppError::Usage(_))
    ));
    let fine = ItemFilter {
        options: vec!["세공 랭크>=1".parse::<OptionFilter>().unwrap()],
        ..ItemFilter::default()
    };
    check_filter_names(&fine, &known, &[]).unwrap();
}

// --- T3: catalog query merges repo data with local observations -------------

#[test]
fn catalog_enchants_merge_repo_and_local_newest_wins() {
    let cat = MemCatalog {
        enchants: vec![
            sourced(
                CatalogSource::LocalObserved,
                enchant_entry("날카로운", "E", "2026-11-09"),
            ),
            sourced(
                CatalogSource::LocalObserved,
                enchant_entry("단단한", "D", "2026-11-01"),
            ),
        ],
        ..MemCatalog::default()
    };
    let repo = vec![
        enchant_entry("날카로운", "E", "2026-11-05"),
        enchant_entry("단단한", "D", "2026-11-08"),
        enchant_entry("날렵한", "C", "2026-11-02"),
    ];
    let all = catalog_enchants(&cat, &repo, &CatalogQuery::default()).unwrap();
    let got: Vec<_> = all
        .iter()
        .map(|s| (s.entry.name.as_str(), s.source))
        .collect();
    assert_eq!(got.len(), 3);
    assert!(got.contains(&("날카로운", CatalogSource::LocalObserved)));
    assert!(got.contains(&("단단한", CatalogSource::RepoObserved)));
    assert!(got.contains(&("날렵한", CatalogSource::RepoObserved)));

    let q = CatalogQuery {
        text: Some("날".into()),
        limit: Some(1),
        ..CatalogQuery::default()
    };
    let some = catalog_enchants(&cat, &repo, &q).unwrap();
    assert_eq!(some.len(), 1);
    assert!(some[0].entry.name.contains('날'));

    let slot = CatalogQuery {
        slot: Some("접미".into()),
        ..CatalogQuery::default()
    };
    assert!(
        catalog_enchants(&MemCatalog::default(), &repo, &slot)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn catalog_reforges_values_and_types_merge_with_repo() {
    let reforge = |cat: &str| ReforgeEntry {
        option: "최소 공격력".into(),
        slot: "1".into(),
        category: cat.into(),
        min_level: 1,
        max_level: 20,
        observed_count: 2,
        first_seen: "2026-11-01".into(),
        last_seen: "2026-11-02".into(),
    };
    let value = |ty: &str, v: &str| OptionValueEntry {
        option_type: ty.into(),
        sub_type: None,
        value: v.into(),
        category: "장갑".into(),
        min_level: None,
        max_level: None,
        observed_count: 1,
        first_seen: "2026-11-01".into(),
        last_seen: "2026-11-01".into(),
    };
    let cat = MemCatalog {
        reforges: vec![sourced(CatalogSource::LocalObserved, reforge("한손 검"))],
        values: vec![sourced(
            CatalogSource::LocalObserved,
            value("세트 효과", "로컬 세트"),
        )],
        ..MemCatalog::default()
    };
    let q = CatalogQuery {
        category: Some("한손 검".into()),
        ..CatalogQuery::default()
    };
    let r = catalog_reforges(&cat, &[reforge("한손 검"), reforge("활")], &q).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(
        r[0].source,
        CatalogSource::LocalObserved,
        "tie -> local wins"
    );

    let repo_values = vec![value("세트 효과", "저장소 세트"), value("에르그", "S")];
    let v = catalog_option_values(
        &cat,
        &repo_values,
        Some("세트 효과"),
        &CatalogQuery::default(),
    )
    .unwrap();
    assert_eq!(v.len(), 2);
    let types = catalog_option_types(&cat, &repo_values).unwrap();
    assert_eq!(
        types,
        vec![("세트 효과".to_owned(), 2), ("에르그".to_owned(), 1)]
    );
}

// ---------- v11: reforge names ----------

#[test]
fn unknown_reforge_name_is_not_found_with_spaced_suggestion() {
    use mabi_ah_app::usecases::catalog::check_reforge_names;
    let known: Vec<String> = ["최대 공격력", "최소 공격력", "마법 공격력", "최대 생명력"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let filter = ItemFilter {
        predicates: vec![OptionPredicate::Reforge(
            "최대공격력>=18".parse::<ReforgePred>().unwrap(),
        )],
        ..ItemFilter::default()
    };
    match check_reforge_names(&filter, &known).unwrap_err() {
        AppError::NotFound { what, hint } => {
            assert!(what.contains("최대공격력"), "{what}");
            let hint = hint.unwrap_or_default();
            assert!(hint.starts_with("비슷한 이름: 최대 공격력"), "{hint}");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
    let ok = ItemFilter {
        predicates: vec![OptionPredicate::Reforge(
            "2:최대 공격력>=18".parse::<ReforgePred>().unwrap(),
        )],
        ..ItemFilter::default()
    };
    check_reforge_names(&ok, &known).unwrap();
    check_reforge_names(&filter, &[]).unwrap();
}
