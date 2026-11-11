//! `catalog`: observed enchant / reforge / option-value names merged from the
//! embedded repo data, the local store and user imports. Every row carries its
//! `source` and `observed_at`. Labanyu reference data (source `labanyu_reference`,
//! `observed_at` = fetch date) adds enchants with effects and curation, per-equipment
//! reforge ranges with the 한돌 cap, and `catalog reference --kind` name lists.
use clap::{Args, Subcommand};
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::CatalogQuery;
use mabi_ah_app::usecases::catalog::catalog_enchants_with;
use mabi_ah_app::usecases::{catalog_option_values, catalog_reforges, learn_catalog};
use mabi_ah_domain::repo_data::{CatalogSource, Sourced};
use serde::Serialize;
use serde_json::{Value, json};

use crate::context::Context;
use crate::repo_data::{LIST_KINDS, Reference};

#[derive(Debug, Args)]
pub struct CatalogArgs {
    #[command(subcommand)]
    pub command: CatalogCommand,
}

#[derive(Debug, Subcommand)]
pub enum CatalogCommand {
    /// Known enchants (name, slot, rank, effects).
    Enchant(EnchantArgs),
    /// Known reforge options per equipment category and slot (observed level range).
    Reforge(ReforgeArgs),
    /// Option types: repo facts (formats) and observed value counts.
    OptionTypes(OptionTypesArgs),
    /// Observed values of one option type (e.g. 토템 효과, 에르그).
    Values(ValuesArgs),
    /// Scan current listings of a category to grow the local catalog (uses API calls).
    Learn(LearnArgs),
    /// Import a catalog JSON file (data/observed format) as user_import.
    Import(ImportArgs),
    /// Labanyu reference name lists (totems, echostone, Murias relics, set bonuses, categories, reforge names/types).
    Reference(ReferenceArgs),
}

impl CatalogCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Enchant(_) => "catalog enchant",
            Self::Reforge(_) => "catalog reforge",
            Self::OptionTypes(_) => "catalog option-types",
            Self::Values(_) => "catalog values",
            Self::Learn(_) => "catalog learn",
            Self::Import(_) => "catalog import",
            Self::Reference(_) => "catalog reference",
        }
    }
}

#[derive(Debug, Args)]
pub struct EnchantArgs {
    /// Name substring.
    #[arg(long)]
    pub query: Option<String>,
    /// Slot (접두 or 접미).
    #[arg(long)]
    pub slot: Option<String>,
    /// Maximum rows.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ReforgeArgs {
    /// Option name substring.
    #[arg(long)]
    pub query: Option<String>,
    /// Equipment category (exact, e.g. 한손 검).
    #[arg(long)]
    pub category: Option<String>,
    /// Reforge slot (1-3).
    #[arg(long)]
    pub slot: Option<String>,
    /// Maximum rows.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct OptionTypesArgs {}

#[derive(Debug, Args)]
pub struct ReferenceArgs {
    /// Reference kind.
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(LIST_KINDS))]
    pub kind: String,
    /// Name substring.
    #[arg(long)]
    pub query: Option<String>,
    /// Maximum rows.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ValuesArgs {
    /// Option type (exact, e.g. 토템 효과).
    #[arg(long = "type")]
    pub option_type: String,
    /// Value substring.
    #[arg(long)]
    pub query: Option<String>,
    /// Category (exact).
    #[arg(long)]
    pub category: Option<String>,
    /// Maximum rows.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct LearnArgs {
    /// Auction category to scan (e.g. 장갑).
    #[arg(long)]
    pub category: String,
    /// Maximum API calls (one page each).
    #[arg(long, default_value_t = 20)]
    pub max_calls: u32,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// JSON file in the data/observed format.
    pub file: std::path::PathBuf,
}

