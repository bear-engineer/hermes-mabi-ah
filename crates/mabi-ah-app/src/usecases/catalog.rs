//! Catalog use cases: learning from category scans, reading the catalog merged with
//! embedded repo data, and checking filter names against it.
//!
//! Merge rule (v3, `repo_data::merge`): per key the newest `last_seen` wins; on a tie
//! the higher source priority (local observation over repo data). Repo entries are
//! passed in already loaded and are tagged [`CatalogSource::RepoObserved`].

use time::OffsetDateTime;

use mabi_ah_domain::catalog;
use mabi_ah_domain::option::predicate::{OptionPredicate, reforge_name_matches};
use mabi_ah_domain::option::{OptionFilter, OptionRegistry};
use mabi_ah_domain::repo_data::{
    CatalogSource, EnchantEntry, ObservedEntry, OptionValueEntry, ReforgeEntry, Sourced, merge,
};

use crate::error::AppError;
use crate::ports::{AuctionGateway, CatalogQuery, CatalogStore, ListingQuery};
use crate::usecases::query::ItemFilter;
use crate::usecases::suggest::suggest_option_names;

/// What one `catalog learn` scan did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnReport {
    pub category: String,
    /// Requests sent (never more than `max_calls`).
    pub calls: u32,
    /// Listings scanned.
    pub items: usize,
    /// Observations written to the catalog.
    pub observations: usize,
    /// Catalog rows purged (older than 30 days).
    pub purged: usize,
    /// The category had more pages than `max_calls` allowed.
    pub truncated: bool,
}

/// Scan current listings of `category`, one page per call and at most `max_calls`
/// calls, recording every listing's options in the catalog as seen at `now`; then
/// purge observations older than 30 days. Pages already scanned stay recorded if a
/// later call fails.
pub fn learn_catalog(
    gw: &mut dyn AuctionGateway,
    store: &mut dyn CatalogStore,
    reg: &OptionRegistry,
    category: &str,
    max_calls: u32,
    now: OffsetDateTime,
) -> Result<LearnReport, AppError> {
    let category = category.trim();
    if category.is_empty() {
        return Err(AppError::Usage("a category is required".to_owned()));
    }
    if max_calls == 0 {
        return Err(AppError::Usage("max calls must be at least 1".to_owned()));
    }
    let mut query = ListingQuery {
        category: Some(category.to_owned()),
        ..ListingQuery::default()
    };
    let mut report = LearnReport {
        category: category.to_owned(),
        calls: 0,
        items: 0,
        observations: 0,
        purged: 0,
        truncated: false,
    };
    loop {
        let page = gw.listings(&query)?;
        report.calls += 1;
        report.items += page.items.len();
        let obs: Vec<_> = page
            .items
            .iter()
            .flat_map(|l| catalog::extract(reg, &l.category, &l.options))
            .collect();
        if !obs.is_empty() {
            report.observations += store.record_catalog(&obs, now)?;
        }
        match page.next_cursor {
            Some(c) if report.calls < max_calls => query.cursor = Some(c),
            Some(_) => {
                report.truncated = true;
                break;
            }
            None => break,
        }
    }
    report.purged = store.purge_catalog(now)?;
    Ok(report)
}

fn has(text: &str, q: Option<&String>) -> bool {
    q.is_none_or(|t| text.contains(t.as_str()))
}

fn is(v: &str, q: Option<&String>) -> bool {
    q.is_none_or(|t| v == t)
}

/// Local rows (unlimited) + matching repo entries, merged, then limited.
fn merged<T: ObservedEntry + Clone>(
    local: Vec<Sourced<T>>,
    repo: &[T],
    keep: impl Fn(&T) -> bool,
    limit: Option<usize>,
) -> Vec<Sourced<T>> {
    let repo = repo.iter().filter(|e| keep(e)).map(|e| Sourced {
        source: CatalogSource::RepoObserved,
        entry: e.clone(),
    });
    let mut out = merge(local.into_iter().chain(repo));
    if let Some(n) = limit {
        out.truncate(n);
    }
    out
}

fn unlimited(q: &CatalogQuery) -> CatalogQuery {
    CatalogQuery {
        limit: None,
        ..q.clone()
    }
}

/// Enchants matching `q` (name substring, exact slot) from the store and repo data.
pub fn catalog_enchants(
    store: &dyn CatalogStore,
    repo: &[EnchantEntry],
    q: &CatalogQuery,
) -> Result<Vec<Sourced<EnchantEntry>>, AppError> {
    catalog_enchants_with(store, repo, &[], q)
}

/// Like [`catalog_enchants`], plus `reference` entries tagged
/// [`CatalogSource::LabanyuReference`] (dated by their `fetched_at`; on a tie every
/// observed source wins).
pub fn catalog_enchants_with(
    store: &dyn CatalogStore,
    repo: &[EnchantEntry],
    reference: &[EnchantEntry],
    q: &CatalogQuery,
) -> Result<Vec<Sourced<EnchantEntry>>, AppError> {
    let keep = |e: &EnchantEntry| has(&e.name, q.text.as_ref()) && is(&e.slot, q.slot.as_ref());
    let reference = reference.iter().filter(|e| keep(e)).map(|e| Sourced {
        source: CatalogSource::LabanyuReference,
        entry: e.clone(),
    });
    let local = store.catalog_enchants(&unlimited(q))?;
    let mut out = merged(
        local.into_iter().chain(reference).collect(),
        repo,
        keep,
        None,
    );
    if let Some(n) = q.limit {
        out.truncate(n);
    }
    Ok(out)
}

