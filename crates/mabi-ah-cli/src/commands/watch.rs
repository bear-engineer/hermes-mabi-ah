//! `watch add|list|rm|check`.
use clap::{Args, Subcommand, ValueEnum};
use mabi_ah_app::error::AppError;
use mabi_ah_app::usecases::{add_watch, check_watch, list_watches, remove_watch};
use mabi_ah_domain::attribution::ATTRIBUTION;
use mabi_ah_domain::watch::{Alert, WatchRule};
use mabi_ah_store::SqliteStore;
use serde_json::{Value, json};

use super::collect::finish_raw;
use super::common::{listing_json, needs_expr, parse_filters};
use crate::context::Context;

#[derive(Debug, Args)]
pub struct WatchArgs {
    #[command(subcommand)]
    pub command: WatchCommand,
}

#[derive(Debug, Subcommand)]
pub enum WatchCommand {
    /// Save a price watch rule.
    Add(WatchAddArgs),
    /// List saved rules.
    List,
    /// Remove a rule by id.
    Rm(WatchRmArgs),
    /// Check current listings against all rules; reports only new alerts.
    Check(WatchCheckArgs),
}

#[derive(Debug, Args)]
pub struct WatchAddArgs {
    /// Exact item name.
    #[arg(long)]
    pub name: String,
    /// Alert when the unit price is at or below this.
    #[arg(long)]
    pub max_price: Option<u64>,
    /// Auction category for the rule.
    #[arg(long)]
    pub category: Option<String>,
    /// Option filter `<type>[:<sub>](>=|<=|=|~)<value>`; repeatable.
    #[arg(long = "option", short = 'o')]
    pub option: Vec<String>,
}

#[derive(Debug, Args)]
pub struct WatchRmArgs {
    pub id: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Emit {
    Json,
    Text,
}

#[derive(Debug, Args)]
pub struct WatchCheckArgs {
    /// Output: json envelope or one text line per alert (cron).
    #[arg(long, value_enum, default_value = "json")]
    pub emit: Emit,
}

impl WatchCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Add(_) => "watch add",
            Self::List => "watch list",
            Self::Rm(_) => "watch rm",
            Self::Check(_) => "watch check",
        }
    }
}

pub fn run(ctx: &mut Context, args: &WatchArgs) -> Result<Value, AppError> {
    match &args.command {
        WatchCommand::Add(a) => add(ctx, a),
        WatchCommand::List => list(ctx),
        WatchCommand::Rm(a) => rm(ctx, a),
        WatchCommand::Check(a) => check(ctx, a),
    }
}

/// Stable id from the rule's content (FNV-1a), so the same rule is the same id.
fn rule_id(name: &str, max: Option<u64>, category: Option<&str>, filters: &[String]) -> String {
    let canon = format!("{name}|{max:?}|{category:?}|{filters:?}");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in canon.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("w{:08x}", h >> 32)
}

fn rule_json(r: &WatchRule) -> Value {
    json!({
        "id": r.id,
        "name": r.base_name,
        "max_price": r.max_unit_price,
        "category": r.category,
        "options": r.filters,
    })
}

fn add(ctx: &mut Context, a: &WatchAddArgs) -> Result<Value, AppError> {
    let name = a.name.trim().trim_start_matches('@').trim();
    if name.is_empty() {
        return Err(AppError::Usage("--name must not be empty".to_owned()));
    }
    if let Some(o) = a.option.iter().find(|o| needs_expr(o.trim())) {
        return Err(AppError::UsageHint {
            message: format!(
                "watch rules take plain `<type>[:<sub>](>=|<=|=|~)<value>` filters only, got '{o}'"
            ),
            hint: "`!`, `|` 와 `\\` 이스케이프는 `listings -o` 에서만 쓸 수 있습니다; 감시에는 단순 필터를 쓰세요"
                .to_owned(),
        });
    }
    parse_filters(&a.option)?;
    let category = a
        .category
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    let rule = WatchRule {
        id: rule_id(name, a.max_price, category, &a.option),
        base_name: name.to_owned(),
        max_unit_price: a.max_price,
        category: category.map(str::to_owned),
        filters: a.option.clone(),
    };
    add_watch(&mut ctx.store, &rule)?;
    Ok(rule_json(&rule))
}

fn list(ctx: &mut Context) -> Result<Value, AppError> {
    let rules = list_watches(&ctx.store)?;
    Ok(json!({
        "items": rules.iter().map(rule_json).collect::<Vec<_>>(),
        "count": rules.len(),
    }))
}

fn rm(ctx: &mut Context, a: &WatchRmArgs) -> Result<Value, AppError> {
    remove_watch(&mut ctx.store, &a.id)?;
    Ok(json!({"removed": a.id}))
}

fn alert_line(a: &Alert) -> String {
    let l = &a.listing;
    format!(
        "[감시 {}] {} {}개 개당 {}골드",
        a.rule_id, l.name.display, l.count, l.unit_price
    )
}

fn check(ctx: &mut Context, a: &WatchCheckArgs) -> Result<Value, AppError> {
    let mut outbox = SqliteStore::open(&ctx.cfg.db_path)?;
    let mut gw = ctx.gateway()?;
    let report = check_watch(&mut gw, &ctx.store, &mut outbox, &ctx.options)?;
    drop(outbox);
    // Deferred delivery (retryable failure, or every rule failed): report the
    // most relevant error; alerts stay queued for the next run.
    let fatal = if report.held {
        report
            .errors
            .iter()
            .position(|(_, e)| e.retryable())
            .or(Some(0))
    } else {
        None
    };
    if let Some(pos) = fatal {
        return Err(report
            .errors
            .into_iter()
            .nth(pos)
            .map(|(_, e)| e)
            .unwrap_or_else(|| AppError::Internal("watch error vanished".to_owned())));
    }
    // Permanent per-rule failures: warn on stderr, keep delivering the rest.
    for (name, e) in &report.errors {
        eprintln!(
            "mabi-ah: 감시 규칙 '{name}'을(를) 확인하지 못했어요 ({}): {e}",
            e.code()
        );
    }
    let rule_errors: Vec<Value> = report
        .errors
        .iter()
        .map(|(name, e)| json!({"name": name, "code": e.code(), "message": e.to_string()}))
        .collect();
    let alerts = report.alerts;
    match a.emit {
        Emit::Text => {
            if alerts.is_empty() {
                finish_raw("");
            }
            let mut lines: Vec<String> = alerts.iter().map(alert_line).collect();
            lines.push(ATTRIBUTION.to_owned());
            finish_raw(&lines.join("\n"))
        }
        Emit::Json => Ok(json!({
            "count": alerts.len(),
            "rule_errors": rule_errors,
            "alerts": alerts.iter().map(|al| json!({
                "rule_id": al.rule_id,
                "listing": listing_json(&al.listing),
            })).collect::<Vec<_>>(),
        })),
    }
}
