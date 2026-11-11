//! `AuctionGateway` over HTTP.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{AuctionGateway, ListingQuery, Page};
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::trade::Trade;

use crate::client::NexonClient;
use crate::dto;
use crate::rate_limit::{Clock, Sleeper};

pub const LIST_PATH: &str = "/mabinogi/v1/auction/list";
pub const HISTORY_PATH: &str = "/mabinogi/v1/auction/history";
pub const SEARCH_PATH: &str = "/mabinogi/v1/auction/keyword-search";

impl<C: Clock, S: Sleeper> AuctionGateway for NexonClient<C, S> {
    fn listings(&mut self, query: &ListingQuery) -> Result<Page<Listing>, AppError> {
        let pairs = listing_pairs(query);
        let body = self.get_json(LIST_PATH, &as_refs(&pairs))?;
        dto::listing_page(body)
    }
    fn trades(&mut self, query: &ListingQuery) -> Result<Page<Trade>, AppError> {
        let pairs = listing_pairs(query);
        let body = self.get_json(HISTORY_PATH, &as_refs(&pairs))?;
        dto::trade_page(body)
    }
    fn search(
        &mut self,
        keywords: &[String],
        cursor: Option<&str>,
    ) -> Result<Page<Listing>, AppError> {
        let mut pairs = vec![("keyword".to_owned(), keywords.join(","))];
        if let Some(c) = cursor.filter(|c| !c.is_empty()) {
            pairs.push(("cursor".to_owned(), c.to_owned()));
        }
        let body = self.get_json(SEARCH_PATH, &as_refs(&pairs))?;
        dto::listing_page(body)
    }
}

fn listing_pairs(q: &ListingQuery) -> Vec<(String, String)> {
    [
        ("item_name", &q.name),
        ("auction_item_category", &q.category),
        ("cursor", &q.cursor),
    ]
    .into_iter()
    .filter_map(|(k, v)| {
        v.as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| (k.to_owned(), s.clone()))
    })
    .collect()
}

fn as_refs(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect()
}
