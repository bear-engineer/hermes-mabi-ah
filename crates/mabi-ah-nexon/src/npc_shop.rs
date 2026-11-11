//! `NpcShopGateway` over HTTP.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{NpcShop, NpcShopGateway};

use crate::client::NexonClient;
use crate::rate_limit::{Clock, Sleeper};

pub const PATH: &str = "/mabinogi/v1/npcshop/list";

impl<C: Clock, S: Sleeper> NpcShopGateway for NexonClient<C, S> {
    fn npc_shop(&mut self, npc: &str, server: &str, channel: u32) -> Result<NpcShop, AppError> {
        let _ = (npc, server, channel);
        todo!("node n2 green")
    }
}
