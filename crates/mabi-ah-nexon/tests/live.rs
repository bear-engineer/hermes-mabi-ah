#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Live smoke test: at most 6 real HTTP requests; prints counts only.
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::{AuctionGateway, CallMeter, HornGateway, ListingQuery, NpcShopGateway};
use mabi_ah_nexon::client::NexonClient;
use mabi_ah_nexon::key::{ApiKey, default_key_path};

const MAX_CALLS: u32 = 6;

/// Hard cap: refuses (and never sends) request number 7.
#[derive(Default)]
struct Capped(Cell<u32>);
impl CallMeter for Capped {
    fn before_request(&self) -> Result<(), AppError> {
        if self.0.get() >= MAX_CALLS {
            return Err(AppError::BudgetExceeded {
                used: u64::from(self.0.get()),
                limit: u64::from(MAX_CALLS),
            });
        }
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

#[test]
fn live_endpoints_smoke() {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let path = default_key_path(home.as_deref());
    let Ok(key) = ApiKey::load(path.as_deref(), std::env::var("NEXON_OPEN_API_KEY").ok()) else {
        println!("live: skipped (no key)");
        return;
    };
    let meter = Rc::new(Capped::default());
    let mut c = NexonClient::new(key, meter.clone());
    // Pace the calls so a burst never trips the 5 rps limit (a retry would eat the cap).
    let pause = || std::thread::sleep(std::time::Duration::from_millis(1200));

    let q = ListingQuery {
        name: Some("롱 소드".into()),
        ..ListingQuery::default()
    };
    let list = c.listings(&q).unwrap();
    println!(
        "live: auction/list items={} requests={}",
        list.items.len(),
        meter.0.get()
    );
    pause();
    let hist = c.trades(&q).unwrap();
    println!(
        "live: auction/history items={} requests={}",
        hist.items.len(),
        meter.0.get()
    );
    pause();
    let kw = c
        .search(&["롱".to_owned(), "소드".to_owned()], None)
        .unwrap();
    println!(
        "live: keyword-search items={} requests={}",
        kw.items.len(),
        meter.0.get()
    );
    pause();
    match c.npc_shop("델", "류트", 1) {
        Ok(s) => println!(
            "live: npcshop tabs={} requests={}",
            s.tabs.len(),
            meter.0.get()
        ),
        Err(e) => println!("live: npcshop error code={}", e.code()),
    }
    pause();
    let horn = c.horn("류트").unwrap();
    println!("live: horn items={} requests={}", horn.len(), meter.0.get());
    pause();
    let bad = c
        .listings(&ListingQuery {
            name: Some("존재하지않는아이템이름zzzqqq".into()),
            ..ListingQuery::default()
        })
        .unwrap_err();
    println!("live: unknown name -> {}", bad.code());
    assert!(matches!(bad, AppError::NotFound { .. }), "{}", bad.code());
    println!("live: http requests={}", meter.0.get());
    assert!(meter.0.get() <= MAX_CALLS);
}
