#![allow(clippy::unwrap_used)]
use mabi_ah_domain::option::{ItemOption, OptionRegistry, OptionValue as V};
use proptest::prelude::*;

fn o(t: &str, sub: Option<&str>, v: Option<&str>, v2: Option<&str>, d: Option<&str>) -> ItemOption {
    ItemOption {
        option_type: t.into(),
        sub_type: sub.map(Into::into),
        value: v.map(Into::into),
        value2: v2.map(Into::into),
        desc: d.map(Into::into),
    }
}

fn p(i: &ItemOption) -> V {
    OptionRegistry::default().parse(i).value
}

fn num(t: &str, v: &str) -> V {
    p(&o(t, None, Some(v), None, None))
}

#[test]
fn plain_numbers() {
    for t in [
        "품질", "남은 거래 횟수", "방어력", "보호", "마법 방어력", "마법 보호", "숙련", "보석 개조",
        "피어싱 레벨", "에코스톤 등급", "남은 사용 횟수", "남은 전용 해제 가능 횟수", "세공 랭크",
    ] {
        assert_eq!(num(t, "7"), V::Number(7), "{t}");
    }
}

#[test]
fn numbers_with_sub_type() {
    for (t, s) in [
        ("토템 효과", "지력"),
        ("토템 추가 옵션", "보너스 대미지"),
        ("토템 강화 제한", "남은 일반 강화 횟수"),
        ("에코스톤 고유 능력", "지력"),
        ("특별 개조", "R"),
    ] {
        assert_eq!(p(&o(t, Some(s), Some("8"), None, None)), V::Number(8), "{t}");
    }
}

#[test]
fn piercing_level_with_plus_value2() {
    assert_eq!(p(&o("피어싱 레벨", None, Some("1"), Some("+2"), None)), V::Number(1));
}

#[test]
fn current_max() {
    for t in ["내구력", "일반 개조"] {
        assert_eq!(p(&o(t, None, Some("5"), Some("9"), None)), V::CurrentMax { cur: 5, max: 9 }, "{t}");
    }
    assert_eq!(p(&o("에르그", Some("S"), Some("50"), Some("50"), None)), V::CurrentMax { cur: 50, max: 50 });
}

#[test]
fn ranges() {
    assert_eq!(p(&o("공격", None, Some("2"), Some("5"), None)), V::Range { min: 2, max: 5 });
    assert_eq!(p(&o("부상률", None, Some("10%"), Some("20%"), None)), V::Range { min: 10, max: 20 });
}

#[test]
fn percents() {
    for t in ["크리티컬", "밸런스", "내구도"] {
        assert_eq!(num(t, "16%"), V::Percent(16.0), "{t}");
    }
}

#[test]
fn colors() {
    assert_eq!(num("색상", "12,11,18"), V::Rgb { r: 12, g: 11, b: 18, sparkle: false });
    assert_eq!(
        p(&o("아이템 색상", Some("파트 A"), Some("128,128,128"), None, None)),
        V::Rgb { r: 128, g: 128, b: 128, sparkle: false }
    );
    assert_eq!(
        p(&o("아이템 색상", Some("파트 B"), Some("1,2,3"), None, Some("(반짝)"))),
        V::Rgb { r: 1, g: 2, b: 3, sparkle: true }
    );
}

#[test]
fn leveled() {
    assert_eq!(
        p(&o("세공 옵션", Some("1"), Some("천옷만들기 성공률(3레벨:2.40 % 증가)"), None, None)),
        V::Leveled { effect: "천옷만들기 성공률".into(), level: 3, amount: Some("2.40 % 증가".into()) }
    );
    assert_eq!(
        num("에코스톤 각성 능력", "음악적 지식 훌륭한 연주 비율 6 레벨 \n(15.00 )"),
        V::Leveled { effect: "음악적 지식 훌륭한 연주 비율".into(), level: 6, amount: Some("15.00".into()) }
    );
}

#[test]
fn enchants() {
    assert_eq!(
        p(&o("인챈트", Some("접두"), Some("광포한 (랭크 7)"), None, Some("대미지밸런스 1 감소,최대대미지 12 증가"))),
        V::Enchant {
            slot: "접두".into(),
            name: "광포한".into(),
            rank: 7,
            effects: vec!["대미지밸런스 1 감소".into(), "최대대미지 12 증가".into()],
        }
    );
    assert_eq!(
        p(&o("인챈트 종류", Some("접미"), Some("궤도 (랭크 7)"), None, None)),
        V::Enchant { slot: "접미".into(), name: "궤도".into(), rank: 7, effects: vec![] }
    );
}

#[test]
fn texts() {
    for (t, v) in [
        ("크기", "1.00cm"),
        ("의장 등급", "트렌드"),
        ("아이템 보호", "인챈트 실패"),
        ("장인 개조", "보호 2 증가,의지 6 증가"),
        ("사용 효과", "최대생명력 39 증가"),
    ] {
        assert_eq!(num(t, v), V::Text(v.into()), "{t}");
    }
    assert_eq!(
        p(&o("세트 효과", Some("1"), Some("플레이머 강화"), Some("1"), None)),
        V::Text("플레이머 강화".into())
    );
}

#[test]
fn flags() {
    for t in ["전용 해제 거래 보증서 사용 불가", "인챈트 불가능"] {
        assert_eq!(num(t, "true"), V::Flag(true), "{t}");
    }
}

#[test]
fn registry_covers_all_observed_types() {
    assert_eq!(OptionRegistry::default().option_types().len(), 40);
}

#[test]
fn unknown_and_bad_values_are_raw() {
    let unk = o("모르는 타입", None, Some("1"), None, None);
    assert_eq!(p(&unk), V::Raw(unk.clone()));
    let bad = o("공격", None, Some("abc"), None, None);
    assert_eq!(p(&bad), V::Raw(bad.clone()));
    let none = o("색상", None, None, None, None);
    assert_eq!(p(&none), V::Raw(none.clone()));
    let big = o("색상", None, Some("1,2,300"), None, None);
    assert_eq!(p(&big), V::Raw(big.clone()));
}

#[test]
fn adding_a_type_is_registering_a_parser() {
    use mabi_ah_domain::option::OptionParser;
    struct Foo;
    impl OptionParser for Foo {
        fn option_type(&self) -> &'static str {
            "새 타입"
        }
        fn parse(&self, _: &ItemOption) -> V {
            V::Flag(true)
        }
    }
    let mut r = OptionRegistry::default();
    r.register(Box::new(Foo));
    assert_eq!(r.parse(&o("새 타입", None, None, None, None)).value, V::Flag(true));
}

proptest! {
    #[test]
    fn never_panics(
        t in prop_oneof![Just("공격".to_string()), Just("색상".to_string()), Just("세공 옵션".to_string()),
            Just("인챈트".to_string()), Just("에코스톤 각성 능력".to_string()), Just("내구력".to_string()),
            Just("부상률".to_string()), ".*"],
        s in proptest::option::of(".*"),
        v in proptest::option::of(".*"),
        v2 in proptest::option::of(".*"),
        d in proptest::option::of(".*"),
    ) {
        let i = ItemOption { option_type: t, sub_type: s, value: v, value2: v2, desc: d };
        let r = OptionRegistry::default();
        let _ = r.parse(&i);
        for ty in r.option_types() {
            let mut j = i.clone();
            j.option_type = ty.to_string();
            let _ = r.parse(&j);
        }
    }
}
