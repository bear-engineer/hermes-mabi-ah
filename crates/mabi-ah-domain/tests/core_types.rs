use mabi_ah_domain::item_name::ItemName;

#[test]
fn item_name_strips_one_leading_at() {
    let n = ItemName::new("@롱 소드", "롱 소드");
    assert_eq!(n.raw, "@롱 소드");
    assert_eq!(n.base, "롱 소드");
    assert_eq!(n.display, "롱 소드");
}

#[test]
fn item_name_strips_only_one_at() {
    assert_eq!(ItemName::new("@@x", "x").base, "@x");
}

#[test]
fn item_name_without_at_is_unchanged() {
    assert_eq!(ItemName::new("롱 소드", "롱 소드").base, "롱 소드");
}

use mabi_ah_domain::error::DomainError;
use mabi_ah_domain::listing::{Listing, ListingDto};
use mabi_ah_domain::trade::{Trade, TradeDto};
use time::macros::datetime;

const LISTING_JSON: &str = r#"{
  "item_name": "@롱 소드",
  "item_display_name": "롱 소드",
  "item_count": 3,
  "auction_item_category": "검",
  "auction_price_per_unit": 1500,
  "date_auction_expire": "2026-11-13T11:43:00.000Z",
  "item_option": [
    {"option_type": "공격", "option_sub_type": null, "option_value": "10", "option_value2": "20", "option_desc": null}
  ]
}"#;

#[test]
fn listing_dto_converts_with_options() {
    let dto: ListingDto = serde_json::from_str(LISTING_JSON).unwrap();
    let l = Listing::try_from(dto).unwrap();
    assert_eq!(l.name.base, "롱 소드");
    assert_eq!(l.name.raw, "@롱 소드");
    assert_eq!(l.count, 3);
    assert_eq!(l.category, "검");
    assert_eq!(l.unit_price, 1500);
    assert_eq!(l.expires_at, datetime!(2026-11-13 11:43:00 UTC));
    assert_eq!(l.options.len(), 1);
    assert_eq!(l.options[0].option_type, "공격");
    assert_eq!(l.options[0].value.as_deref(), Some("10"));
    assert_eq!(l.total_price(), 4500);
}

#[test]
fn listing_null_or_missing_options_is_empty() {
    let j = LISTING_JSON.replace(
        r#""item_option": [
    {"option_type": "공격", "option_sub_type": null, "option_value": "10", "option_value2": "20", "option_desc": null}
  ]"#,
        r#""item_option": null"#,
    );
    let dto: ListingDto = serde_json::from_str(&j).unwrap();
    assert!(Listing::try_from(dto).unwrap().options.is_empty());
    let j2 = j.replace(r#","item_option": null"#, "").replace(r#", "item_option": null"#, "");
    let dto: ListingDto = serde_json::from_str(&j2).unwrap();
    assert!(Listing::try_from(dto).unwrap().options.is_empty());
}

#[test]
fn listing_bad_timestamp_is_error() {
    let j = LISTING_JSON.replace("2026-11-13T11:43:00.000Z", "not-a-date");
    let dto: ListingDto = serde_json::from_str(&j).unwrap();
    assert!(matches!(
        Listing::try_from(dto),
        Err(DomainError::InvalidTimestamp(_))
    ));
}

#[test]
fn listing_total_price_saturates() {
    let dto: ListingDto = serde_json::from_str(LISTING_JSON).unwrap();
    let mut l = Listing::try_from(dto).unwrap();
    l.unit_price = u64::MAX;
    assert_eq!(l.total_price(), u64::MAX);
}

#[test]
fn trade_dto_converts() {
    let j = r#"{
      "auction_buy_id": "abc123",
      "item_name": "롱 소드",
      "item_display_name": "롱 소드",
      "item_count": 2,
      "auction_item_category": "검",
      "auction_price_per_unit": 700,
      "date_auction_buy": "2026-11-11T01:02:03.000Z",
      "item_option": null
    }"#;
    let dto: TradeDto = serde_json::from_str(j).unwrap();
    let t = Trade::try_from(dto).unwrap();
    assert_eq!(t.trade_id, "abc123");
    assert_eq!(t.name.base, "롱 소드");
    assert_eq!(t.count, 2);
    assert_eq!(t.unit_price, 700);
    assert_eq!(t.traded_at, datetime!(2026-11-11 01:02:03 UTC));
    assert!(t.options.is_empty());
}

#[test]
fn trade_bad_timestamp_is_error() {
    let j = r#"{"auction_buy_id":"a","item_name":"x","item_display_name":"x","item_count":1,
      "auction_item_category":"검","auction_price_per_unit":1,"date_auction_buy":"2026-13-99"}"#;
    let dto: TradeDto = serde_json::from_str(j).unwrap();
    assert!(matches!(
        Trade::try_from(dto),
        Err(DomainError::InvalidTimestamp(_))
    ));
}
