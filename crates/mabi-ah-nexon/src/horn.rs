//! `HornGateway` over HTTP.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{HornGateway, HornMessage};

use crate::client::NexonClient;
use crate::dto;
use crate::rate_limit::{Clock, Sleeper};

pub const PATH: &str = "/mabinogi/v1/horn-bugle-world/history";

impl<C: Clock, S: Sleeper> HornGateway for NexonClient<C, S> {
    fn horn(&mut self, server: &str) -> Result<Vec<HornMessage>, AppError> {
        let body = self.get_json(PATH, &[("server_name", server)])?;
        dto::horn_messages(body)
    }
}
