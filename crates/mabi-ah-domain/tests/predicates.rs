#![allow(clippy::unwrap_used)]
use mabi_ah_domain::blueprint::{BlueprintKind, blueprint_kind, is_blueprint_name};
use mabi_ah_domain::option::enchant_rank::EnchantRank;
use mabi_ah_domain::option::predicate::{
    Cmp, EnchantPred, EnchantSlot, ErgPred, FilterExpr, ModPred, OptionPredicate,
    PredicateParseError, ReforgePred,
};
use mabi_ah_domain::option::{FilterParseError, ItemOption, OptionRegistry};
use mabi_ah_domain::price_ko::price_ko;

fn o(t: &str, sub: Option<&str>, v: Option<&str>, v2: Option<&str>, d: Option<&str>) -> ItemOption {
    ItemOption {
        option_type: t.into(),
        sub_type: sub.map(Into::into),
        value: v.map(Into::into),
        value2: v2.map(Into::into),
        desc: d.map(Into::into),
    }
}
fn ench(slot: &str, v: &str) -> ItemOption {
    o(
        "인챈트",
        Some(slot),
        Some(v),
        None,
        Some("최대대미지 5 증가"),
    )
}
fn refo(slot: &str, v: &str) -> ItemOption {
    o("세공 옵션", Some(slot), Some(v), None, None)
}
fn reg() -> OptionRegistry {
    OptionRegistry::default()
}

mod enchant {
    use super::*;
    fn hit(p: &str, opts: &[ItemOption]) -> bool {
        p.parse::<EnchantPred>().unwrap().matches(opts, &reg())
    }
    fn item() -> Vec<ItemOption> {
        vec![
            ench("접두", "날카로운 (랭크 E)"),
            ench("접미", "회오리 (랭크 6)"),
        ]
    }

    #[test]
    fn parses_fields() {
        let p: EnchantPred = "접두:날카로운".parse().unwrap();
        assert_eq!(p.slot, Some(EnchantSlot::Prefix));
        assert_eq!(p.name.as_deref(), Some("날카로운"));
        let p: EnchantPred = "접미>=7".parse().unwrap();
        assert_eq!(p.slot, Some(EnchantSlot::Suffix));
        assert_eq!(p.min, Some(EnchantRank::N7));
        assert_eq!(p.max, None);
        let p: EnchantPred = ">=E".parse().unwrap();
        assert_eq!((p.slot, p.name, p.min), (None, None, Some(EnchantRank::E)));
    }

    #[test]
    fn slot_name_rank_matching() {
        let i = item();
        assert!(hit("접두", &i));
        assert!(hit("접두:날카로운", &i));
        assert!(!hit("접미:날카로운", &i));
        assert!(hit("날카로운", &i));
        assert!(!hit("날카", &i), "name is exact");
        assert!(hit("접두:날카로운=E", &i));
        assert!(!hit("접두:날카로운=D", &i));
    }

    #[test]
    fn rank_range_uses_enchant_order() {
        let i = item();
        assert!(hit(">=E", &i));
        assert!(hit("접미>=7", &i), "6 is stronger than 7");
        assert!(!hit("접미>=5", &i));
        assert!(hit("접두<=E", &i));
        assert!(!hit("접두>=D", &i));
        assert!(hit(">=F,<=A", &i), "range of both bounds; 6 is above A");
        assert!(hit("접두:날카로운>=F,<=A", &i));
        assert!(!hit("접미>=F,<=A", &i), "6 is above A");
    }

    #[test]
    fn same_option_must_satisfy_all_parts() {
        let i = item();
        assert!(!hit("접미:날카로운", &i));
        assert!(!hit("접두>=7", &i));
    }

    #[test]
    fn errors_carry_token() {
        let e = "중간:x".parse::<EnchantPred>().unwrap_err();
        assert_eq!(e, PredicateParseError::BadSlot("중간".into()));
        assert_eq!(e.token(), "중간");
        let e = "접미>=Z".parse::<EnchantPred>().unwrap_err();
        assert_eq!(e, PredicateParseError::BadRank("Z".into()));
        assert_eq!(e.token(), "Z");
        assert!("".parse::<EnchantPred>().is_err());
    }
}

mod reforge {
    use super::*;
    fn hit(p: &str, opts: &[ItemOption]) -> bool {
        p.parse::<ReforgePred>().unwrap().matches(opts, &reg())
    }
    fn item() -> Vec<ItemOption> {
        vec![
            refo("1", "최대 공격력 18 레벨"),
            refo("2", "듀얼건 최대 대미지(20레벨:100 증가)"),
            refo("3", "최소 공격력(3레벨:2.40 % 증가)"),
        ]
    }

