//! `npc-shop`: what an NPC shop sells right now.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::query_npc_shop;
use serde_json::{Value, json};

use super::common::option_json;
use crate::context::Context;
use crate::output::rfc3339;

#[derive(Debug, Args)]
pub struct NpcShopArgs {
    #[arg(long)]
    pub npc: String,
    #[arg(long)]
    pub server: String,
    #[arg(long, default_value_t = 1)]
    pub channel: u32,
}

pub fn run(ctx: &mut Context, args: &NpcShopArgs) -> Result<Value, AppError> {
    let mut gw = ctx.gateway()?;
    let shop = query_npc_shop(&mut gw, &args.npc, &args.server, args.channel)?;
    let tabs: Vec<Value> = shop
        .tabs
        .iter()
        .map(|t| {
            let items: Vec<Value> = t
                .items
                .iter()
                .map(|i| {
                    json!({
                        "display_name": i.display_name,
                        "count": i.count,
                        "prices": i.prices.iter()
                            .map(|p| json!({"price_type": p.price_type, "value": p.value}))
                            .collect::<Vec<_>>(),
                        "options": i.options.iter().map(option_json).collect::<Vec<_>>(),
                        "limit_type": i.limit_type,
                        "limit_value": i.limit_value,
                    })
                })
                .collect();
            json!({"name": t.name, "items": items})
        })
        .collect();
    Ok(json!({
        "npc": shop.npc,
        "server": shop.server,
        "channel": shop.channel,
        "fetched_at": rfc3339(shop.fetched_at),
        "next_update_at": shop.next_update_at.map(rfc3339),
        "tabs": tabs,
    }))
}
