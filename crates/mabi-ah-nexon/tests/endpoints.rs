#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Endpoint adapters against a tiny_http fake server.
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mabi_ah_app::error::{AppError, UpstreamKind};
use mabi_ah_app::ports::{AuctionGateway, CallMeter, HornGateway, ListingQuery, NpcShopGateway};
use mabi_ah_nexon::client::NexonClient;
use mabi_ah_nexon::key::ApiKey;
use mabi_ah_nexon::pager::collect_pages;
use mabi_ah_nexon::rate_limit::{Clock, Sleeper, TokenBucket};
use mabi_ah_nexon::retry::RetryPolicy;
use serde_json::{Value, json};

const KEY: &str = "test_KEY_0123456789abcdefghijklmnopqrstuvwxyz_ZZ";

struct Zero;
impl Clock for Zero {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}
struct NoSleep;
impl Sleeper for NoSleep {
    fn sleep(&self, _d: Duration) {}
}

#[derive(Default)]
struct Count(Cell<u32>);
impl CallMeter for Count {
    fn before_request(&self) -> Result<(), AppError> {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Seen {
    url: String,
    key: Option<String>,
}

type Handler = Box<dyn Fn(&str, usize) -> (u16, String) + Send>;

fn serve(handler: Handler) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for (n, rq) in server.incoming_requests().enumerate() {
            let url = rq.url().to_owned();
            let key = rq
                .headers()
                .iter()
                .find(|h| h.field.equiv("x-nxopen-api-key"))
                .map(|h| h.value.as_str().to_owned());
            log.lock().unwrap().push(Seen {
                url: url.clone(),
                key,
            });
            let (status, body) = handler(&url, n);
            let resp = tiny_http::Response::from_string(body)
                .with_status_code(status)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                );
            let _ = rq.respond(resp);
        }
    });
    (base, seen)
}

fn client(base: &str, attempts: u32) -> (NexonClient<Zero, NoSleep>, Rc<Count>) {
    let meter = Rc::new(Count::default());
    let c = NexonClient::with_parts(
        base,
        ApiKey::new(KEY).unwrap(),
        TokenBucket::new(1000, 1000, Zero, NoSleep),
        RetryPolicy {
            max_attempts: attempts,
            base_ms: 1,
            cap_ms: 2,
        },
        meter.clone(),
        Box::new(NoSleep),
    );
    (c, meter)
}

fn err_body(name: &str, message: &str) -> String {
    json!({"error": {"name": name, "message": message}}).to_string()
}

fn listing(name: &str, price: u64) -> Value {
    json!({
        "item_name": name, "item_display_name": name, "item_count": 2,
        "auction_item_category": "미지의 카테고리", "auction_price_per_unit": price,
        "date_auction_expire": "2026-11-13T11:43:00.000Z", "item_option": null,
        "extra_unknown_field": 1
    })
}

fn trade(id: &str) -> Value {
    json!({
        "auction_buy_id": id, "item_name": "@롱 소드", "item_display_name": "롱 소드",
        "item_count": 1, "auction_item_category": "검", "auction_price_per_unit": 500,
        "date_auction_buy": "2026-11-11T01:02:03.000Z",
        "item_option": [{"option_type": "공격", "option_value": "10", "option_value2": "20"}]
    })
}

fn cursor_of(url: &str) -> Option<String> {
    url.split(['?', '&'])
        .find_map(|p| p.strip_prefix("cursor=").map(str::to_owned))
}