    #[test]
    fn parses_fields() {
        let p: ReforgePred = "2:최대 공격력>=18".parse().unwrap();
        assert_eq!(p.slot, Some(2));
        assert_eq!(p.name, "최대 공격력");
        assert_eq!(p.level, Some((Cmp::Ge, 18)));
        let p: ReforgePred = "최대 공격력".parse().unwrap();
        assert_eq!((p.slot, p.level), (None, None));
    }

    #[test]
    fn slot_agnostic_by_default() {
        let i = item();
        assert!(hit("최대 공격력>=18", &i));
        assert!(!hit("2:최대 공격력>=18", &i));
        assert!(hit("1:최대 공격력>=18", &i));
        assert!(!hit("최대 공격력>=19", &i));
        assert!(hit("최대 공격력<=18", &i));
        assert!(hit("최대 공격력=18", &i));
    }

    #[test]
    fn both_value_formats() {
        let i = item();
        assert!(hit("최소 공격력>=3", &i), "paren format");
        assert!(hit("최대 대미지>=20", &i), "name is a trailing-words match");
        assert!(hit("듀얼건 최대 대미지=20", &i));
        assert!(hit("3:최소 공격력", &i));
        assert!(!hit("최대", &i), "no partial word");
    }

    #[test]
    fn errors_carry_token() {
        assert_eq!(
            "4:최대 공격력".parse::<ReforgePred>().unwrap_err(),
            PredicateParseError::BadSlot("4".into())
        );
        assert_eq!(
            "최대 공격력>=x".parse::<ReforgePred>().unwrap_err(),
            PredicateParseError::BadLevel("x".into())
        );
        assert!(matches!(
            "1:>=3".parse::<ReforgePred>().unwrap_err(),
            PredicateParseError::MissingName(_)
        ));
    }
}

mod erg_mod {
    use super::*;
    fn erg(g: &str, cur: &str, max: &str) -> ItemOption {
        o("에르그", Some(g), Some(cur), Some(max), None)
    }

    #[test]
    fn erg_grade_and_level() {
        let i = [erg("S", "45", "50")];
        let h = |p: &str| p.parse::<ErgPred>().unwrap().matches(&i, &reg());
        assert!(h("S>=40"));
        assert!(!h("S>=46"));
        assert!(h("s"));
        assert!(!h("A>=1"));
        assert!(h("S<=45"));
        assert_eq!(
            "S>=40".parse::<ErgPred>().unwrap(),
            ErgPred {
                grade: "S".into(),
                level: Some((Cmp::Ge, 40))
            }
        );
        assert_eq!(
            "SS>=1".parse::<ErgPred>().unwrap_err(),
            PredicateParseError::BadGrade("SS".into())
        );
    }

    #[test]
    fn mods_and_piercing() {
        let i = [
            o("특별 개조", Some("R"), Some("7"), None, None),
            o("일반 개조", None, Some("5"), Some("5"), None),
            o("보석 개조", None, Some("2"), None, None),
            o(
                "장인 개조",
                None,
                Some("방어 15 증가,보호 9 증가"),
                None,
                None,
            ),
            o("피어싱 레벨", None, Some("3"), None, None),
        ];
        let h = |p: &str| p.parse::<ModPred>().unwrap().matches(&i, &reg());
        assert!(h("특별:R>=7"));
        assert!(!h("특별:S>=7"));
        assert!(!h("특별:R>=8"));
        assert!(h("일반=5"));
        assert!(h("보석"));
        assert!(h("피어싱>=3"));
        assert!(!h("피어싱>=4"));
        assert!(h("장인~보호 9"));
        assert!(!h("장인~공격"));
        assert!(h("특별 개조>=1"));
        assert_eq!(
            "엉뚱>=1".parse::<ModPred>().unwrap_err(),
            PredicateParseError::UnknownModKind("엉뚱".into())
        );
    }

    #[test]
    fn dispatch_enum() {
        let i = [erg("S", "45", "50")];
        let p = OptionPredicate::Erg("S>=40".parse().unwrap());
        assert!(p.matches(&i, &reg()));
    }
}

mod filter_expr {
    use super::*;
    fn hit(f: &str, opts: &[ItemOption]) -> bool {
        f.parse::<FilterExpr>().unwrap().matches(opts, &reg())
    }
    fn color(v: &str) -> ItemOption {
        o("색상", None, Some(v), None, Some("빨강"))
    }

    #[test]
    fn old_syntax_still_works() {
        let i = [o("에르그", Some("S"), Some("45"), Some("50"), None)];
        assert!(hit("에르그>=40", &i));
        assert!(!hit("에르그>=46", &i));
        let e: FilterExpr = "에르그>=40".parse().unwrap();
        assert!(!e.negated);
        assert_eq!(e.alternatives.len(), 1);
    }

