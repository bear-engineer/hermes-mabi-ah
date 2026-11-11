//! Listing/trade queries, keyword search and name suggestions.
use mabi_ah_domain::category::CategoryRegistry;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::{OptionFilter, OptionRegistry};
use mabi_ah_domain::price::PriceStats;
use mabi_ah_domain::trade::Trade;

use crate::error::AppError;
use crate::ports::{AuctionGateway, ListingQuery};

#[derive(Debug, Clone, PartialEq)]
pub struct ListingsResult {
    pub listings: Vec<Listing>,
    pub stats: Option<PriceStats>,
    pub pages: u32,
    /// More pages existed than were fetched.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradesResult {
    pub trades: Vec<Trade>,
    pub stats: Option<PriceStats>,
    pub pages: u32,
    pub truncated: bool,
}

pub fn query_listings(
    _gw: &mut dyn AuctionGateway,
    _query: &ListingQuery,
    _filters: &[OptionFilter],
    _opts: &OptionRegistry,
    _cats: &mut CategoryRegistry,
    _all_pages: bool,
    _max_pages: u32,
) -> Result<ListingsResult, AppError> {
    todo!("node A green: query_listings")
}

pub fn query_trades(
    _gw: &mut dyn AuctionGateway,
    _query: &ListingQuery,
    _filters: &[OptionFilter],
    _opts: &OptionRegistry,
    _cats: &mut CategoryRegistry,
    _all_pages: bool,
    _max_pages: u32,
) -> Result<TradesResult, AppError> {
    todo!("node A green: query_trades")
}

pub fn search_listings(
    _gw: &mut dyn AuctionGateway,
    _keywords: &[String],
    _category: Option<&str>,
    _filters: &[OptionFilter],
    _opts: &OptionRegistry,
    _cats: &mut CategoryRegistry,
) -> Result<ListingsResult, AppError> {
    todo!("node A green: search_listings")
}

/// Distinct item base names found by searching the words of `name` (max 10).
pub fn suggest_names(_gw: &mut dyn AuctionGateway, _name: &str) -> Result<Vec<String>, AppError> {
    todo!("node A green: suggest_names")
}
