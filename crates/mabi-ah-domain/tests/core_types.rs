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
