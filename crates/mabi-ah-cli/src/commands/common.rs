//! Shared argument parsing and JSON shaping for the query commands.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{CatalogQuery, ListingQuery};
use mabi_ah_app::usecases::{ItemFilter, QueryEnv, catalog_enchants, check_filter_names};
use mabi_ah_domain::listing::Listing;
use mabi_ah_domain::option::predicate::{
    EnchantPred, ErgPred, FilterExpr, OptionPredicate, ReforgePred,
};
use mabi_ah_domain::option::{ItemOption, OptionFilter};
use mabi_ah_domain::price::PriceStats;
use mabi_ah_domain::price_ko::price_ko;
use mabi_ah_domain::trade::Trade;
use serde_json::{Value, json};

use crate::context::Context;
use crate::output::rfc3339;

/// Option filters shared by `listings`, `trades` and `search` (all ANDed).
#[derive(Debug, Args, Default)]
pub struct FilterArgs {
    /// Option filter `[!]<type>[:<sub>](>=|<=|=|~)<value>[|<value>...]`; repeatable.
    #[arg(long = "option", short = 'o')]
    pub option: Vec<String>,
    /// Enchant `[접두|접미:]<name>[(>=|<=|=)<rank>]`; repeatable.
    #[arg(long)]
    pub enchant: Vec<String>,
    /// Enchant rank range `[접두|접미](>=|<=|=)<rank>[,...]` (F<..<A<9<..<1); repeatable.
    #[arg(long)]
    pub enchant_rank: Vec<String>,
    /// Reforge option `[<slot>:]<option>[(>=|<=|=)<level>]`, any slot unless given; repeatable.
    #[arg(long)]
    pub reforge: Vec<String>,
    /// Erg `<grade>[(>=|<=|=)<level>]` (grade S/A/B); repeatable.
    #[arg(long)]
    pub erg: Vec<String>,
    /// Drop blueprints, patterns and symbols (도면/옷본/심볼).
    #[arg(long)]
    pub exclude_blueprints: bool,
}

fn usage(flag: &str, raw: &str, e: impl std::fmt::Display) -> AppError {
    AppError::Usage(format!("{flag} '{raw}': {e}"))
}

impl FilterArgs {
    /// Parses every flag into one [`ItemFilter`]. Plain `-o` filters keep the v0.1
    /// grammar; a leading `!` or an unescaped `|` makes it an OR/NOT expression.
    pub fn item_filter(&self) -> Result<ItemFilter, AppError> {
        let mut f = ItemFilter {
            exclude_blueprints: self.exclude_blueprints,
            ..ItemFilter::default()
        };
        for raw in &self.option {
            let t = raw.trim();
            if t.starts_with('!') || has_unescaped_pipe(t) {
                let e: FilterExpr = t.parse().map_err(|e| usage("--option", raw, e))?;
                f.predicates.push(OptionPredicate::Generic(e));
            } else {
                f.options
                    .push(t.parse().map_err(|e| usage("--option", raw, e))?);
            }
        }
        for raw in self.enchant.iter().chain(&self.enchant_rank) {
            let p: EnchantPred = raw.parse().map_err(|e| usage("--enchant", raw, e))?;
            f.predicates.push(OptionPredicate::Enchant(p));
        }
        for raw in &self.reforge {
            let p: ReforgePred = raw.parse().map_err(|e| usage("--reforge", raw, e))?;
            f.predicates.push(OptionPredicate::Reforge(p));
        }
        for raw in &self.erg {
            let p: ErgPred = raw.parse().map_err(|e| usage("--erg", raw, e))?;
            f.predicates.push(OptionPredicate::Erg(p));
        }
        Ok(f)
    }
}

fn has_unescaped_pipe(s: &str) -> bool {
    let mut esc = false;
    for c in s.chars() {
        match c {
            '\\' if !esc => esc = true,
            '|' if !esc => return true,
            _ => esc = false,
        }
    }
    false
}

