//! Wire envelopes and conversion to port types. Unknown fields are ignored.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{HornMessage, NpcShop, NpcShopItem, NpcShopPrice, NpcShopTab, Page};
use mabi_ah_domain::listing::{Listing, ListingDto};
use mabi_ah_domain::option::ItemOption;
use mabi_ah_domain::trade::{Trade, TradeDto};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Deserialize)]
struct AuctionItems {
    #[serde(default)]
    auction_item: Vec<ListingDto>,
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct AuctionHistory {
    #[serde(default)]
    auction_history: Vec<TradeDto>,
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct ShopEnvelope {
    #[serde(default)]
    shop: Vec<ShopTab>,
    date_inquire: String,
    #[serde(default)]
    date_shop_next_update: Option<String>,
}

#[derive(Deserialize)]
struct ShopTab {
    tab_name: String,
    #[serde(default)]
    item: Vec<ShopItem>,
}

#[derive(Deserialize)]
struct ShopItem {
    item_display_name: String,
    #[serde(default)]
    item_count: u64,
    #[serde(default)]
    item_option: Option<Vec<ItemOption>>,
    #[serde(default)]
    price: Option<Vec<ShopPrice>>,
    #[serde(default)]
    limit_type: Option<String>,
    #[serde(default)]
    limit_value: Option<u64>,
}

#[derive(Deserialize)]
struct ShopPrice {
    price_type: String,
    price_value: u64,
}

#[derive(Deserialize)]
struct HornEnvelope {
    #[serde(default)]
    horn_bugle_world_history: Vec<HornDto>,
}

#[derive(Deserialize)]
struct HornDto {
    character_name: String,
    message: String,
    date_send: String,
}

fn decode<T: DeserializeOwned>(body: Value, what: &str) -> Result<T, AppError> {
    serde_json::from_value(body)
        .map_err(|e| AppError::Internal(format!("unexpected {what} response shape: {e}")))
}

fn time_of(s: &str) -> Result<OffsetDateTime, AppError> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|_| AppError::Internal(format!("unparsable timestamp in response: {s}")))
}

/// Empty cursor strings mean "end", same as null.
fn cursor(c: Option<String>) -> Option<String> {
    c.filter(|s| !s.is_empty())
}

pub fn listing_page(body: Value) -> Result<Page<Listing>, AppError> {
    let env: AuctionItems = decode(body, "auction list")?;
    let items = env
        .auction_item
        .into_iter()
        .map(|d| Listing::try_from(d).map_err(|e| AppError::Internal(e.to_string())))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page {
        items,
        next_cursor: cursor(env.next_cursor),
    })
}

pub fn trade_page(body: Value) -> Result<Page<Trade>, AppError> {
    let env: AuctionHistory = decode(body, "auction history")?;
    let items = env
        .auction_history
        .into_iter()
        .map(|d| Trade::try_from(d).map_err(|e| AppError::Internal(e.to_string())))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Page {
        items,
        next_cursor: cursor(env.next_cursor),
    })
}

pub fn npc_shop(body: Value, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError> {
    let env: ShopEnvelope = decode(body, "npc shop")?;
    let tabs = env
        .shop
        .into_iter()
        .map(|t| NpcShopTab {
            name: t.tab_name,
            items: t
                .item
                .into_iter()
                .map(|i| NpcShopItem {
                    display_name: i.item_display_name,
                    count: i.item_count,
                    prices: i
                        .price
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| NpcShopPrice {
                            price_type: p.price_type,
                            value: p.price_value,
                        })
                        .collect(),
                    options: i.item_option.unwrap_or_default(),
                    limit_type: i.limit_type,
                    limit_value: i.limit_value,
                })
                .collect(),
        })
        .collect();
    Ok(NpcShop {
        npc: npc.to_owned(),
        server: server.to_owned(),
        channel,
        fetched_at: time_of(&env.date_inquire)?,
        next_update_at: env
            .date_shop_next_update
            .as_deref()
            .and_then(|s| time_of(s).ok()),
        tabs,
    })
}

pub fn horn_messages(body: Value) -> Result<Vec<HornMessage>, AppError> {
    let env: HornEnvelope = decode(body, "horn bugle")?;
    env.horn_bugle_world_history
        .into_iter()
        .map(|h| {
            Ok(HornMessage {
                character_name: h.character_name,
                message: h.message,
                sent_at: time_of(&h.date_send)?,
            })
        })
        .collect()
}
