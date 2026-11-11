//! Wire envelopes and conversion to port types.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{HornMessage, NpcShop, Page};
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::trade::Trade;
use serde_json::Value;

pub fn listing_page(body: Value) -> Result<Page<Listing>, AppError> {
    let _ = body;
    todo!("node n2 green")
}

pub fn trade_page(body: Value) -> Result<Page<Trade>, AppError> {
    let _ = body;
    todo!("node n2 green")
}

pub fn npc_shop(body: Value, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError> {
    let _ = (body, npc, server, channel);
    todo!("node n2 green")
}

pub fn horn_messages(body: Value) -> Result<Vec<HornMessage>, AppError> {
    let _ = body;
    todo!("node n2 green")
}
