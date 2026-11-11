#![allow(clippy::unwrap_used)]
use mabi_ah_domain::category::{CategoryRegistry, OBSERVED_EXTRAS, SPEC_CATEGORIES};
use mabi_ah_domain::price::PriceStats;

#[test]
fn spec_has_79_unique_values() {
    let mut v: Vec<_> = SPEC_CATEGORIES.to_vec();
    v.sort_unstable();
    v.dedup();
    assert_eq!(v.len(), 79);
    assert!(SPEC_CATEGORIES.contains(&"검"));
}

#[test]
fn extras_are_not_in_spec() {
    for e in OBSERVED_EXTRAS {
        assert!(!SPEC_CATEGORIES.contains(&e), "{e}");
    }
}

#[test]
fn registry_knows_spec_and_extras() {
    let r = CategoryRegistry::new();
    assert!(r.is_known("검"));
    assert!(r.is_known("애뮬릿"));
    assert!(!r.is_known("없는 카테고리"));
    assert_eq!(r.all().len(), 79 + 6);
}

#[test]
fn record_discovered_dedups() {
    let mut r = CategoryRegistry::new();
    assert!(r.record_discovered("신규"));
    assert!(!r.record_discovered("신규"));
    assert!(!r.record_discovered("검"));
    assert!(!r.record_discovered("유물"));
    assert!(r.is_known("신규"));
    assert_eq!(r.all().len(), 86);
    assert_eq!(r.all().last().map(String::as_str), Some("신규"));
}

#[test]
fn stats_empty_is_none() {
    assert_eq!(PriceStats::from_units(Vec::new()), None);
    assert_eq!(PriceStats::from_units([(5, 0)]), None);
}

#[test]
fn stats_single() {
    let s = PriceStats::from_units([(100, 4)]).unwrap();
    assert_eq!(
        (s.min, s.p25, s.median, s.p75, s.max),
        (100, 100, 100, 100, 100)
    );
    assert_eq!((s.listings, s.units), (1, 4));
}

#[test]
fn stats_weighted_by_count() {
    // units: 10 x1, 20 x1, 30 x8 -> 10 units; median rank 5 -> 30
    let s = PriceStats::from_units([(30, 8), (10, 1), (20, 1)]).unwrap();
    assert_eq!(s.min, 10);
    assert_eq!(s.p25, 30);
    assert_eq!(s.median, 30);
    assert_eq!(s.max, 30);
    assert_eq!((s.listings, s.units), (3, 10));
}

#[test]
fn stats_unweighted_four() {
    let s = PriceStats::from_units([(1, 1), (2, 1), (3, 1), (4, 1)]).unwrap();
    assert_eq!((s.min, s.p25, s.median, s.p75, s.max), (1, 1, 2, 3, 4));
}
