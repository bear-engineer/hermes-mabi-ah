//! Client-side option filter: `<option_type>[:<sub_type>](>=|<=|=|~)<value>`.

use std::str::FromStr;

use super::{ItemOption, OptionRegistry};

/// Filter string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FilterParseError {
    #[error("missing operator (>=, <=, = or ~) in '{0}'")]
    MissingOperator(String),
    #[error("missing option_type in '{0}'")]
    MissingType(String),
    #[error("missing value in '{0}'")]
    MissingValue(String),
    #[error("operator needs a numeric value, got '{0}'")]
    NotNumeric(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Ge,
    Le,
    Eq,
    Contains,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionFilter {
    pub option_type: String,
    pub sub_type: Option<String>,
    pub op: FilterOp,
    pub value: String,
}

impl FromStr for OptionFilter {
    type Err = FilterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let _ = s;
        todo!()
    }
}

impl OptionFilter {
    /// True if any option satisfies the filter.
    pub fn matches(&self, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        let _ = (options, reg);
        todo!()
    }
}
