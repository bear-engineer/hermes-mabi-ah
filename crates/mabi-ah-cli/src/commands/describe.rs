//! `describe`: machine-readable description of the CLI — every subcommand and
//! flag (walked from the clap definition), filter grammar, option families,
//! enchant ranks, exit codes and examples.
use clap::{ArgAction, Args, CommandFactory};
use mabi_ah_app::error::AppError;
use serde_json::{Value, json};

use crate::context::Context;
use crate::repo_data::RepoData;

#[derive(Debug, Args)]
pub struct DescribeArgs {}

/// Usage examples; every leaf subcommand has at least one (test-enforced).
pub const EXAMPLES: &[&str] = &[
    "mabi-ah listings --name \"@롱 소드\" --stats",
    "mabi-ah listings --category 장갑 --enchant '접두:날카로운' --enchant-rank '>=E'",
    "mabi-ah listings --category \"한손 검\" --reforge '최대 공격력>=18' --exclude-blueprints",
    "mabi-ah listings --category \"한손 검\" --reforge '2:최대 공격력>=18' --erg 'S>=40'",
    "mabi-ah listings --category 검 -o '!인챈트 불가능=true' -o '색상~빨강|파랑'",
    "mabi-ah trades --name \"@롱 소드\" --stats --enchant-rank '>=E'",
    "mabi-ah search 드래곤 블레이드 --exclude-blueprints",
    "mabi-ah npc-shop --npc 델 --server 류트",
    "mabi-ah horn --server 류트 --redact-names",
    "mabi-ah collect --quiet",
    "mabi-ah trend --name \"@롱 소드\" --window 7d",
    "mabi-ah rank --by volume --window 24h",
    "mabi-ah watch add --name \"@롱 소드\" --max-price 100000",
    "mabi-ah watch list",
    "mabi-ah watch rm 1",
    "mabi-ah watch check --emit text",
    "mabi-ah categories",
    "mabi-ah budget",
    "mabi-ah doctor",
    "mabi-ah config",
    "mabi-ah catalog enchant --query 날카",
    "mabi-ah catalog reforge --category \"한손 검\"",
    "mabi-ah catalog option-types",
    "mabi-ah catalog values --type \"토템 효과\"",
    "mabi-ah catalog learn --category 장갑 --max-calls 3",
    "mabi-ah catalog import ./enchants.json",
    "mabi-ah catalog reference --kind totem_abilities --query 대미지",
    "mabi-ah catalog reference --kind echostone_awakening_abilities --query 체력 --limit 5",
    "mabi-ah option-prices --category 토템 --family \"토템 효과\" --max-pages 2",
    "mabi-ah describe",
];

