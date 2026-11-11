#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Query commands end to end: the real binary against a tiny_http fake server.
use std::process::Command;
use std::sync::{Arc, Mutex};

use mabi_ah_app::ports::BudgetLedger;
use mabi_ah_domain::attribution::ATTRIBUTION;
use mabi_ah_store::SqliteBudget;
use serde_json::{Value, json};
use tempfile::TempDir;
use time::OffsetDateTime;

const KEY: &str = "test_KEY_0123456789abcdefghijklmnopqrstuvwxyz_ZZ";

type Handler = Box<dyn Fn(&str) -> (u16, String) + Send>;

fn serve(handler: Handler) -> (String, Arc<Mutex<Vec<String>>>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for rq in server.incoming_requests() {
            let url = rq.url().to_owned();
            log.lock().unwrap().push(url.clone());
            let (status, body) = handler(&url);
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap();
            let _ = rq.respond(
                tiny_http::Response::from_string(body)
                    .with_status_code(status)
                    .with_header(header),
            );
        }
    });
    (base, seen)
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {:?}", self.stdout))
    }
}

struct Env {
    dir: TempDir,
    base: String,
}

impl Env {
    fn new(base: &str) -> Self {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("key"), format!("{KEY}\n")).unwrap();
        Self {
            dir,
            base: base.to_owned(),
        }
    }
    fn db(&self) -> std::path::PathBuf {
        self.dir.path().join("t.db")
    }
    fn run(&self, args: &[&str]) -> Run {
        let out = Command::new(env!("CARGO_BIN_EXE_mabi-ah"))
            .env_remove("NEXON_OPEN_API_KEY")
            .env_remove("MABI_AH_BASE_URL")
            .arg("--db")
            .arg(self.db())
            .arg("--key-file")
            .arg(self.dir.path().join("key"))
            .arg("--base-url")
            .arg(&self.base)
            .args(args)
            .output()
            .unwrap();
        Run {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

fn listing(name: &str, price: u64) -> Value {
    json!({
        "item_name": name, "item_display_name": name, "item_count": 2,
        "auction_item_category": "검", "auction_price_per_unit": price,
        "date_auction_expire": "2026-11-13T11:43:00.000Z",
        "item_option": [{"option_type": "공격", "option_value": "10", "option_value2": "20"}]
    })
}

fn listing_body(items: &[Value]) -> String {
    json!({"auction_item": items, "next_cursor": null}).to_string()
}

fn err_body(name: &str) -> String {
    json!({"error": {"name": name, "message": "x"}}).to_string()
}

#[test]
fn listings_envelope_and_stats() {
    let (base, seen) = serve(Box::new(|_| {
        (
            200,
            listing_body(&[listing("@롱 소드", 300), listing("@롱 소드", 100)]),
        )
    }));
    let env = Env::new(&base);
    let r = env.run(&["listings", "--name", "@롱 소드", "--stats"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(r.stderr.is_empty(), "stderr: {}", r.stderr);
    let v = r.json();
    assert_eq!(v["schema"], "mabi_ah/v1");
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "listings");
    assert!(v["error"].is_null());
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(v["data"]["items"][0]["base_name"], "롱 소드");
    assert_eq!(v["data"]["items"][0]["total_price"], 600);
    assert_eq!(v["data"]["stats"]["min"], 100);
    assert_eq!(v["meta"]["calls_used"], 1);
    assert_eq!(v["meta"]["budget"]["tier"], "development");
    assert_eq!(v["meta"]["budget"]["used_today"], 1);
    assert_eq!(v["meta"]["budget"]["limit_today"], 900);
    assert_eq!(v["meta"]["staleness"]["stale"], true);
    assert!(v["meta"]["staleness"]["newest_trade_at"].is_null());
    assert_eq!(v["meta"]["attribution"], ATTRIBUTION);
    assert!(v["meta"]["fetched_at"].as_str().unwrap().ends_with('Z'));
    assert!(!r.stdout.contains(KEY));
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn listings_option_filter_applies() {
    let (base, _) = serve(Box::new(|_| {
        (
            200,
            listing_body(&[listing("@a", 5), {
                let mut l = listing("@b", 6);
                l["item_option"] = json!([{"option_type": "공격", "option_value": "1", "option_value2": "2"}]);
                l
            }]),
        )
    }));
    let env = Env::new(&base);
    let r = env.run(&["listings", "--name", "@a", "--option", "공격>=15"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let v = r.json();
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(v["data"]["items"][0]["item_name"], "@a");
}

#[test]
fn trades_and_search_and_horn_and_npc_shop() {
    let (base, _) = serve(Box::new(|url| {
        if url.contains("/auction/history") {
            (
                200,
                json!({"auction_history": [{
                    "auction_buy_id": "t1", "item_name": "@롱 소드", "item_display_name": "롱 소드",
                    "item_count": 1, "auction_item_category": "검", "auction_price_per_unit": 500,
                    "date_auction_buy": "2026-11-11T01:02:03.000Z", "item_option": null
                }], "next_cursor": null})
                .to_string(),
            )
        } else if url.contains("keyword-search") {
            (200, listing_body(&[listing("@롱 소드", 7)]))
        } else if url.contains("npcshop") {
            (
                200,
                json!({"shop_tab_count": 1, "shop": [{"tab_name": "탭", "item": [{
                    "item_display_name": "물약", "item_count": 3, "item_option": [],
                    "price": [{"price_type": "Gold", "price_value": 100}]}]}],
                    "date_inquire": "2026-11-11T00:00:00.000Z",
                    "date_shop_next_update": "2026-11-11T00:36:00.000Z"})
                .to_string(),
            )
        } else {
            (
                200,
                json!({"horn_bugle_world_history": [{
                    "character_name": "테스트캐릭터", "message": "안녕",
                    "date_send": "2026-11-11T08:28:35.000Z"}]})
                .to_string(),
            )
        }
    }));
    let env = Env::new(&base);

    let r = env.run(&["trades", "--name", "@롱 소드"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(r.json()["data"]["items"][0]["trade_id"], "t1");

    let r = env.run(&["search", "롱", "소드"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(r.json()["data"]["items"].as_array().unwrap().len(), 1);

    let r = env.run(&["npc-shop", "--npc", "테스트상인", "--server", "류트"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(r.json()["data"]["tabs"][0]["items"][0]["prices"][0]["value"], 100);

    let r = env.run(&["horn", "--server", "류트"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(r.json()["data"]["messages"][0]["character_name"], "테스트캐릭터");

    let r = env.run(&["horn", "--server", "류트", "--redact-names"]);
    assert_eq!(r.json()["data"]["messages"][0]["character_name"], "***");
    assert!(!r.stdout.contains("테스트캐릭터"));
}

#[test]
fn unknown_name_exits_3_with_hint() {
    let (base, _) = serve(Box::new(|url| {
        if url.contains("keyword-search") {
            (200, listing_body(&[listing("@롱 소드", 7)]))
        } else {
            (400, err_body("OPENAPI00004"))
        }
    }));
    let env = Env::new(&base);
    let r = env.run(&["listings", "--name", "롱소드"]);
    assert_eq!(r.code, 3);
    let v = r.json();
    assert_eq!(v["ok"], false);
    assert!(v["data"].is_null());
    assert_eq!(v["error"]["code"], "not_found");
    assert_eq!(v["error"]["retryable"], false);
    assert!(v["error"]["hint"].as_str().unwrap().contains("롱 소드"));
    assert!(!v["error"]["message_ko"].as_str().unwrap().is_empty());
    assert!(!r.stderr.is_empty(), "errors are also logged to stderr");
    assert!(!r.stderr.contains(KEY) && !r.stdout.contains(KEY));
}

#[test]
fn budget_cap_exits_5_and_force_budget_lifts_it() {
    let (base, seen) = serve(Box::new(|_| (200, listing_body(&[listing("@a", 1)]))));
    let env = Env::new(&base);
    // Opens (and migrates) the DB, then fills today's usage up to the 90% cap.
    mabi_ah_store::SqliteStore::open(&env.db()).unwrap();
    let mut ledger = SqliteBudget::open(&env.db()).unwrap();
    for _ in 0..900 {
        ledger.record_call(OffsetDateTime::now_utc()).unwrap();
    }
    let r = env.run(&["listings", "--name", "@a"]);
    assert_eq!(r.code, 5, "stderr: {}", r.stderr);
    let v = r.json();
    assert_eq!(v["error"]["code"], "budget_exceeded");
    assert_eq!(v["meta"]["budget"]["used_today"], 900);
    assert!(seen.lock().unwrap().is_empty(), "no request may be sent");

    let r = env.run(&["--force-budget", "listings", "--name", "@a"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let v = r.json();
    assert_eq!(v["meta"]["budget"]["limit_today"], 1000);
    assert_eq!(v["meta"]["calls_used"], 1);
}

#[test]
fn maintenance_exits_6_retryable() {
    let (base, _) = serve(Box::new(|_| (503, err_body("OPENAPI00011"))));
    let env = Env::new(&base);
    let r = env.run(&["listings", "--name", "@a"]);
    assert_eq!(r.code, 6, "stderr: {}", r.stderr);
    let v = r.json();
    assert_eq!(v["error"]["code"], "upstream_api_maintenance");
    assert_eq!(v["error"]["nexon_code"], "OPENAPI00011");
    assert_eq!(v["error"]["retryable"], true);
}

#[test]
fn usage_errors_exit_2_as_envelope() {
    let (base, seen) = serve(Box::new(|_| (200, listing_body(&[]))));
    let env = Env::new(&base);
    let r = env.run(&["listings"]);
    assert_eq!(r.code, 2);
    assert_eq!(r.json()["error"]["code"], "usage");
    let r = env.run(&["listings", "--name", "@a", "--option", "공격"]);
    assert_eq!(r.code, 2);
    let r = env.run(&["no-such-command"]);
    assert_eq!(r.code, 2);
    assert_eq!(r.json()["ok"], false);
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn missing_key_exits_4() {
    let (base, _) = serve(Box::new(|_| (200, listing_body(&[]))));
    let env = Env::new(&base);
    let out = Command::new(env!("CARGO_BIN_EXE_mabi-ah"))
        .env_remove("NEXON_OPEN_API_KEY")
        .arg("--db")
        .arg(env.db())
        .arg("--key-file")
        .arg(env.dir.path().join("missing-key"))
        .arg("--base-url")
        .arg(&base)
        .args(["listings", "--name", "@a"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
}

#[test]
fn table_format_is_plain_text() {
    let (base, _) = serve(Box::new(|_| (200, listing_body(&[listing("@롱 소드", 300)]))));
    let env = Env::new(&base);
    let r = env.run(&["--format", "table", "listings", "--name", "@롱 소드"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(serde_json::from_str::<Value>(&r.stdout).is_err());
    assert!(r.stdout.contains("unit_price"));
    assert!(r.stdout.contains("300"));
}

#[test]
fn snapshot_listings_envelope() {
    let (base, _) = serve(Box::new(|_| {
        (200, listing_body(&[listing("@롱 소드", 300), listing("@롱 소드", 100)]))
    }));
    let env = Env::new(&base);
    let r = env.run(&["listings", "--name", "@롱 소드", "--stats"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let mut v = r.json();
    v["meta"]["fetched_at"] = json!("[timestamp]");
    insta::assert_json_snapshot!(v);
}
