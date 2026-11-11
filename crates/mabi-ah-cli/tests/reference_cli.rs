#![allow(clippy::unwrap_used, clippy::expect_used)]
//! v11: Labanyu reference data in the CLI (catalog enchant/reforge/reference,
//! reforge name check before any API call). Real binary, fake server.
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tempfile::TempDir;

const KEY: &str = "test_KEY_0123456789abcdefghijklmnopqrstuvwxyz_ZZ";

struct Env {
    dir: TempDir,
    base: String,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Env {
    fn new() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        std::thread::spawn(move || {
            for rq in server.incoming_requests() {
                log.lock().unwrap().push(rq.url().to_owned());
                let _ = rq.respond(
                    tiny_http::Response::from_string(r#"{"auction_item":[],"next_cursor":null}"#)
                        .with_status_code(200),
                );
            }
        });
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("key"), format!("{KEY}\n")).unwrap();
        Self { dir, base, seen }
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_mabi-ah"))
            .env_remove("NEXON_OPEN_API_KEY")
            .env_remove("MABI_AH_BASE_URL")
            .arg("--db")
            .arg(self.dir.path().join("t.db"))
            .arg("--key-file")
            .arg(self.dir.path().join("key"))
            .arg("--base-url")
            .arg(&self.base)
            .args(args)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let v = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "stdout is not JSON ({e}): {stdout:?} stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            )
        });
        (out.status.code().unwrap_or(-1), v)
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (code, v) = self.run(args);
        assert_eq!(code, 0, "{v}");
        v["data"].clone()
    }
}

fn rows(d: &Value) -> Vec<Value> {
    d["rows"].as_array().cloned().unwrap_or_default()
}

#[test]
fn catalog_enchant_includes_reference_rows_with_curation() {
    let env = Env::new();
    let d = env.ok(&["catalog", "enchant", "--query", "날카", "--limit", "3"]);
    let got = rows(&d);
    assert!(!got.is_empty() && got.len() <= 3, "{d}");
    for r in &got {
        // a same-day embedded observed row (data/observed) wins a tie over the reference
        let src = r["source"].as_str().unwrap();
        assert!(["labanyu_reference", "repo_observed"].contains(&src), "{r}");
        assert!(r["name"].as_str().unwrap().contains("날카"));
        assert!(r["effects"].is_array(), "{r}");
        assert!(r.get("dungeon_grade").is_some(), "{r}");
        assert!(r["equipment_slot_classification"].is_array(), "{r}");
        assert!(r["observed_at"].is_string(), "{r}");
    }
    let all = env.ok(&["catalog", "enchant", "--query", "날카로운"]);
    assert!(
        rows(&all)
            .iter()
            .any(|r| r["name"] == "날카로운" && r["rank"] == "E" && r["slot"] == "접두"),
        "{all}"
    );
}

#[test]
fn catalog_reforge_shows_reference_ranges_with_limit_break() {
    let env = Env::new();
    let d = env.ok(&[
        "catalog",
        "reforge",
        "--category",
        "한손 검",
        "--query",
        "최대 공격력",
    ]);
    let r = rows(&d)
        .into_iter()
        .find(|r| r["option"] == "최대 공격력")
        .unwrap_or_else(|| panic!("{d}"));
    assert_eq!(r["category"], "한손 검");
    assert_eq!(r["source"], "labanyu_reference");
    assert!(r["min_level"].is_u64() && r["max_level"].is_u64(), "{r}");
    assert!(r["limit_break_max_level"].is_u64(), "{r}");
    assert!(r["range_text"].as_str().unwrap().contains("한돌"), "{r}");
}

#[test]
fn catalog_reference_lists_each_kind() {
    let env = Env::new();
    for kind in [
        "auction_item_categories",
        "reforge_equipment_types",
        "reforge_option_names",
        "totem_abilities",
        "echostone_awakening_abilities",
        "murias_relic_effects",
        "set_bonus_names",
    ] {
        let d = env.ok(&["catalog", "reference", "--kind", kind, "--limit", "2"]);
        assert_eq!(d["kind"], kind, "{d}");
        assert_eq!(d["source"], "labanyu_reference");
        assert!(d["source_url"].as_str().unwrap().starts_with("https://"));
        assert!(d["fetched_at"].is_string());
        assert_eq!(rows(&d).len(), 2, "{kind}: {d}");
        assert!(d["total"].as_u64().unwrap() >= 2);
    }
    let d = env.ok(&[
        "catalog",
        "reference",
        "--kind",
        "echostone_awakening_abilities",
        "--query",
        "체력",
    ]);
    assert!(!rows(&d).is_empty());
    for r in rows(&d) {
        assert!(r["name"].as_str().unwrap().contains("체력"));
        assert!(r["max_level"].is_u64());
        assert_eq!(r["source"], "labanyu_reference");
    }
    let (code, _) = env.run(&["catalog", "reference", "--kind", "nope"]);
    assert_eq!(code, 2);
}

#[test]
fn unknown_reforge_name_fails_fast_with_suggestion_and_no_api_call() {
    let env = Env::new();
    let (code, v) = env.run(&[
        "listings",
        "--category",
        "한손 검",
        "--reforge",
        "최대공격력>=18",
    ]);
    assert_eq!(code, 3, "{v}");
    let hint = v["error"]["hint"].as_str().unwrap_or_default();
    assert!(hint.contains("최대 공격력"), "{v}");
    assert_eq!(v["meta"]["calls_used"], 0, "{v}");
    assert!(env.seen.lock().unwrap().is_empty());
    // A known name passes the check and queries the API.
    let (code, v) = env.run(&[
        "listings",
        "--category",
        "한손 검",
        "--reforge",
        "최대 공격력>=18",
    ]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(env.seen.lock().unwrap().len(), 1);
}

#[test]
fn describe_lists_reference_command_and_source() {
    let env = Env::new();
    let d = env.ok(&["describe"]);
    let s = d.to_string();
    assert!(
        s.contains("catalog reference"),
        "describe misses catalog reference"
    );
    assert!(
        d["catalog_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "labanyu_reference")
    );
}
