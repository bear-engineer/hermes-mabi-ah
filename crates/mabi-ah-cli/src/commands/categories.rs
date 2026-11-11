//! `categories`: spec categories plus any discovered at runtime.
use clap::Args;
use mabi_ah_app::error::AppError;
use mabi_ah_domain::category::{OBSERVED_EXTRAS, SPEC_CATEGORIES};
use serde_json::{Value, json};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct CategoriesArgs {}

pub fn run(ctx: &mut Context, _args: &CategoriesArgs) -> Result<Value, AppError> {
    let items: Vec<Value> = ctx
        .categories
        .all()
        .into_iter()
        .map(|name| {
            let source = if SPEC_CATEGORIES.contains(&name.as_str()) {
                "spec"
            } else if OBSERVED_EXTRAS.contains(&name.as_str()) {
                "observed"
            } else {
                "discovered"
            };
            json!({"name": name, "source": source})
        })
        .collect();
    Ok(json!({"count": items.len(), "items": items}))
}