fn clean(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn row<T: Serialize>(s: &Sourced<T>, last_seen: &str) -> Value {
    let mut v = serde_json::to_value(&s.entry).unwrap_or(Value::Null);
    if let Value::Object(m) = &mut v {
        m.insert("source".into(), json!(s.source.as_str()));
        m.insert("observed_at".into(), json!(last_seen));
    }
    v
}

fn rows_data(rows: Vec<Value>) -> Value {
    json!({ "count": rows.len(), "rows": rows })
}

pub fn run(ctx: &mut Context, args: &CatalogArgs) -> Result<Value, AppError> {
    match &args.command {
        CatalogCommand::Enchant(a) => {
            let repo = ctx.repo()?;
            let q = CatalogQuery {
                text: clean(&a.query),
                slot: clean(&a.slot),
                limit: a.limit,
                ..CatalogQuery::default()
            };
            let reference = ctx.reference()?;
            let rows = catalog_enchants_with(
                &ctx.store,
                &repo.enchants,
                &reference.enchant_entries(),
                &q,
            )?;
            Ok(rows_data(
                rows.iter().map(|s| enchant_row(s, &reference)).collect(),
            ))
        }
        CatalogCommand::Reforge(a) => {
            let repo = ctx.repo()?;
            let q = CatalogQuery {
                text: clean(&a.query),
                category: clean(&a.category),
                slot: clean(&a.slot),
                limit: a.limit,
            };
            let reference = ctx.reference()?;
            let rows = catalog_reforges(
                &ctx.store,
                &repo.reforges,
                &CatalogQuery {
                    limit: None,
                    ..q.clone()
                },
            )?;
            Ok(rows_data(reforge_rows(&rows, &reference, &q)))
        }
        CatalogCommand::Reference(a) => reference_list(&ctx.reference()?, a),
        CatalogCommand::Values(a) => {
            let repo = ctx.repo()?;
            let q = CatalogQuery {
                text: clean(&a.query),
                category: clean(&a.category),
                limit: a.limit,
                ..CatalogQuery::default()
            };
            let t = a.option_type.trim();
            let rows = catalog_option_values(&ctx.store, &repo.option_values, Some(t), &q)?;
            Ok(rows_data(
                rows.iter().map(|s| row(s, &s.entry.last_seen)).collect(),
            ))
        }
        CatalogCommand::OptionTypes(_) => option_types(ctx),
        CatalogCommand::Learn(a) => {
            let mut gw = ctx.gateway()?;
            let now = time::OffsetDateTime::now_utc();
            let r = learn_catalog(
                &mut gw,
                &mut ctx.store,
                &ctx.options,
                &a.category,
                a.max_calls,
                now,
            )?;
            Ok(json!({
                "category": r.category,
                "calls": r.calls,
                "items": r.items,
                "observations": r.observations,
                "purged": r.purged,
                "truncated": r.truncated,
            }))
        }
        CatalogCommand::Import(a) => {
            let text = std::fs::read_to_string(&a.file)
                .map_err(|e| AppError::Usage(format!("cannot read {}: {e}", a.file.display())))?;
            let today = time::OffsetDateTime::now_utc().date();
            let r = ctx.store.import_catalog_json(&text, today)?;
            Ok(json!({
                "enchants": r.enchants,
                "reforges": r.reforges,
                "option_values": r.option_values,
                "source": CatalogSource::UserImport.as_str(),
            }))
        }
    }
}

/// Enchant row plus Labanyu curation (`stats`, `target_names`, `dungeon_grade`,
/// `equipment_slot_classification`) when the reference knows the enchant, else null.
fn enchant_row(s: &Sourced<mabi_ah_domain::repo_data::EnchantEntry>, r: &Reference) -> Value {
    let mut v = row(s, &s.entry.last_seen);
    let e = &s.entry;
    let known = r.enchants.as_ref().and_then(|f| {
        f.entries
            .iter()
            .find(|x| x.slot == e.slot && x.name == e.name && x.rank == e.rank)
    });
    if let Value::Object(m) = &mut v {
        let (grade, slots, stats, targets) = match known {
            Some(k) => (
                json!(k.dungeon_grade),
                json!(k.equipment_slot_classification),
                json!(k.stats),
                json!(k.target_names),
            ),
            None => (Value::Null, Value::Null, Value::Null, Value::Null),
        };
        m.insert("dungeon_grade".into(), grade);
        m.insert("equipment_slot_classification".into(), slots);
        m.insert("stats".into(), stats);
        m.insert("target_names".into(), targets);
    }
    v
}

/// Observed reforge rows (with the reference 한돌 cap when known) followed by the
/// reference per-equipment-type ranges (no slot; only when no `--slot` is given).
fn reforge_rows(
    observed: &[Sourced<mabi_ah_domain::repo_data::ReforgeEntry>],
    r: &Reference,
    q: &CatalogQuery,
) -> Vec<Value> {
    let ranges: &[mabi_ah_domain::repo_data::ReforgeLevelRangeRef] = r
        .ranges
        .as_ref()
        .map(|f| f.entries.as_slice())
        .unwrap_or(&[]);
    let mut out: Vec<Value> = observed
        .iter()
        .map(|s| {
            let mut v = row(s, &s.entry.last_seen);
            let cap = ranges
                .iter()
                .find(|x| x.option_name == s.entry.option && x.equipment_type == s.entry.category)
                .and_then(|x| x.limit_break_max_level);
            v["limit_break_max_level"] = json!(cap);
            v
        })
        .collect();
    if q.slot.is_none()
        && let Some(f) = &r.ranges
    {
        for x in &f.entries {
            let text_ok = q
                .text
                .as_ref()
                .is_none_or(|t| x.option_name.contains(t.as_str()));
            let cat_ok = q.category.as_ref().is_none_or(|c| &x.equipment_type == c);
            if text_ok && cat_ok {
                out.push(json!({
                    "option": x.option_name,
                    "slot": Value::Null,
                    "category": x.equipment_type,
                    "min_level": x.min_level,
                    "max_level": x.max_level,
                    "limit_break_max_level": x.limit_break_max_level,
                    "range_text": x.range_text,
                    "source": CatalogSource::LabanyuReference.as_str(),
                    "observed_at": f.fetched_date(),
                }));
            }
        }
    }
    if let Some(n) = q.limit {
        out.truncate(n);
    }
    out
}

/// `catalog reference --kind K [--query] [--limit]`.
fn reference_list(r: &Reference, a: &ReferenceArgs) -> Result<Value, AppError> {
    let Some(list) = r.rows(&a.kind) else {
        return Err(AppError::NotFound {
            what: format!("reference kind '{}'", a.kind),
            hint: Some(format!("embedded kinds: {}", LIST_KINDS.join(", "))),
        });
    };
    let q = clean(&a.query);
    let mut rows: Vec<Value> = list
        .rows
        .into_iter()
        .filter(|(name, _)| q.as_ref().is_none_or(|t| name.contains(t.as_str())))
        .map(|(_, mut v)| {
            v["source"] = json!(CatalogSource::LabanyuReference.as_str());
            v
        })
        .collect();
    let total = rows.len();
    if let Some(n) = a.limit {
        rows.truncate(n);
    }
    Ok(json!({
        "kind": list.kind,
        "source": CatalogSource::LabanyuReference.as_str(),
        "source_url": list.source_url,
        "fetched_at": list.fetched_at,
        "attribution": list.attribution,
        "notes": list.notes,
        "total": total,
        "count": rows.len(),
        "rows": rows,
    }))
}

/// One row per option type: repo facts (value formats, status) joined with the
/// merged observed values (count, newest source and date).
fn option_types(ctx: &Context) -> Result<Value, AppError> {
    let repo = ctx.repo()?;
    let values = catalog_option_values(
        &ctx.store,
        &repo.option_values,
        None,
        &CatalogQuery::default(),
    )?;
    let mut rows: Vec<(String, Value)> = Vec::new();
    for f in &repo.option_families {
        rows.push((
            f.option_type.clone(),
            json!({
                "option_type": f.option_type,
                "sub_type": f.sub_type,
                "value_format": f.value_format,
                "value2_format": f.value2_format,
                "status": f.status,
                "value_count": 0,
                "source": CatalogSource::RepoFact.as_str(),
                "observed_at": Value::Null,
            }),
        ));
    }
    for s in &values {
        let t = &s.entry.option_type;
        let idx = match rows.iter().position(|(k, _)| k == t) {
            Some(i) => i,
            None => {
                rows.push((
                    t.clone(),
                    json!({"option_type": t, "value_count": 0, "source": Value::Null,
                           "observed_at": Value::Null}),
                ));
                rows.len() - 1
            }
        };
        let r = &mut rows[idx].1;
        r["value_count"] = json!(r["value_count"].as_u64().unwrap_or(0) + 1);
        let newer = r["observed_at"]
            .as_str()
            .is_none_or(|d| s.entry.last_seen.as_str() > d);
        if newer {
            r["observed_at"] = json!(s.entry.last_seen);
            r["source"] = json!(s.source.as_str());
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(rows_data(rows.into_iter().map(|(_, v)| v).collect()))
}
