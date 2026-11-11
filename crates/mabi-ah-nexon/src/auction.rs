//! `AuctionGateway` over HTTP.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{AuctionGateway, ListingQuery, Page};
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::trade::Trade;

use crate::client::NexonClient;
use crate::rate_limit::{Clock, Sleeper};

pub const LIST_PATH: &str = "/mabinogi/v1/auction/list";
pub const HISTORY_PATH: &str = "/mabinogi/v1/auction/history";
pub const SEARCH_PATH: &str = "/mabinogi/v1/auction/keyword-search";

impl<C: Clock, S: Sleeper> AuctionGateway for NexonClient<C, S> {
    fn listings(&mut self, query: &ListingQuery) -> Result<Page<Listing>, AppError> {
        let _ = query;
        todo!("node n2 green")
    }
    fn trades(&mut self, query: &ListingQuery) -> Result<Page<Trade>, AppError> {
        let _ = query;
        todo!("node n2 green")
    }
    fn search(
        &mut self,
        keywords: &[String],
        cursor: Option<&str>,
    ) -> Result<Page<Listing>, AppError> {
        let _ = (keywords, cursor);
        todo!("node n2 green")
    }
}