#[test]
fn cursor_walk_three_pages() {
    let (base, seen) = serve(Box::new(|url, _| {
        let (items, next) = match cursor_of(url).as_deref() {
            None => (vec![listing("a", 1)], json!("c2")),
            Some("c2") => (vec![listing("b", 2)], json!("c3")),
            _ => (vec![listing("c", 3)], Value::Null),
        };
        (
            200,
            json!({"auction_item": items, "next_cursor": next}).to_string(),
        )
    }));
    let (mut c, meter) = client(&base, 3);
    let q = ListingQuery {
        name: Some("롱 소드".into()),
        ..ListingQuery::default()
    };
    let paged = collect_pages(None, 10, |cur| {
        c.listings(&ListingQuery {
            cursor: cur.map(str::to_owned),
            ..q.clone()
        })
    })
    .unwrap();
    assert_eq!(paged.items.len(), 3);
    assert!(!paged.truncated);
    assert_eq!(paged.items[0].category, "미지의 카테고리");
    assert_eq!(meter.0.get(), 3);
    let s = seen.lock().unwrap();
    assert!(s[0].url.starts_with("/mabinogi/v1/auction/list?"));
    assert!(s[0].url.contains("item_name=%EB%A1%B1"), "{}", s[0].url);
    assert!(s[2].url.contains("cursor=c3"));
}

#[test]
fn max_pages_marks_truncated() {
    let (base, _) = serve(Box::new(|_, n| {
        (
            200,
            json!({"auction_item": [listing("a", 1)], "next_cursor": format!("more{n}")})
                .to_string(),
        )
    }));
    let (mut c, _) = client(&base, 3);
    let paged = collect_pages(None, 2, |cur| {
        c.listings(&ListingQuery {
            cursor: cur.map(str::to_owned),
            ..ListingQuery::default()
        })
    })
    .unwrap();
    assert_eq!(paged.items.len(), 2);
    assert!(paged.truncated);
}

#[test]
fn empty_result_is_ok() {
    let (base, _) = serve(Box::new(|_, _| {
        (
            200,
            json!({"auction_item": [], "next_cursor": null}).to_string(),
        )
    }));
    let (mut c, _) = client(&base, 3);
    let page = c.listings(&ListingQuery::default()).unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.next_cursor, None);
}

#[test]
fn trades_and_search_paths() {
    let (base, seen) = serve(Box::new(|url, _| {
        if url.contains("history") {
            (
                200,
                json!({"auction_history": [trade("T1")], "next_cursor": null}).to_string(),
            )
        } else {
            (
                200,
                json!({"auction_item": [listing("x", 5)], "next_cursor": null}).to_string(),
            )
        }
    }));
    let (mut c, _) = client(&base, 3);
    let t = c.trades(&ListingQuery::default()).unwrap();
    assert_eq!(t.items[0].trade_id, "T1");
    assert_eq!(t.items[0].name.base, "롱 소드");
    assert_eq!(t.items[0].options.len(), 1);
    c.search(&["숏".to_owned(), "소드".to_owned()], None)
        .unwrap();
    let s = seen.lock().unwrap();
    assert!(s[0].url.starts_with("/mabinogi/v1/auction/history"));
    assert!(
        s[1].url
            .starts_with("/mabinogi/v1/auction/keyword-search?keyword=")
    );
    assert!(
        s[1].url.contains("%2C") || s[1].url.contains(','),
        "{}",
        s[1].url
    );
}

#[test]
fn npc_shop_and_horn() {
    let (base, seen) = serve(Box::new(|url, _| {
        if url.contains("npcshop") {
            (
                200,
                json!({
                    "shop_tab_count": 1,
                    "shop": [{"tab_name": "탭", "item": [{
                        "item_display_name": "물약", "item_count": 3,
                        "item_option": [], "price": [{"price_type": "Gold", "price_value": 100}],
                        "limit_type": "주간", "limit_value": 5, "image_url": "x"
                    }]}],
                    "date_inquire": "2026-11-11T00:00:00.000Z",
                    "date_shop_next_update": "2026-11-11T00:36:00.000Z"
                })
                .to_string(),
            )
        } else {
            (
                200,
                json!({"horn_bugle_world_history": [{
                    "character_name": "테스트캐릭터", "message": "안녕",
                    "date_send": "2026-11-11T08:28:35.000Z"
                }]})
                .to_string(),
            )
        }
    }));
    let (mut c, _) = client(&base, 3);
    let shop = c.npc_shop("테스트상인", "류트", 1).unwrap();
    assert_eq!(shop.npc, "테스트상인");
    assert_eq!(shop.channel, 1);
    assert_eq!(shop.tabs[0].items[0].prices[0].value, 100);
    assert_eq!(shop.tabs[0].items[0].limit_value, Some(5));
    assert!(shop.next_update_at.is_some());
    let h = c.horn("류트").unwrap();
    assert_eq!(h[0].message, "안녕");
    let s = seen.lock().unwrap();
    assert!(
        s[0].url.contains("npc_name=")
            && s[0].url.contains("server_name=")
            && s[0].url.contains("channel=1")
    );
    assert!(
        s[1].url
            .starts_with("/mabinogi/v1/horn-bugle-world/history?server_name=")
    );
}