    #[test]
    fn negation() {
        let flag = [o("인챈트 불가능", None, Some("true"), None, None)];
        assert!(hit("인챈트 불가능=true", &flag));
        assert!(!hit("!인챈트 불가능=true", &flag));
        assert!(hit("!인챈트 불가능=true", &[]));
    }

    #[test]
    fn or_alternatives_share_prefix() {
        let red = [o("색상", None, Some("1,2,3"), None, Some("빨강 계열"))];
        let blue = [o("색상", None, Some("1,2,3"), None, Some("파랑 계열"))];
        let green = [o("색상", None, Some("1,2,3"), None, Some("초록 계열"))];
        for i in [&red, &blue] {
            assert!(hit("색상~빨강|파랑", i));
        }
        assert!(!hit("색상~빨강|파랑", &green));
        assert!(hit("색상~초록|파랑", &green));
        assert!(!hit("!색상~빨강|파랑", &red));
        assert!(hit("!색상~빨강|파랑", &green));
    }

    #[test]
    fn or_full_alternatives() {
        let i = [o("품질", None, Some("3"), None, None)];
        assert!(hit("에르그>=40|품질>=3", &i));
        assert!(!hit("에르그>=40|품질>=4", &i));
        let _ = color("x");
    }

    #[test]
    fn escaping() {
        let i = [o("사용 효과", None, Some("a|b"), None, None)];
        assert!(hit(r"사용 효과~a\|b", &i));
        assert!(!hit(r"사용 효과~a\|c", &i));
        let e: FilterExpr = r"\!x=1".parse().unwrap();
        assert!(!e.negated);
        assert_eq!(e.alternatives[0].option_type, "!x");
        let e: FilterExpr = r"사용 효과=a\\b".parse().unwrap();
        assert_eq!(e.alternatives[0].value, r"a\b");
    }

    #[test]
    fn errors_keep_tokens() {
        assert_eq!(
            "에르그>=abc".parse::<FilterExpr>().unwrap_err(),
            FilterParseError::NotNumeric("abc".into())
        );
        assert!(matches!(
            "에르그>=1|".parse::<FilterExpr>().unwrap_err(),
            FilterParseError::MissingValue(_)
        ));
        assert!(matches!(
            "!".parse::<FilterExpr>().unwrap_err(),
            FilterParseError::MissingOperator(_)
        ));
        assert!(matches!(
            "".parse::<FilterExpr>().unwrap_err(),
            FilterParseError::MissingOperator(_)
        ));
    }

    #[test]
    fn enchant_numeric_filter_uses_rank_order() {
        let i = [ench("접두", "날카로운 (랭크 D)")];
        assert!(hit("인챈트>=E", &i));
        assert!(!hit("인챈트<=E", &i));
        assert!(hit("인챈트=D", &i));
        assert!(hit("인챈트>=F", &i));
        let j = [ench("접미", "회오리 (랭크 6)")];
        assert!(hit("인챈트>=7", &j));
        assert!(
            hit("인챈트>=A", &j),
            "digit ranks are stronger than letters"
        );
        assert!(!hit("인챈트>=5", &j));
        assert_eq!(
            "인챈트>=Z".parse::<FilterExpr>().unwrap_err(),
            FilterParseError::NotNumeric("Z".into())
        );
    }
}

#[test]
fn price_ko_table() {
    for (n, s) in [
        (0, "0"),
        (7490, "7,490"),
        (999, "999"),
        (9999, "9,999"),
        (10000, "1만"),
        (12345, "1만2345"),
        (150_000_000, "1억5000만"),
        (9_300_000_000, "93억"),
        (100_020_000, "1억2만"),
        (123_456_789, "1억2345만6789"),
        (1_000_000_000_000, "1조"),
        (u64::MAX, "1844경6744조737억955만1615"),
    ] {
        assert_eq!(price_ko(n), s, "{n}");
    }
}

#[test]
fn blueprint_detection() {
    assert!(is_blueprint_name("도면 - 호크아이 건틀렛"));
    assert!(is_blueprint_name("@옷본 - 숙녀복"));
    assert!(is_blueprint_name("라이트 심볼"));
    assert!(is_blueprint_name("마스터 도면"));
    assert!(!is_blueprint_name("호크아이 건틀렛"));
    assert!(!is_blueprint_name(""));
    assert_eq!(blueprint_kind("도면 - 가"), Some(BlueprintKind::Blueprint));
    assert_eq!(blueprint_kind("옷본 - 가"), Some(BlueprintKind::Pattern));
    assert_eq!(blueprint_kind("가 심볼"), Some(BlueprintKind::Symbol));
    assert_eq!(blueprint_kind("검"), None);
}
