//! Listing/trade queries, keyword search and name suggestions.
use mabi_ah_domain::category::CategoryRegistry;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::{OptionFilter, OptionRegistry};
use mabi_ah_domain::price::PriceStats;
use mabi_ah_domain::trade::Trade;

use crate::error::AppError;
use crate::ports::{AuctionGateway, ListingQuery, Page};

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

/// Fetch pages via `fetch` until the cursor ends, `all_pages` is off or
/// `max_pages` is reached. Returns items, pages fetched and truncation.
fn fetch_pages<T>(
    query: &ListingQuery,
    all_pages: bool,
    max_pages: u32,
    mut fetch: impl FnMut(&ListingQuery) -> Result<Page<T>, AppError>,
) -> Result<(Vec<T>, u32, bool), AppError> {
    let limit = if all_pages { max_pages.max(1) } else { 1 };
    let mut q = query.clone();
    let (mut items, mut pages) = (Vec::new(), 0);
    loop {
        let page = fetch(&q)?;
        pages += 1;
        items.extend(page.items);
        match page.next_cursor {
            Some(c) if pages < limit => q.cursor = Some(c),
            Some(_) => return Ok((items, pages, true)),
            None => return Ok((items, pages, false)),
        }
    }
}

fn keep(
    options: &[mabi_ah_domain::option::ItemOption],
    filters: &[OptionFilter],
    reg: &OptionRegistry,
) -> bool {
    filters.iter().all(|f| f.matches(options, reg))
}

fn listing_result(
    listings: Vec<Listing>,
    pages: u32,
    truncated: bool,
    cats: &mut CategoryRegistry,
) -> ListingsResult {
    for l in &listings {
        cats.record_discovered(&l.category);
    }
    let stats = PriceStats::from_units(listings.iter().map(|l| (l.unit_price, l.count)));
    ListingsResult {
        listings,
        stats,
        pages,
        truncated,
    }
}

pub fn query_listings(
    gw: &mut dyn AuctionGateway,
    query: &ListingQuery,
    filters: &[OptionFilter],
    opts: &OptionRegistry,
    cats: &mut CategoryRegistry,
    all_pages: bool,
    max_pages: u32,
) -> Result<ListingsResult, AppError> {
    let fetched = fetch_pages(query, all_pages, max_pages, |q| gw.listings(q));
    let (mut items, pages, truncated) = match fetched {
        Ok(v) => v,
        Err(AppError::NotFound { what, hint }) if query.name.is_some() => {
            let name = query.name.as_deref().unwrap_or_default();
            let hint = match suggest_names(gw, name) {
                Ok(names) if !names.is_empty() => {
                    Some(format!("비슷한 이름: {}", names.join(", ")))
                }
                _ => hint,
            };
            return Err(AppError::NotFound { what, hint });
        }
        Err(e) => return Err(e),
    };
    items.retain(|l| keep(&l.options, filters, opts));
    Ok(listing_result(items, pages, truncated, cats))
}

pub fn query_trades(
    gw: &mut dyn AuctionGateway,
    query: &ListingQuery,
    filters: &[OptionFilter],
    opts: &OptionRegistry,
    cats: &mut CategoryRegistry,
    all_pages: bool,
    max_pages: u32,
) -> Result<TradesResult, AppError> {
    let (mut trades, pages, truncated) =
        fetch_pages(query, all_pages, max_pages, |q| gw.trades(q))?;
    trades.retain(|t| keep(&t.options, filters, opts));
    for t in &trades {
        cats.record_discovered(&t.category);
    }
    let stats = PriceStats::from_units(trades.iter().map(|t| (t.unit_price, t.count)));
    Ok(TradesResult {
        trades,
        stats,
        pages,
        truncated,
    })
}

/// Max keywords the search endpoint accepts.
const MAX_KEYWORDS: usize = 10;

pub fn search_listings(
    gw: &mut dyn AuctionGateway,
    keywords: &[String],
    category: Option<&str>,
    filters: &[OptionFilter],
    opts: &OptionRegistry,
    cats: &mut CategoryRegistry,
) -> Result<ListingsResult, AppError> {
    if keywords.is_empty() {
        return Err(AppError::Usage(
            "at least one keyword is required".to_owned(),
        ));
    }
    if keywords.len() > MAX_KEYWORDS {
        return Err(AppError::Usage(format!(
            "at most {MAX_KEYWORDS} keywords are allowed"
        )));
    }
    let page = gw.search(keywords, None)?;
    let truncated = page.next_cursor.is_some();
    let mut items = page.items;
    items.retain(|l| category.is_none_or(|c| l.category == c) && keep(&l.options, filters, opts));
    Ok(listing_result(items, 1, truncated, cats))
}

/// Distinct item base names found by searching the words of `name` (max 10).
pub fn suggest_names(gw: &mut dyn AuctionGateway, name: &str) -> Result<Vec<String>, AppError> {
    let words: Vec<String> = name
        .split_whitespace()
        .take(MAX_KEYWORDS)
        .map(str::to_owned)
        .collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let page = gw.search(&words, None)?;
    let mut names: Vec<String> = Vec::new();
    for l in page.items {
        if names.len() == MAX_KEYWORDS {
            break;
        }
        if !names.contains(&l.name.base) {
            names.push(l.name.base);
        }
    }
    Ok(names)
}
