#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `catalog export --to <dir>`: whitelist fields, deterministic output, only
//! local_observed rows, window and expiry. Real binary against a fake server.
use std::process::Command;

use serde_json::{Value, json};
use tempfile::TempDir;
use time::{Duration, OffsetDateTime};

const KEY: &str = "test_KEY_0123456789abcdefghijklmnopqrstuvwxyz_ZZ";

fn opt(t: &str, sub: Option<&str>, v: &str, v2: Option<&str>) -> Value {
    json!({"option_type": t, "option_sub_type": sub, "option_value": v, "option_value2": v2})
}

fn page() -> String {
    let item = json!({
        "item_name": "@롱 소드", "item_display_name": "@롱 소드", "item_count": 1,
        "auction_item_category": "한손 검", "auction_price_per_unit": 150_000_000u64,
        "date_auction_expire": "2026-11-13T11:43:00.000Z",
        "item_option": [
            opt("인챈트", Some("접두"), "날카로운 (랭크 E)", None),
            opt("세공 옵션", Some("1"), "최대 공격력(18레벨:18 증가)", None),
            opt("에르그", Some("S"), "45", Some("50")),
        ]
    });
    json!({"auction_item": [item], "next_cursor": null}).to_string()
}

struct Env {
    dir: TempDir,
    base: String,
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Env {
    fn new() -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        std::thread::spawn(move || {
            for rq in server.incoming_requests() {
                let _ = rq.respond(tiny_http::Response::from_string(page()));
            }
        });
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("key"), format!("{KEY}\n")).unwrap();
        Self { dir, base }
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
    fn ok(&self, args: &[&str]) -> Value {
        let r = self.run(args);
        assert_eq!(r.code, 0, "stdout: {} stderr: {}", r.stdout, r.stderr);
        serde_json::from_str(&r.stdout).unwrap()
    }
}

fn read(dir: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn export_writes_whitelisted_dated_files_deterministically() {
    let env = Env::new();
    env.ok(&["listings", "--category", "한손 검"]);
    let out1 = TempDir::new().unwrap();
    let out2 = TempDir::new().unwrap();
    let v = env.ok(&["catalog", "export", "--to", out1.path().to_str().unwrap()]);
    env.ok(&["catalog", "export", "--to", out2.path().to_str().unwrap()]);
    assert_eq!(v["data"]["files"].as_array().unwrap().len(), 3, "{v}");

    let today = OffsetDateTime::now_utc().date();
    for (file, kind) in [
        ("enchants.json", "observed_enchant"),
        ("reforges.json", "observed_reforge"),
        ("option_values.json", "observed_option_value"),
    ] {
        let text = read(out1.path(), file);
        assert_eq!(text, read(out2.path(), file), "{file} not deterministic");
        assert!(text.ends_with('\n'));
        let f: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(f["kind"], kind);
        assert_eq!(f["attribution"], "Data based on NEXON Open API");
        assert_eq!(f["observed_to"], today.to_string().as_str(), "{file}");
        assert_eq!(
            f["expires_at"],
            (today + Duration::days(30)).to_string().as_str()
        );
        assert!(f["observed_from"].is_string());
        // whitelist: no price/person data anywhere in the file
        let lower = text.to_lowercase();
        for bad in ["price", "seller", "150000000", "character"] {
            assert!(!lower.contains(bad), "{file} contains {bad}");
        }
    }
    let e: Value = serde_json::from_str(&read(out1.path(), "enchants.json")).unwrap();
    assert_eq!(e["entries"][0]["name"], "날카로운");
}

#[test]
fn export_never_reexports_imported_or_reference_rows() {
    let env = Env::new();
    let today = OffsetDateTime::now_utc().date();
    let d = |off: i64| (today + Duration::days(off)).to_string();
    let file = json!({
        "schema_version": 1, "kind": "observed_enchant",
        "attribution": "Data based on NEXON Open API",
        "observed_from": d(-2), "observed_to": d(-1), "expires_at": d(20),
        "entries": [{"slot": "접미", "name": "테스트인챈트", "rank": "6", "effects": [],
                     "observed_count": 2, "first_seen": d(-2), "last_seen": d(-1)}]
    });
    let path = env.dir.path().join("import.json");
    std::fs::write(&path, file.to_string()).unwrap();
    env.ok(&["catalog", "import", path.to_str().unwrap()]);
    let out = TempDir::new().unwrap();
    let v = env.ok(&["catalog", "export", "--to", out.path().to_str().unwrap()]);
    assert_eq!(v["data"]["files"].as_array().unwrap().len(), 0, "{v}");
    assert!(!out.path().join("enchants.json").exists());
}
