//! Stub: node c2 implements this command and its arguments.
use clap::Args;
use mabi_ah_app::error::AppError;
use serde_json::Value;

use crate::context::Context;

#[derive(Debug, Args)]
pub struct CategoriesArgs {}

pub fn run(_ctx: &mut Context, _args: &CategoriesArgs) -> Result<Value, AppError> {
    Err(AppError::Internal("not implemented".to_owned()))
}