#[test]
fn bad_name_400_is_not_found() {
    let (base, _) = serve(Box::new(|_, _| {
        (400, err_body("OPENAPI00004", "invalid item name"))
    }));
    let (mut c, meter) = client(&base, 3);
    let e = c.listings(&ListingQuery::default()).unwrap_err();
    assert!(matches!(e, AppError::NotFound { .. }), "{e:?}");
    assert_eq!(meter.0.get(), 1);
}

#[test]
fn bad_cursor_00003_is_usage() {
    let (base, _) = serve(Box::new(|_, _| {
        (400, err_body("OPENAPI00003", "bad cursor"))
    }));
    let (mut c, meter) = client(&base, 3);
    let e = c.listings(&ListingQuery::default()).unwrap_err();
    assert!(matches!(e, AppError::Usage(_)), "{e:?}");
    assert_eq!(meter.0.get(), 1);
}

#[test]
fn rate_limit_twice_then_success() {
    let (base, _) = serve(Box::new(|_, n| {
        if n < 2 {
            (429, err_body("OPENAPI00007", "slow down"))
        } else {
            (
                200,
                json!({"auction_item": [], "next_cursor": null}).to_string(),
            )
        }
    }));
    let (mut c, meter) = client(&base, 4);
    assert!(c.listings(&ListingQuery::default()).is_ok());
    assert_eq!(meter.0.get(), 3, "meter counts retries");
}

#[test]
fn maintenance_503_is_upstream_after_retries() {
    let (base, _) = serve(Box::new(|_, _| {
        (503, err_body("OPENAPI00011", "maintenance"))
    }));
    let (mut c, meter) = client(&base, 3);
    let e = c.listings(&ListingQuery::default()).unwrap_err();
    match e {
        AppError::Upstream { kind, nexon_code } => {
            assert_eq!(kind, UpstreamKind::ApiMaintenance);
            assert_eq!(nexon_code.as_deref(), Some("OPENAPI00011"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(meter.0.get(), 3);
}

#[test]
fn header_sent_and_key_never_in_error() {
    let (base, seen) = serve(Box::new(|_, _| {
        (400, err_body("OPENAPI00004", &format!("bad {KEY} value")))
    }));
    let (mut c, _) = client(&base, 3);
    let e = c.listings(&ListingQuery::default()).unwrap_err();
    assert_eq!(seen.lock().unwrap()[0].key.as_deref(), Some(KEY));
    let text = format!("{e} {e:?}");
    assert!(!text.contains(KEY), "{text}");
    assert!(!text.contains(&KEY[KEY.len() - 32..]), "{text}");
}

#[test]
fn connection_refused_is_upstream_without_key() {
    let (mut c, meter) = client("http://127.0.0.1:1", 2);
    let e = c.listings(&ListingQuery::default()).unwrap_err();
    assert!(
        matches!(
            e,
            AppError::Upstream {
                kind: UpstreamKind::Internal,
                ..
            }
        ),
        "{e:?}"
    );
    assert_eq!(meter.0.get(), 2);
    assert!(!format!("{e:?}").contains(KEY));
}
