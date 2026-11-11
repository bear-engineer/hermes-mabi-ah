//! Typed view of an [`ItemOption`]; unparseable input stays `Raw`.

use super::ItemOption;

/// Parsed option payload.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Number(i64),
    Range { min: i64, max: i64 },
    Percent(f64),
    CurrentMax { cur: i64, max: i64 },
    Rgb { r: u8, g: u8, b: u8, sparkle: bool },
    Leveled { effect: String, level: u32, amount: Option<String> },
    Enchant { slot: String, name: String, rank: u32, effects: Vec<String> },
    Text(String),
    Flag(bool),
    Raw(ItemOption),
}

/// An option with its type keys and parsed value.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedOption {
    pub option_type: String,
    pub sub_type: Option<String>,
    pub value: OptionValue,
}
