//! Item name triple: the raw API name, its base form and the display name.

/// `raw` is what the API returns (may start with `@`), `base` is `raw` without
/// one leading `@`, `display` is the human-readable name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemName {
    pub raw: String,
    pub base: String,
    pub display: String,
}

impl ItemName {
    pub fn new(raw: impl Into<String>, display: impl Into<String>) -> Self {
        let _ = (raw.into(), display.into());
        todo!("item_name")
    }
}
