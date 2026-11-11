#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Store-backed commands end to end: the real binary, a tiny_http fake server
//! and a temp database.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mabi_ah_domain::attribution::ATTRIBUTION;
use serde_json::{Value, json};
use tempfile::TempDir;
use time::{Duration, OffsetDateTime};

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
    fn run(&self, args: &[&str]) -> Run {
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
        Run {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

fn stamp(t: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

fn trade(id: &str, name: &str, price: u64, minutes_ago: i64) -> Value {
    json!({
        "auction_buy_id": id, "item_name": name, "item_display_name": name.trim_start_matches('@'),
        "item_count": 2, "auction_item_category": "검", "auction_price_per_unit": price,
        "date_auction_buy": stamp(OffsetDateTime::now_utc() - Duration::minutes(minutes_ago)),
        "item_option": null
    })
}

fn history(items: &[Value]) -> String {
    json!({"auction_history": items, "next_cursor": null}).to_string()
}

/// First history request gets t1..t3, later ones t2..t4 (overlap on t2, t3).
fn overlapping_server() -> (String, Arc<Mutex<Vec<String>>>) {
    let n = AtomicUsize::new(0);
    serve(Box::new(move |_| {
        let body = if n.fetch_add(1, Ordering::SeqCst) == 0 {
            history(&[
                trade("t3", "@롱 소드", 300, 5),
                trade("t2", "@롱 소드", 200, 10),
                trade("t1", "@롱 소드", 100, 20),
            ])
        } else {
            history(&[
                trade("t4", "@롱 소드", 400, 1),
                trade("t3", "@롱 소드", 300, 5),
                trade("t2", "@롱 소드", 200, 10),
            ])
        };
        (200, body)
    }))
}

#[test]
fn collect_twice_with_overlap_keeps_unique_rows() {
    let (base, _) = overlapping_server();
    let env = Env::new(&base);
    let r1 = env.run(&["collect"]);
    assert_eq!(r1.code, 0, "stderr: {}", r1.stderr);
    let v1 = r1.json();
    assert_eq!(v1["command"], "collect");
    assert_eq!(v1["data"]["inserted"], 3);
    let r2 = env.run(&["collect"]);
    assert_eq!(r2.code, 0, "stderr: {}", r2.stderr);
    let v2 = r2.json();
    assert_eq!(v2["data"]["fetched"], 3);
    assert_eq!(v2["data"]["inserted"], 1);
    assert_eq!(v2["data"]["reached_watermark"], true);
    let rank = env.run(&["rank", "--window", "24h"]).json();
    assert_eq!(rank["data"]["items"][0]["trades"], 4);
    assert_eq!(rank["data"]["items"][0]["units"], 8);
}

#[test]
fn collect_quiet_prints_nothing_and_releases_lock() {
    let (base, _) = overlapping_server();
    let env = Env::new(&base);
    let r = env.run(&["collect", "--quiet", "--max-calls", "5", "--retain-days", "30"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "");
    assert_eq!(r.stderr, "");
    let again = env.run(&["collect", "--quiet"]);
    assert_eq!(again.code, 0, "stderr: {}", again.stderr);
    assert!(!env.dir.path().join("t.db.collect.lock").exists());
}

#[test]
fn collect_refuses_while_lock_is_held() {
    let (base, seen) = overlapping_server();
    let env = Env::new(&base);
    std::fs::write(env.dir.path().join("t.db.collect.lock"), "").unwrap();
    let r = env.run(&["collect"]);
    assert_eq!(r.code, 7, "stdout: {}", r.stdout);
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn trend_and_rank_json_shape() {
    let (base, _) = overlapping_server();
    let env = Env::new(&base);
    assert_eq!(env.run(&["collect"]).code, 0);

    let t = env.run(&["trend", "--name", "@롱 소드", "--window", "24h", "--bucket", "hour"]);
    assert_eq!(t.code, 0, "stderr: {}", t.stderr);
    let v = t.json();
    assert_eq!(v["command"], "trend");
    assert_eq!(v["data"]["bucket"], "hour");
    assert_eq!(v["data"]["name"], "롱 소드");
    let pts = v["data"]["items"].as_array().unwrap();
    assert!(!pts.is_empty());
    for k in ["start", "trades", "units", "vwap", "min", "max", "has_gap"] {
        assert!(pts[0].get(k).is_some(), "missing {k}");
    }
    let total: u64 = pts.iter().map(|p| p["trades"].as_u64().unwrap()).sum();
    assert_eq!(total, 3);
    assert!(v["data"]["gaps"].is_array());

    let r = env.run(&["rank", "--by", "value", "--window", "24h", "--category", "검", "--top", "5"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    let v = r.json();
    assert_eq!(v["data"]["by"], "value");
    let e = &v["data"]["items"][0];
    assert_eq!(e["rank"], 1);
    assert_eq!(e["base_name"], "롱 소드");
    assert_eq!(e["value"], 1200);
    assert_eq!(e["trades"], 3);

    let bad = env.run(&["rank", "--window", "soon"]);
    assert_eq!(bad.code, 2);
    let bad = env.run(&["trend", "--bucket", "week"]);
    assert_eq!(bad.code, 2);
}

fn listings_server() -> (String, Arc<Mutex<Vec<String>>>) {
    serve(Box::new(|_| {
        (
            200,
            json!({"auction_item": [{
                "item_name": "@롱 소드", "item_display_name": "롱 소드", "item_count": 1,
                "auction_item_category": "검", "auction_price_per_unit": 150,
                "date_auction_expire": "2036-11-13T11:43:00.000Z",
                "item_option": [{"option_type": "공격", "option_value": "10", "option_value2": "20"}]
            }], "next_cursor": null})
            .to_string(),
        )
    }))
}

#[test]
fn watch_add_check_fires_once_then_nothing() {
    let (base, _) = listings_server();
    let env = Env::new(&base);
    let add = env.run(&[
        "watch", "add", "--name", "@롱 소드", "--max-price", "200", "--category", "검",
        "--option", "공격>=5",
    ]);
    assert_eq!(add.code, 0, "stdout: {} stderr: {}", add.stdout, add.stderr);
    let id = add.json()["data"]["id"].as_str().unwrap().to_owned();
    assert!(!id.is_empty());
    assert_eq!(add.json()["command"], "watch add");

    let list = env.run(&["watch", "list"]).json();
    assert_eq!(list["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(list["data"]["items"][0]["id"], id.as_str());
    assert_eq!(list["data"]["items"][0]["max_price"], 200);

    let first = env.run(&["watch", "check", "--emit", "text"]);
    assert_eq!(first.code, 0, "stderr: {}", first.stderr);
    let lines: Vec<&str> = first.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "stdout: {}", first.stdout);
    assert!(lines[0].contains("롱 소드") && lines[0].contains("150") && lines[0].contains(&id));
    assert_eq!(lines[1], ATTRIBUTION);

    let second = env.run(&["watch", "check", "--emit", "text"]);
    assert_eq!(second.code, 0, "stderr: {}", second.stderr);
    assert_eq!(second.stdout, "");

    let json_run = env.run(&["watch", "check", "--emit", "json"]).json();
    assert_eq!(json_run["command"], "watch check");
    assert_eq!(json_run["data"]["count"], 0);
}

#[test]
fn watch_validation_and_rm() {
    let (base, _) = listings_server();
    let env = Env::new(&base);
    let bad = env.run(&["watch", "add", "--name", "롱 소드", "--option", "???"]);
    assert_eq!(bad.code, 2, "stdout: {}", bad.stdout);
    let empty = env.run(&["watch", "add", "--name", "  "]);
    assert_eq!(empty.code, 2);
    let missing = env.run(&["watch", "rm", "nope"]);
    assert_eq!(missing.code, 3);

    let id = env.run(&["watch", "add", "--name", "롱 소드"]).json()["data"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let rm = env.run(&["watch", "rm", &id]);
    assert_eq!(rm.code, 0, "stderr: {}", rm.stderr);
    let list = env.run(&["watch", "list"]).json();
    assert_eq!(list["data"]["count"], 0);
}

#[test]
fn categories_budget_config_and_doctor() {
    let (base, seen) = listings_server();
    let env = Env::new(&base);

    let c = env.run(&["categories"]).json();
    let items = c["data"]["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["name"] == "검" && i["source"] == "spec"));

    let b = env.run(&["budget"]).json();
    assert_eq!(b["data"]["used_today"], 0);
    assert_eq!(b["data"]["limit_today"], 900);
    assert_eq!(b["data"]["tier"], "development");

    let cfg = env.run(&["config"]);
    assert_eq!(cfg.code, 0, "stderr: {}", cfg.stderr);
    assert!(cfg.json()["data"]["db_path"].as_str().unwrap().ends_with("t.db"));

    let d = env.run(&["doctor"]);
    assert_eq!(d.code, 0, "stderr: {}", d.stderr);
    let v = d.json();
    assert_eq!(v["data"]["key_present"], true);
    assert_eq!(v["data"]["budget"]["limit_today"], 900);
    assert_eq!(v["data"]["staleness"]["stale"], true);
    for r in [&d, &cfg] {
        assert!(!r.stdout.contains(KEY) && !r.stderr.contains(KEY));
        assert!(!r.stdout.contains(&KEY[..16]));
    }
    assert!(seen.lock().unwrap().is_empty(), "these commands make no API calls");
}