fn flag_json(a: &clap::Arg) -> Value {
    let positional = a.is_positional();
    json!({
        "id": a.get_id().as_str(),
        "long": a.get_long().map(|l| format!("--{l}")),
        "short": a.get_short().map(|s| format!("-{s}")),
        "positional": positional,
        "required": a.is_required_set(),
        "repeatable": matches!(a.get_action(), ArgAction::Append | ArgAction::Count)
            || (positional && a.get_num_args().is_some_and(|n| n.max_values() > 1)),
        "takes_value": a.get_action().takes_values(),
        "default": a.get_default_values().iter().map(|v| v.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        "values": a.get_possible_values().iter().map(|v| v.get_name().to_owned()).collect::<Vec<_>>(),
        "help": a.get_help().map(|h| h.to_string()),
    })
}

fn visible_args(c: &clap::Command) -> impl Iterator<Item = &clap::Arg> {
    c.get_arguments()
        .filter(|a| !a.is_hide_set() && !a.is_global_set())
        .filter(|a| !matches!(a.get_id().as_str(), "help" | "version"))
}

/// Leaf subcommands as `(full name, command)`, e.g. `catalog enchant`.
pub fn leaves(root: &clap::Command) -> Vec<(String, clap::Command)> {
    fn walk(prefix: &str, c: &clap::Command, out: &mut Vec<(String, clap::Command)>) {
        for s in c.get_subcommands().filter(|s| s.get_name() != "help") {
            let name = if prefix.is_empty() {
                s.get_name().to_owned()
            } else {
                format!("{prefix} {}", s.get_name())
            };
            if s.has_subcommands() {
                walk(&name, s, out);
            } else {
                out.push((name, s.clone()));
            }
        }
    }
    let mut out = Vec::new();
    walk("", root, &mut out);
    out
}

pub fn describe(root: &clap::Command, repo: &RepoData) -> Value {
    let commands: Vec<Value> = leaves(root)
        .iter()
        .map(|(name, c)| {
            let prefix = format!("mabi-ah {name}");
            json!({
                "name": name,
                "about": c.get_about().map(|a| a.to_string()),
                "flags": visible_args(c).map(flag_json).collect::<Vec<_>>(),
                "examples": EXAMPLES
                    .iter()
                    .filter(|e| **e == prefix || e.starts_with(&format!("{prefix} ")))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    let globals: Vec<Value> = root
        .get_arguments()
        .filter(|a| a.is_global_set() && !a.is_hide_set())
        .map(flag_json)
        .collect();
    json!({
        "name": root.get_name(),
        "version": env!("CARGO_PKG_VERSION"),
        "global_flags": globals,
        "commands": commands,
        "filter_grammar": {
            "option": "-o '[!]<type>[:<sub>](>=|<=|=|~)<value>[|<value>...]' — >=,<= numeric; = numeric or exact text; ~ contains; leading ! negates; | ORs values (\\| literal pipe); repeat = AND",
            "enchant": "--enchant '[접두|접미:]<name>[(>=|<=|=)<rank>]'",
            "enchant_rank": "--enchant-rank '[접두|접미](>=|<=|=)<rank>[,(>=|<=)<rank>]' — order F<E<D<C<B<A<9<8<...<1",
            "reforge": "--reforge '[<slot 1-3>:]<option>[(>=|<=|=)<level>]' — any slot unless given",
            "erg": "--erg '<grade S|A|B>[(>=|<=|=)<level>]'",
            "exclude_blueprints": "--exclude-blueprints drops names containing 도면/옷본/심볼",
            "combine": "all flags are ANDed; unknown option type -> exit 2, unknown enchant or reforge option name -> exit 3 (checked before any API call), with similar names in error.hint/message",
        },
        "option_families": repo.option_families,
        "enchant_ranks": repo.enchant_ranks,
        "enchant_slots": repo.enchant_slots,
        "equipment_types": repo.equipment_types,
        "catalog_sources": ["repo_fact", "labanyu_reference", "repo_observed", "local_observed", "user_import"],
        "catalog_source_precedence": "on equal dates: user_import > local_observed > repo_observed > labanyu_reference > repo_fact",
        "reference_kinds": crate::repo_data::LIST_KINDS,
        "exit_codes": {
            "0": "ok", "1": "internal", "2": "usage", "3": "not found", "4": "key/permission",
            "5": "rate/budget exceeded", "6": "nexon maintenance/outage", "7": "store error",
        },
        "examples": EXAMPLES,
    })
}

pub fn run(ctx: &mut Context, _args: &DescribeArgs) -> Result<Value, AppError> {
    Ok(describe(&crate::Cli::command(), &ctx.repo()?))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn today() -> time::Date {
        time::Date::from_calendar_date(2026, time::Month::November, 11).unwrap()
    }

    #[test]
    fn describe_matches_clap_definition() {
        let root = crate::Cli::command();
        let d = describe(&root, &RepoData::load(today()).unwrap());
        let cmds = d["commands"].as_array().unwrap();
        let all = leaves(&root);
        assert_eq!(cmds.len(), all.len());
        let mut no_help = Vec::new();
        for (name, c) in &all {
            let entry = cmds
                .iter()
                .find(|e| e["name"] == name.as_str())
                .unwrap_or_else(|| panic!("describe misses command {name}"));
            assert!(
                !entry["examples"].as_array().unwrap().is_empty(),
                "no example for {name}"
            );
            let flags = entry["flags"].as_array().unwrap();
            for a in visible_args(c) {
                assert!(
                    flags.iter().any(|f| f["id"] == a.get_id().as_str()),
                    "describe misses {name} {}",
                    a.get_id()
                );
                if a.get_help().is_none() && !a.is_positional() {
                    no_help.push(format!("{name} {}", a.get_id()));
                }
            }
        }
        assert!(no_help.is_empty(), "flags without help: {no_help:?}");
        for a in root
            .get_arguments()
            .filter(|a| a.is_global_set() && !a.is_hide_set())
        {
            assert!(
                d["global_flags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["id"] == a.get_id().as_str())
            );
        }
        for key in ["option", "enchant", "enchant_rank", "reforge", "erg"] {
            assert!(d["filter_grammar"][key].is_string(), "{key}");
        }
        assert!(!d["option_families"].as_array().unwrap().is_empty());
    }

    #[test]
    fn every_example_names_a_real_command() {
        let names: Vec<String> = leaves(&crate::Cli::command())
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        for e in EXAMPLES {
            let rest = e.strip_prefix("mabi-ah ").unwrap();
            assert!(
                names
                    .iter()
                    .any(|n| rest == n || rest.starts_with(&format!("{n} "))),
                "example for unknown command: {e}"
            );
        }
    }
}
