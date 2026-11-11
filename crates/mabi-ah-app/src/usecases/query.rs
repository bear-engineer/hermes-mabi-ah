//! Listing/trade queries, keyword search and name suggestions.
//!
//! The `*_in` variants take an [`ItemFilter`] (option filters, family predicates,
//! blueprint exclusion) and a [`QueryEnv`]; when the env carries a catalog store,
//! every fetched listing/trade is recorded as catalog observations before
//! filtering. The older functions keep their signatures and behaviour.
use time::OffsetDateTime;

use mabi_ah_domain::blueprint::is_blueprint_name;
use mabi_ah_domain::catalog;
use mabi_ah_domain::category::CategoryRegistry;
use mabi_ah_domain::item_name::ItemName;
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::predicate::OptionPredicate;
use mabi_ah_domain::option::{ItemOption, OptionFilter, OptionRegistry};
use mabi_ah_domain::price::PriceStats;
use mabi_ah_domain::trade::Trade;

use crate::error::AppError;
use crate::ports::{AuctionGateway, CatalogStore, ListingQuery, Page};

/// Everything an item must satisfy to be kept (all parts are ANDed).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemFilter {
    /// Plain `-o` filters (v0.1 behaviour).
    pub options: Vec<OptionFilter>,
    /// Family predicates (enchant, reforge, erg, mod) and generic OR/NOT expressions.
    pub predicates: Vec<OptionPredicate>,
    /// Drop items whose name marks a blueprint, pattern or symbol.
    pub exclude_blueprints: bool,
}

impl ItemFilter {
    pub fn keep(&self, name: &ItemName, options: &[ItemOption], reg: &OptionRegistry) -> bool {
        if self.exclude_blueprints
            && (is_blueprint_name(&name.raw) || is_blueprint_name(&name.display))
        {
            return false;
        }
        self.options.iter().all(|f| f.matches(options, reg))
            && self.predicates.iter().all(|p| p.matches(options, reg))
    }
}

/// Registries a query reads/updates, plus an optional catalog store that receives
/// observations (with the time they were seen).
pub struct QueryEnv<'a> {
    pub opts: &'a OptionRegistry,
    pub cats: &'a mut CategoryRegistry,
    pub catalog: Option<(&'a mut dyn CatalogStore, OffsetDateTime)>,
}

impl QueryEnv<'_> {
    /// Record catalog observations from fetched items (no-op without a store).
    fn observe<'i>(
        &mut self,
        items: impl IntoIterator<Item = (&'i str, &'i [ItemOption])>,
    ) -> Result<(), AppError> {
        let Some((store, seen_at)) = self.catalog.as_mut() else {
            return Ok(());
        };
        let obs: Vec<_> = items
            .into_iter()
            .flat_map(|(cat, opts)| catalog::extract(self.opts, cat, opts))
            .collect();
        if !obs.is_empty() {
            store.record_catalog(&obs, *seen_at)?;
        }
        Ok(())
    }
}

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
pub(crate) fn fetch_pages<T>(
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

fn plain(filters: &[OptionFilter]) -> ItemFilter {
    ItemFilter {
        options: filters.to_vec(),
        ..ItemFilter::default()
    }
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
    let mut env = QueryEnv {
        opts,
        cats,
        catalog: None,
    };
    query_listings_in(gw, query, &plain(filters), &mut env, all_pages, max_pages)
}

/// Listings filtered by `filter`; observations go to `env.catalog` when set.
pub fn query_listings_in(
    gw: &mut dyn AuctionGateway,
    query: &ListingQuery,
    filter: &ItemFilter,
    env: &mut QueryEnv<'_>,
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
    env.observe(
        items
            .iter()
            .map(|l| (l.category.as_str(), l.options.as_slice())),
    )?;
    items.retain(|l| filter.keep(&l.name, &l.options, env.opts));
    Ok(listing_result(items, pages, truncated, env.cats))
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
    let mut env = QueryEnv {
        opts,
        cats,
        catalog: None,
    };
    query_trades_in(gw, query, &plain(filters), &mut env, all_pages, max_pages)
}

/// Trades filtered by `filter`; observations go to `env.catalog` when set.
pub fn query_trades_in(
    gw: &mut dyn AuctionGateway,
    query: &ListingQuery,
    filter: &ItemFilter,
    env: &mut QueryEnv<'_>,
    all_pages: bool,
    max_pages: u32,
) -> Result<TradesResult, AppError> {
    let (mut trades, pages, truncated) =
        fetch_pages(query, all_pages, max_pages, |q| gw.trades(q))?;
    env.observe(
        trades
            .iter()
            .map(|t| (t.category.as_str(), t.options.as_slice())),
    )?;
    trades.retain(|t| filter.keep(&t.name, &t.options, env.opts));
    for t in &trades {
        env.cats.record_discovered(&t.category);
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
    let mut env = QueryEnv {
        opts,
        cats,
        catalog: None,
    };
    search_listings_in(gw, keywords, category, &plain(filters), &mut env)
}

/// Keyword search filtered by category and `filter`; observations go to
/// `env.catalog` when set (all fetched listings, before filtering).
pub fn search_listings_in(
    gw: &mut dyn AuctionGateway,
    keywords: &[String],
    category: Option<&str>,
    filter: &ItemFilter,
    env: &mut QueryEnv<'_>,
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
    env.observe(
        items
            .iter()
            .map(|l| (l.category.as_str(), l.options.as_slice())),
    )?;
    items.retain(|l| {
        category.is_none_or(|c| l.category == c) && filter.keep(&l.name, &l.options, env.opts)
    });
    Ok(listing_result(items, 1, truncated, env.cats))
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
