use mabi_ah_domain::option::enchant_rank::EnchantRank;
use proptest::prelude::*;

const ORDER: [&str; 15] = [
    "F", "E", "D", "C", "B", "A", "9", "8", "7", "6", "5", "4", "3", "2", "1",
];

#[test]
fn order_table_is_fixed() {
    let parsed: Vec<EnchantRank> = ORDER.iter().map(|s| s.parse().unwrap()).collect();
    assert_eq!(parsed, EnchantRank::ALL);
    for w in parsed.windows(2) {
        assert!(w[0] < w[1], "{} < {}", w[0], w[1]);
    }
    assert!(EnchantRank::A < EnchantRank::N9);
    assert!(EnchantRank::N2 < EnchantRank::N1);
    for (i, r) in parsed.iter().enumerate() {
        assert_eq!(usize::from(r.ordinal()), i);
    }
}

#[test]
fn display_round_trips() {
    for s in ORDER {
        let r: EnchantRank = s.parse().unwrap();
        assert_eq!(r.to_string(), s);
    }
}

#[test]
fn accepts_prefixed_and_lowercase_forms() {
    assert_eq!("랭크 E".parse::<EnchantRank>().unwrap(), EnchantRank::E);
    assert_eq!("랭크 6".parse::<EnchantRank>().unwrap(), EnchantRank::N6);
    assert_eq!(" e ".parse::<EnchantRank>().unwrap(), EnchantRank::E);
}

#[test]
fn rejects_garbage() {
    for s in ["", "G", "0", "10", "EE", "랭크", "랭크 ", "-1", "랭크 X"] {
        assert!(s.parse::<EnchantRank>().is_err(), "{s:?}");
    }
}

proptest! {
    #[test]
    fn from_str_never_panics(s in ".*") {
        let _ = s.parse::<EnchantRank>();
    }
}
