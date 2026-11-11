//! Stub: node c2 implements `watch add|list|rm|check` and their arguments.
use clap::{Args, Subcommand};
use mabi_ah_app::error::AppError;
use serde_json::Value;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct WatchArgs {
    #[command(subcommand)]
    pub command: WatchCommand,
}

#[derive(Debug, Subcommand)]
pub enum WatchCommand {
    Add(WatchAddArgs),
    List,
    Rm(WatchRmArgs),
    Check,
}

#[derive(Debug, Args)]
pub struct WatchAddArgs {}

#[derive(Debug, Args)]
pub struct WatchRmArgs {}

impl WatchCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Add(_) => "watch add",
            Self::List => "watch list",
            Self::Rm(_) => "watch rm",
            Self::Check => "watch check",
        }
    }
}

pub fn run(_ctx: &mut Context, _args: &WatchArgs) -> Result<Value, AppError> {
    Err(AppError::Internal("not implemented".to_owned()))
}
