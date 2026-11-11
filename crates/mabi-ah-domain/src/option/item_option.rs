//! Raw option element exactly as the API sends it (see PROJECT_TERMS.md `ItemOption`).

use serde::{Deserialize, Serialize};

/// One element of `item_option`. All values arrive as strings or null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemOption {
    pub option_type: String,
    #[serde(default, rename = "option_sub_type")]
    pub sub_type: Option<String>,
    #[serde(default, rename = "option_value")]
    pub value: Option<String>,
    #[serde(default, rename = "option_value2")]
    pub value2: Option<String>,
    #[serde(default, rename = "option_desc")]
    pub desc: Option<String>,
}