/// Reforge options matching `q` (option substring, exact category and slot).
pub fn catalog_reforges(
    store: &dyn CatalogStore,
    repo: &[ReforgeEntry],
    q: &CatalogQuery,
) -> Result<Vec<Sourced<ReforgeEntry>>, AppError> {
    let local = store.catalog_reforges(&unlimited(q))?;
    Ok(merged(
        local,
        repo,
        |e| {
            has(&e.option, q.text.as_ref())
                && is(&e.category, q.category.as_ref())
                && is(&e.slot, q.slot.as_ref())
        },
        q.limit,
    ))
}

/// Option values of `option_type` (all when `None`) matching `q` (value substring,
/// exact category).
pub fn catalog_option_values(
    store: &dyn CatalogStore,
    repo: &[OptionValueEntry],
    option_type: Option<&str>,
    q: &CatalogQuery,
) -> Result<Vec<Sourced<OptionValueEntry>>, AppError> {
    let local = store.catalog_option_values(option_type, &unlimited(q))?;
    Ok(merged(
        local,
        repo,
        |e| {
            option_type.is_none_or(|t| e.option_type == t)
                && has(&e.value, q.text.as_ref())
                && is(&e.category, q.category.as_ref())
        },
        q.limit,
    ))
}

/// Option types with their merged value-row counts, sorted by type.
pub fn catalog_option_types(
    store: &dyn CatalogStore,
    repo: &[OptionValueEntry],
) -> Result<Vec<(String, u64)>, AppError> {
    let all = catalog_option_values(store, repo, None, &CatalogQuery::default())?;
    let mut out: Vec<(String, u64)> = Vec::new();
    for s in all {
        match out.iter_mut().find(|(t, _)| *t == s.entry.option_type) {
            Some((_, n)) => *n += 1,
            None => out.push((s.entry.option_type, 1)),
        }
    }
    out.sort();
    Ok(out)
}

fn similar(input: &str, known: &[String]) -> Option<String> {
    let names = suggest_option_names(input, known);
    (!names.is_empty()).then(|| format!("비슷한 이름: {}", names.join(", ")))
}

/// Usage error (exit 2) naming similar types when `option_type` is not in
/// `known_types`; `None` when it is known or there is nothing to judge by.
pub fn unknown_type_error(option_type: &str, known_types: &[String]) -> Option<AppError> {
    let t = option_type.trim();
    if known_types.is_empty() || known_types.iter().any(|k| k.trim() == t) {
        return None;
    }
    Some(AppError::UsageHint {
        message: format!("unknown option type '{t}'"),
        hint: similar(t, known_types)
            .unwrap_or_else(|| "`catalog option-types` 로 옵션 종류를 확인하세요".to_owned()),
    })
}

/// Check the names a filter uses. An option type missing from `known_types` is a
/// usage error (exit 2) naming similar types; an enchant predicate naming an enchant
/// missing from `known_enchants` is not found (exit 3) with similar names in the
/// hint. An empty known list skips that check (nothing to judge by).
pub fn check_filter_names(
    filter: &ItemFilter,
    known_types: &[String],
    known_enchants: &[String],
) -> Result<(), AppError> {
    let generic = filter.predicates.iter().flat_map(|p| match p {
        OptionPredicate::Generic(e) => e.alternatives.as_slice(),
        _ => &[],
    });
    let filters: Vec<&OptionFilter> = filter.options.iter().chain(generic).collect();
    if !known_types.is_empty() {
        for f in filters {
            if let Some(e) = unknown_type_error(&f.option_type, known_types) {
                return Err(e);
            }
        }
    }
    if known_enchants.is_empty() {
        return Ok(());
    }
    for p in &filter.predicates {
        if let OptionPredicate::Enchant(e) = p
            && let Some(name) = e.name.as_deref().map(str::trim)
            && !known_enchants.iter().any(|k| k == name)
        {
            return Err(AppError::NotFound {
                what: format!("enchant '{name}'"),
                hint: Some(similar(name, known_enchants).unwrap_or_else(|| {
                    "`catalog enchant` 또는 `catalog learn`으로 이름을 확인하세요".to_owned()
                })),
            });
        }
    }
    Ok(())
}

/// Check reforge predicate names against `known_reforges` (catalog + reference
/// option names): an unknown name is not found (exit 3) with similar names in the
/// hint. An empty known list skips the check.
pub fn check_reforge_names(filter: &ItemFilter, known_reforges: &[String]) -> Result<(), AppError> {
    if known_reforges.is_empty() {
        return Ok(());
    }
    for p in &filter.predicates {
        if let OptionPredicate::Reforge(r) = p {
            let name = r.name.trim();
            if !known_reforges.iter().any(|k| reforge_name_matches(k, name)) {
                return Err(AppError::NotFound {
                    what: format!("reforge option '{name}'"),
                    hint: Some(similar(name, known_reforges).unwrap_or_else(|| {
                        "`catalog reforge` 또는 `catalog reference --kind reforge_option_names`로 이름을 확인하세요"
                            .to_owned()
                    })),
                });
            }
        }
    }
    Ok(())
}
