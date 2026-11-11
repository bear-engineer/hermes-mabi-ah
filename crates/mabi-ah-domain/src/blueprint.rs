//! Detects blueprint / pattern / symbol items by name.

/// Kind of non-equipment "recipe-like" item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintKind {
    /// 도면
    Blueprint,
    /// 옷본
    Pattern,
    /// 심볼
    Symbol,
}

const KEYWORDS: [(&str, BlueprintKind); 3] = [
    ("도면", BlueprintKind::Blueprint),
    ("옷본", BlueprintKind::Pattern),
    ("심볼", BlueprintKind::Symbol),
];

/// Kind of the item if its name contains 도면 / 옷본 / 심볼 (a leading `@` is ignored).
pub fn blueprint_kind(name: &str) -> Option<BlueprintKind> {
    let name = name.trim().trim_start_matches('@');
    KEYWORDS
        .iter()
        .find(|(kw, _)| name.contains(kw))
        .map(|&(_, k)| k)
}

pub fn is_blueprint_name(name: &str) -> bool {
    blueprint_kind(name).is_some()
}
