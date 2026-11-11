//! Listing (매물): one active auction-house offer.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::DomainError;
use crate::item_name::ItemName;
use crate::option::ItemOption;

#[derive(Debug, Clone, PartialEq)]
pub struct Listing {
    pub name: ItemName,
    pub count: u64,
    pub category: String,
    pub unit_price: u64,
    pub expires_at: OffsetDateTime,
    pub options: Vec<ItemOption>,
}

impl Listing {
    /// `unit_price * count`, saturating at `u64::MAX`.
    pub fn total_price(&self) -> u64 {
        self.unit_price.saturating_mul(self.count)
    }
}

/// Wire shape of one `auction/list` element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListingDto {
    pub item_name: String,
    pub item_display_name: String,
    pub item_count: u64,
    pub auction_item_category: String,
    pub auction_price_per_unit: u64,
    pub date_auction_expire: String,
    #[serde(default)]
    pub item_option: Option<Vec<ItemOption>>,
}

/// Parse an API UTC timestamp such as `2026-11-13T11:43:00.000Z`.
pub(crate) fn parse_utc(s: &str) -> Result<OffsetDateTime, DomainError> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| DomainError::InvalidTimestamp(s.to_owned()))
}

impl TryFrom<ListingDto> for Listing {
    type Error = DomainError;
    fn try_from(dto: ListingDto) -> Result<Self, DomainError> {
        Ok(Self {
            name: ItemName::new(dto.item_name, dto.item_display_name),
            count: dto.item_count,
            category: dto.auction_item_category,
            unit_price: dto.auction_price_per_unit,
            expires_at: parse_utc(&dto.date_auction_expire)?,
            options: dto.item_option.unwrap_or_default(),
        })
    }
}
