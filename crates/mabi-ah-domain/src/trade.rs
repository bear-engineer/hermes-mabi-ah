//! Trade (거래): one completed auction-house sale.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::DomainError;
use crate::item_name::ItemName;
use crate::option::ItemOption;

#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    /// `auction_buy_id` from the API.
    pub trade_id: String,
    pub name: ItemName,
    pub count: u64,
    pub category: String,
    pub unit_price: u64,
    pub traded_at: OffsetDateTime,
    pub options: Vec<ItemOption>,
}

/// Wire shape of one `auction/history` element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeDto {
    pub auction_buy_id: String,
    pub item_name: String,
    pub item_display_name: String,
    pub item_count: u64,
    pub auction_item_category: String,
    pub auction_price_per_unit: u64,
    pub date_auction_buy: String,
    #[serde(default)]
    pub item_option: Option<Vec<ItemOption>>,
}

impl TryFrom<TradeDto> for Trade {
    type Error = DomainError;
    fn try_from(dto: TradeDto) -> Result<Self, DomainError> {
        let _ = dto;
        todo!("trade try_from")
    }
}
