//! Subcommands. Each module owns its clap args and a `run` returning the
//! envelope `data`.
pub mod budget;
pub mod categories;
pub mod collect;
pub mod common;
pub mod config_cmd;
pub mod doctor;
pub mod horn;
pub mod listings;
pub mod npc_shop;
pub mod rank;
pub mod search;
pub mod trades;
pub mod trend;
pub mod watch;

use clap::Subcommand;
use mabi_ah_app::error::AppError;
use serde_json::Value;

use crate::context::Context;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Current auction listings for an item or category.
    Listings(common::QueryArgs),
    /// Completed trades from the API.
    Trades(common::QueryArgs),
    /// Keyword search over current listings.
    Search(search::SearchArgs),
    /// NPC shop contents.
    NpcShop(npc_shop::NpcShopArgs),
    /// Horn-of-bugle world shouts.
    Horn(horn::HornArgs),
    /// Collect trades into the local store.
    Collect(collect::CollectArgs),
    /// Price trend from collected trades.
    Trend(trend::TrendArgs),
    /// Top items from collected trades.
    Rank(rank::RankArgs),
    /// Price watch rules.
    Watch(watch::WatchArgs),
    /// Known auction categories.
    Categories(categories::CategoriesArgs),
    /// Today's API call budget.
    Budget(budget::BudgetArgs),
    /// Environment and data health check.
    Doctor(doctor::DoctorArgs),
    /// Show the resolved configuration.
    Config(config_cmd::ConfigArgs),
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Listings(_) => "listings",
            Self::Trades(_) => "trades",
            Self::Search(_) => "search",
            Self::NpcShop(_) => "npc-shop",
            Self::Horn(_) => "horn",
            Self::Collect(_) => "collect",
            Self::Trend(_) => "trend",
            Self::Rank(_) => "rank",
            Self::Watch(w) => w.command.name(),
            Self::Categories(_) => "categories",
            Self::Budget(_) => "budget",
            Self::Doctor(_) => "doctor",
            Self::Config(_) => "config",
        }
    }

    pub fn run(&self, ctx: &mut Context) -> Result<Value, AppError> {
        match self {
            Self::Listings(a) => listings::run(ctx, a),
            Self::Trades(a) => trades::run(ctx, a),
            Self::Search(a) => search::run(ctx, a),
            Self::NpcShop(a) => npc_shop::run(ctx, a),
            Self::Horn(a) => horn::run(ctx, a),
            Self::Collect(a) => collect::run(ctx, a),
            Self::Trend(a) => trend::run(ctx, a),
            Self::Rank(a) => rank::run(ctx, a),
            Self::Watch(a) => watch::run(ctx, a),
            Self::Categories(a) => categories::run(ctx, a),
            Self::Budget(a) => budget::run(ctx, a),
            Self::Doctor(a) => doctor::run(ctx, a),
            Self::Config(a) => config_cmd::run(ctx, a),
        }
    }
}
