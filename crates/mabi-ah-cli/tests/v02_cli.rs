#![allow(clippy::unwrap_used, clippy::expect_used)]
//! v0.2 CLI: family filter flags, price_ko, blueprint exclusion, catalog,
//! option-prices, describe, name suggestions. Real binary against a fake server.
use std::process::Command;
use std::sync::{Arc, Mutex};

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
    fn ok(&self) -> Value {
        assert_eq!(
            self.code, 0,
            "stdout: {} stderr: {}",
            self.stdout, self.stderr
        );
        self.json()
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

fn opt(t: &str, sub: Option<&str>, v: &str, v2: Option<&str>) -> Value {
    json!({"option_type": t, "option_sub_type": sub, "option_value": v, "option_value2": v2})
}

fn item(name: &str, price: u64, options: Vec<Value>) -> Value {
    json!({
        "item_name": name, "item_display_name": name, "item_count": 1,
        "auction_item_category": "한손 검", "auction_price_per_unit": price,
        "date_auction_expire": "2026-11-13T11:43:00.000Z",
        "item_option": options
    })
}

/// A rich sword (enchant E, reforge 18, erg S 45, red) and a plain blueprint.
fn fixture() -> Vec<Value> {
    vec![
        item(
            "@롱 소드",
            150_000_000,
            vec![
                opt("인챈트", Some("접두"), "날카로운 (랭크 E)", None),
                opt("세공 옵션", Some("1"), "최대 공격력(18레벨:18 증가)", None),
                opt("에르그", Some("S"), "45", Some("50")),
                opt("색상", Some("파트 A"), "빨강", None),
            ],
        ),
        item(
            "롱 소드 도면",
            7490,
            vec![opt("색상", Some("파트 A"), "초록", None)],
        ),
    ]
}

fn body(items: &[Value], next: Option<&str>) -> String {
    json!({"auction_item": items, "next_cursor": next}).to_string()
}

fn names(v: &Value) -> Vec<String> {
    v["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["item_name"].as_str().unwrap().to_owned())
        .collect()
}

fn fake() -> Env {
    let (base, _) = serve(Box::new(|_| (200, body(&fixture(), None))));
    Env::new(&base)
}

const SWORD: &str = "@롱 소드";

#[test]
fn family_filter_flags_match_and_reject() {
    let env = fake();
    let cases: &[(&[&str], &[&str])] = &[
        (&["--enchant", "접두:날카로운"], &[SWORD]),
        (&["--enchant", "접미:날카로운"], &[]),
        (&["--enchant-rank", ">=E"], &[SWORD]),
        (&["--enchant-rank", ">=A"], &[]),
        (&["--reforge", "최대 공격력>=18"], &[SWORD]),
        (&["--reforge", "최대 공격력>=19"], &[]),
        (&["--reforge", "1:최대 공격력>=18"], &[SWORD]),
        (&["--reforge", "2:최대 공격력>=18"], &[]),
        (&["--erg", "S>=40"], &[SWORD]),
        (&["--erg", "S>=46"], &[]),
        (&["-o", "!인챈트 불가능=true"], &[SWORD, "롱 소드 도면"]),
        (&["-o", "색상~빨강|파랑"], &[SWORD]),
        (&["-o", "!색상~빨강"], &["롱 소드 도면"]),
        (&["--exclude-blueprints"], &[SWORD]),
    ];
    for (flags, want) in cases {
        let mut args = vec!["listings", "--category", "한손 검"];
        args.extend_from_slice(flags);
        let v = env.run(&args).ok();
        assert_eq!(names(&v), *want, "flags {flags:?}");
    }
    let args = [
        "search",
        "롱",
        "소드",
        "--erg",
        "S>=40",
        "--exclude-blueprints",
    ];
    assert_eq!(names(&env.run(&args).ok()), [SWORD]);
}

#[test]
fn trades_accept_family_flags() {
    let (base, _) = serve(Box::new(|_| {
        let mut t = fixture()[0].clone();
        t["auction_buy_id"] = json!("b1");
        t["date_auction_buy"] = json!("2026-11-11T01:00:00.000Z");
        (
            200,
            json!({"auction_history": [t], "next_cursor": null}).to_string(),
        )
    }));
    let env = Env::new(&base);
    let v = env
        .run(&["trades", "--category", "한손 검", "--enchant-rank", ">=E"])
        .ok();
    assert_eq!(names(&v), [SWORD]);
    assert_eq!(v["data"]["items"][0]["price_ko"], "1억5000만");
}

#[test]
fn items_carry_price_ko() {
    let env = fake();
    let v = env.run(&["listings", "--category", "한손 검"]).ok();
    let items = v["data"]["items"].as_array().unwrap();
    assert_eq!(items[0]["price_ko"], "1억5000만");
    assert_eq!(items[1]["price_ko"], "7,490");
}

#[test]
fn bad_filter_flag_is_usage_error() {
    let env = fake();
    for flags in [
        ["--erg", "SS>=40"],
        ["--enchant-rank", ">=Q"],
        ["--reforge", ""],
    ] {
        let mut args = vec!["listings", "--category", "한손 검"];
        args.extend_from_slice(&flags);
        let r = env.run(&args);
        assert_eq!(r.code, 2, "flags {flags:?}: {}", r.stdout);
        assert_eq!(r.json()["error"]["code"], "usage");
    }
}

#[test]
fn listings_record_catalog_and_catalog_commands_read_it() {
    let env = fake();
    env.run(&["listings", "--category", "한손 검"]).ok();

    let v = env.run(&["catalog", "enchant", "--query", "날카"]).ok();
    let rows = v["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{v}");
    assert_eq!(rows[0]["name"], "날카로운");
    assert_eq!(rows[0]["slot"], "접두");
    assert_eq!(rows[0]["rank"], "E");
    assert_eq!(rows[0]["source"], "local_observed");
    assert!(rows[0]["observed_at"].is_string(), "{v}");

    let v = env
        .run(&["catalog", "reforge", "--category", "한손 검"])
        .ok();
    let rows = v["data"]["rows"].as_array().unwrap();
    assert_eq!(rows[0]["option"], "최대 공격력");
    assert_eq!(rows[0]["max_level"], 18);
    assert_eq!(rows[0]["source"], "local_observed");

    let v = env.run(&["catalog", "option-types"]).ok();
    let rows = v["data"]["rows"].as_array().unwrap();
    assert!(rows.iter().any(|r| r["option_type"] == "에르그"), "{v}");
    assert!(
        rows.iter()
            .any(|r| r["option_type"] == "세공 옵션" && r["source"] == "repo_fact"),
        "{v}"
    );
    assert!(rows.iter().all(|r| r.get("observed_at").is_some()), "{v}");

    let v = env.run(&["catalog", "values", "--type", "에르그"]).ok();
    let rows = v["data"]["rows"].as_array().unwrap();
    assert_eq!(rows[0]["value"], "S");
    assert_eq!(rows[0]["source"], "local_observed");
}

#[test]
fn enchant_typo_is_not_found_with_suggestion() {
    let env = fake();
    env.run(&["listings", "--category", "한손 검"]).ok();
    let r = env.run(&[
        "listings",
        "--category",
        "한손 검",
        "--enchant",
        "접두:날카론운",
    ]);
    assert_eq!(r.code, 3, "{}", r.stdout);
    let hint = r.json()["error"]["hint"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(hint.contains("날카로운"), "hint: {hint}");
}

#[test]
fn unknown_option_type_is_usage_with_suggestion() {
    let env = fake();
    let r = env.run(&["listings", "--category", "한손 검", "-o", "세공옵션>=1"]);
    assert_eq!(r.code, 2, "{}", r.stdout);
    assert!(r.stdout.contains("세공 옵션"), "{}", r.stdout);
}

#[test]
fn catalog_learn_respects_max_calls() {
    let (base, seen) = serve(Box::new(|_| (200, body(&fixture(), Some("next")))));
    let env = Env::new(&base);
    let v = env
        .run(&["catalog", "learn", "--category", "장갑", "--max-calls", "3"])
        .ok();
    assert!(seen.lock().unwrap().len() <= 3);
    assert_eq!(v["data"]["calls"], 3);
    assert_eq!(v["data"]["truncated"], true);
    assert!(v["data"]["observations"].as_u64().unwrap() > 0);
}

#[test]
fn catalog_import_rows_are_user_import() {
    let env = fake();
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
    let v = env.run(&["catalog", "import", path.to_str().unwrap()]).ok();
    assert_eq!(v["data"]["enchants"], 1, "{v}");
    let v = env.run(&["catalog", "enchant", "--query", "테스트"]).ok();
    assert_eq!(v["data"]["rows"][0]["source"], "user_import");
    assert_eq!(v["data"]["rows"][0]["observed_at"], d(-1).as_str());

    let bad = env.dir.path().join("bad.json");
    std::fs::write(&bad, "{\"price\": 1}").unwrap();
    assert_eq!(
        env.run(&["catalog", "import", bad.to_str().unwrap()]).code,
        2
    );
}

#[test]
fn option_prices_table_over_two_pages() {
    let (base, seen) = serve(Box::new(|url| {
        if url.contains("cursor=p2") {
            let it = item(
                "@롱 소드",
                900,
                vec![opt(
                    "세공 옵션",
                    Some("2"),
                    "최대 공격력(18레벨:18 증가)",
                    None,
                )],
            );
            (200, body(&[it], None))
        } else {
            (200, body(&fixture(), Some("p2")))
        }
    }));
    let env = Env::new(&base);
    let v = env
        .run(&[
            "option-prices",
            "--category",
            "한손 검",
            "--family",
            "세공 옵션",
            "--max-pages",
            "2",
        ])
        .ok();
    assert_eq!(seen.lock().unwrap().len(), 2);
    let d = &v["data"];
    assert_eq!(d["pages"], 2);
    assert_eq!(d["truncated"], false);
    assert_eq!(d["rows"][0]["value"], "최대 공격력");
    assert_eq!(d["rows"][0]["level"], "18");
    assert_eq!(d["rows"][0]["min_price"], 900);
    assert_eq!(d["rows"][0]["min_price_ko"], "900");
    assert_eq!(d["rows"][0]["count"], 2);
}

#[test]
fn describe_lists_commands_flags_grammar_families() {
    let env = Env::new("http://127.0.0.1:9");
    let v = env.run(&["describe"]).ok();
    let d = &v["data"];
    let cmds: Vec<&str> = d["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    for want in [
        "listings",
        "trades",
        "search",
        "collect",
        "catalog enchant",
        "catalog reforge",
        "catalog option-types",
        "catalog values",
        "catalog learn",
        "catalog import",
        "option-prices",
        "describe",
        "watch add",
    ] {
        assert!(cmds.contains(&want), "missing {want}: {cmds:?}");
    }
    let listings = d["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "listings")
        .unwrap();
    let flags: Vec<&str> = listings["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["long"].as_str().unwrap())
        .collect();
    for f in [
        "--enchant",
        "--enchant-rank",
        "--reforge",
        "--erg",
        "--exclude-blueprints",
    ] {
        assert!(flags.contains(&f), "{flags:?}");
    }
    assert!(d["filter_grammar"]["option"].is_string());
    assert!(d["filter_grammar"]["enchant"].is_string());
    assert!(
        d["option_families"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["option_type"] == "세공 옵션")
    );
    assert!(!d["examples"].as_array().unwrap().is_empty());
    assert!(d["exit_codes"].is_object());
}

#[test]
fn collect_records_catalog_observations() {
    let (base, _) = serve(Box::new(|_| {
        let mut t = fixture()[0].clone();
        t["auction_buy_id"] = json!("c1");
        let at = OffsetDateTime::now_utc() - Duration::minutes(5);
        t["date_auction_buy"] = json!(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00.000Z",
            at.year(),
            u8::from(at.month()),
            at.day(),
            at.hour(),
            at.minute()
        ));
        (
            200,
            json!({"auction_history": [t], "next_cursor": null}).to_string(),
        )
    }));
    let env = Env::new(&base);
    env.run(&["collect", "--max-calls", "1"]).ok();
    let v = env.run(&["catalog", "enchant"]).ok();
    assert_eq!(v["data"]["rows"][0]["name"], "날카로운", "{v}");
}
