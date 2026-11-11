//! Catalog observations extracted from auction item options.
//!
//! A [`CatalogObservation`] is what one option of one item says about the game's
//! enchants, reforges and option values. Nothing here is interpreted beyond the
//! parsers: anything the parsers cannot read, and every `세공 랭크` option (its
//! meaning is not known), is kept as [`CatalogObservation::Unparsed`] with the raw
//! option. Pure; no I/O.

use crate::option::enchant_rank::EnchantRank;
use crate::option::{ItemOption, OptionRegistry, OptionValue};

/// `option_type` of the reforge rank. Observed shape unknown; always kept raw.
pub const REFORGE_RANK_TYPE: &str = "세공 랭크";

/// Option types whose values are worth cataloguing by name (and level where numeric).
pub const VALUE_TYPES: [&str; 9] = [
    "토템 효과",
    "토템 추가 옵션",
    "에코스톤 각성 능력",
    "에코스톤 고유 능력",
    "에코스톤 등급",
    "세트 효과",
    "의장 등급",
    "에르그",
    "특별 개조",
];

/// One fact read from an item option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogObservation {
    /// `인챈트` (with effects) or `인챈트 종류` (scroll, no effects).
    Enchant {
        slot: String,
        name: String,
        rank: EnchantRank,
        effects: Vec<String>,
    },
    /// `세공 옵션`: option name, level, slot index text and the item's category.
    Reforge {
        option: String,
        level: u32,
        slot: String,
        category: String,
    },
    /// A named value of a [`VALUE_TYPES`] option. `level` is the numeric part when there is one.
    OptionValue {
        option_type: String,
        sub_type: Option<String>,
        value: String,
        level: Option<u32>,
        category: String,
    },
    /// Not interpreted; the raw option is kept as received.
    Unparsed { category: String, raw: ItemOption },
}

/// Extract observations from one item's options. Identical options are reported once;
/// order follows the input. Options that carry no catalog information yield nothing.
pub fn extract(
    registry: &OptionRegistry,
    category: &str,
    options: &[ItemOption],
) -> Vec<CatalogObservation> {
    todo!()
}

fn observe(
    registry: &OptionRegistry,
    category: &str,
    o: &ItemOption,
) -> Option<CatalogObservation> {
    let unparsed = || CatalogObservation::Unparsed {
        category: category.to_owned(),
        raw: o.clone(),
    };
    let ty = o.option_type.as_str();
    if ty == REFORGE_RANK_TYPE {
        return Some(unparsed());
    }
    let value = registry.parse_value(o);
    if matches!(value, OptionValue::Raw(_)) {
        return Some(unparsed());
    }
    match (ty, value) {
        (
            "인챈트" | "인챈트 종류",
            OptionValue::Enchant {
                slot,
                name,
                rank,
                effects,
            },
        ) => Some(CatalogObservation::Enchant {
            slot,
            name,
            rank,
            effects,
        }),
        ("세공 옵션", OptionValue::Leveled { effect, level, .. }) => {
            match o
                .sub_type
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(slot) => Some(CatalogObservation::Reforge {
                    option: effect,
                    level,
                    slot: slot.to_owned(),
                    category: category.to_owned(),
                }),
                None => Some(unparsed()),
            }
        }
        (t, value) if VALUE_TYPES.contains(&t) => option_value(t, o, value, category),
        _ => None,
    }
}

fn option_value(
    ty: &str,
    o: &ItemOption,
    value: OptionValue,
    category: &str,
) -> Option<CatalogObservation> {
    let sub = o
        .sub_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let level = |n: i64| u32::try_from(n).ok();
    let (name, level) = match value {
        OptionValue::Leveled { effect, level, .. } => (effect, Some(level)),
        OptionValue::Text(t) => (t, None),
        OptionValue::Number(n) => (sub.clone()?, level(n)),
        OptionValue::CurrentMax { cur, .. } => (sub.clone()?, level(cur)),
        _ => return None,
    };
    Some(CatalogObservation::OptionValue {
        option_type: ty.to_owned(),
        sub_type: sub,
        value: name,
        level,
        category: category.to_owned(),
    })
}