/// Parses the filter flags and checks the option types / enchant names they use
/// against the registry, repo facts and the merged catalog (exit 2 / exit 3 with
/// similar names).
pub fn checked_filter(ctx: &Context, args: &FilterArgs) -> Result<ItemFilter, AppError> {
    let filter = args.item_filter()?;
    if filter.options.is_empty() && filter.predicates.is_empty() {
        return Ok(filter);
    }
    let repo = ctx.repo()?;
    let mut types: Vec<String> = ctx
        .options
        .option_types()
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    types.extend(repo.option_families.iter().map(|f| f.option_type.clone()));
    types.extend(
        ctx.store
            .catalog_option_types()?
            .into_iter()
            .map(|(t, _)| t),
    );
    types.extend(
        ctx.store
            .catalog_unparsed(None)?
            .into_iter()
            .map(|r| r.raw.option_type),
    );
    types.sort();
    types.dedup();
    let mut enchants: Vec<String> =
        catalog_enchants(&ctx.store, &repo.enchants, &CatalogQuery::default())?
            .into_iter()
            .map(|s| s.entry.name)
            .collect();
    enchants.sort();
    enchants.dedup();
    check_filter_names(&filter, &types, &enchants)?;
    Ok(filter)
}

/// Query environment that records catalog observations into the local store.
pub fn query_env(ctx: &mut Context) -> QueryEnv<'_> {
    let now = time::OffsetDateTime::now_utc();
    QueryEnv {
        opts: &ctx.options,
        cats: &mut ctx.categories,
        catalog: Some((&mut ctx.store, now)),
    }
}

/// Arguments shared by `listings` and `trades`.
#[derive(Debug, Args)]
pub struct QueryArgs {
    /// Exact item name (a leading '@' is part of the raw API name).
    #[arg(long)]
    pub name: Option<String>,
    /// Auction category (e.g. 검).
    #[arg(long)]
    pub category: Option<String>,
    #[command(flatten)]
    pub filter: FilterArgs,
    /// Include price statistics.
    #[arg(long)]
    pub stats: bool,
    /// Follow next_cursor until the end (bounded by --max-pages).
    #[arg(long)]
    pub all_pages: bool,
    /// Page limit for --all-pages (one API call per page).
    #[arg(long, default_value_t = 10)]
    pub max_pages: u32,
}

impl QueryArgs {
    pub fn query(&self) -> Result<ListingQuery, AppError> {
        let (name, category) = (clean(&self.name), clean(&self.category));
        if name.is_none() && category.is_none() {
            return Err(AppError::Usage(
                "--name or --category is required".to_owned(),
            ));
        }
        Ok(ListingQuery {
            name: name.map(str::to_owned),
            category: category.map(str::to_owned),
            cursor: None,
        })
    }
}

fn clean(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

pub fn parse_filters(raw: &[String]) -> Result<Vec<OptionFilter>, AppError> {
    raw.iter()
        .map(|s| {
            s.parse::<OptionFilter>()
                .map_err(|e| AppError::Usage(e.to_string()))
        })
        .collect()
}

pub fn option_json(o: &ItemOption) -> Value {
    json!({
        "option_type": o.option_type,
        "sub_type": o.sub_type,
        "value": o.value,
        "value2": o.value2,
        "desc": o.desc,
    })
}

pub fn listing_json(l: &Listing) -> Value {
    json!({
        "item_name": l.name.raw,
        "base_name": l.name.base,
        "display_name": l.name.display,
        "count": l.count,
        "category": l.category,
        "unit_price": l.unit_price,
        "price_ko": price_ko(l.unit_price),
        "total_price": l.total_price(),
        "expires_at": rfc3339(l.expires_at),
        "options": l.options.iter().map(option_json).collect::<Vec<_>>(),
    })
}

pub fn trade_json(t: &Trade) -> Value {
    json!({
        "trade_id": t.trade_id,
        "item_name": t.name.raw,
        "base_name": t.name.base,
        "display_name": t.name.display,
        "count": t.count,
        "category": t.category,
        "unit_price": t.unit_price,
        "price_ko": price_ko(t.unit_price),
        "total_price": t.unit_price.saturating_mul(t.count),
        "traded_at": rfc3339(t.traded_at),
        "options": t.options.iter().map(option_json).collect::<Vec<_>>(),
    })
}

pub fn stats_json(s: &Option<PriceStats>) -> Value {
    match s {
        None => Value::Null,
        Some(s) => json!({
            "min": s.min, "p25": s.p25, "median": s.median, "p75": s.p75, "max": s.max,
            "listings": s.listings, "units": s.units,
        }),
    }
}
