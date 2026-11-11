//! `NpcShopGateway` over HTTP.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{NpcShop, NpcShopGateway};

use crate::client::NexonClient;
use crate::dto;
use crate::rate_limit::{Clock, Sleeper};

pub const PATH: &str = "/mabinogi/v1/npcshop/list";

impl<C: Clock, S: Sleeper> NpcShopGateway for NexonClient<C, S> {
    fn npc_shop(&mut self, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError> {
        let ch = channel.to_string();
        let body = self.get_json(
            PATH,
            &[("npc_name", npc), ("server_name", server), ("channel", &ch)],
        )?;
        dto::npc_shop(body, npc, server, channel)
    }
}
