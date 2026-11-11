//! Detects blueprint / pattern / symbol items by name.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintKind {
    Blueprint,
    Pattern,
    Symbol,
}

pub fn blueprint_kind(_name: &str) -> Option<BlueprintKind> {
    todo!()
}

pub fn is_blueprint_name(name: &str) -> bool {
    blueprint_kind(name).is_some()
}
