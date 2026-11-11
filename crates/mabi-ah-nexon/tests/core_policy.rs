use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use mabi_ah_nexon::error_code::{ErrorKind, NexonCode, RetryClass, parse_error_body};
use mabi_ah_nexon::key::{ApiKey, KeyError, default_key_path};
use mabi_ah_nexon::limits::Tier;
use mabi_ah_nexon::rate_limit::{Clock, Sleeper, TokenBucket};
use mabi_ah_nexon::retry::RetryPolicy;

const KEY: &str = "ak_test_0123456789abcdefghijklmnopqrstuvwxyzZZZZ";

fn tail32(s: &str) -> &str {
    &s[s.len() - 32..]
}

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("mabi_ah_key_{}_{name}", std::process::id()));
    std::fs::write(&p, contents).expect("write temp key file");
    p
}

// ---- key ----

#[test]
fn debug_and_display_never_show_key() {
    let k = ApiKey::new(KEY).unwrap();
    let dbg = format!("{k:?}");
    let disp = format!("{k}");
    assert_eq!(dbg, "ApiKey(****ZZZZ)");
    assert_eq!(disp, "ApiKey(****ZZZZ)");
    for out in [&dbg, &disp] {
        assert!(!out.contains(KEY));
        assert!(!out.contains(tail32(KEY)));
        assert!(!out.contains(&KEY[..20]));
    }
}

#[test]
fn redact_removes_full_key_and_last_32_chars() {
    let k = ApiKey::new(KEY).unwrap();
    let text = format!("header={KEY} tail={} end", tail32(KEY));
    let out = k.redact(&text);
    assert!(!out.contains(KEY));
    assert!(!out.contains(tail32(KEY)));
    assert_eq!(out, "header=<redacted> tail=<redacted> end");
}

#[test]
fn redact_leaves_unrelated_text_alone() {
    let k = ApiKey::new(KEY).unwrap();
    assert_eq!(k.redact("nothing secret here"), "nothing secret here");
}

#[test]
fn new_trims_and_rejects_empty() {
    assert_eq!(ApiKey::new("  abc \n").unwrap().expose_secret(), "abc");
    assert_eq!(ApiKey::new("   \n"), Err(KeyError::Empty));
}

#[test]
fn env_takes_precedence_over_file() {
    let f = temp_file("prec", "file_key\n");
    let k = ApiKey::load(Some(&f), Some(" env_key ".to_owned())).unwrap();
    assert_eq!(k.expose_secret(), "env_key");
    let _ = std::fs::remove_file(f);
}

#[test]
fn file_is_used_when_env_absent_and_trimmed() {
    let f = temp_file("file", "  file_key\r\n");
    let k = ApiKey::load(Some(&f), None).unwrap();
    assert_eq!(k.expose_secret(), "file_key");
    let _ = std::fs::remove_file(f);
}

#[test]
fn empty_env_and_empty_file_are_errors() {
    assert_eq!(ApiKey::load(None, Some("  ".to_owned())), Err(KeyError::Empty));
    let f = temp_file("empty", "   \n");
    assert_eq!(ApiKey::load(Some(&f), None), Err(KeyError::Empty));
    let _ = std::fs::remove_file(f);
}

#[test]
fn neither_source_is_missing() {
    assert_eq!(ApiKey::load(None, None), Err(KeyError::Missing));
}

#[test]
fn default_path_resolves_from_given_home() {
    assert_eq!(
        default_key_path(Some(Path::new("/home/x"))),
        Some(PathBuf::from("/home/x/.config/nexon/openapi_key"))
    );
    assert_eq!(default_key_path(None), None);
}

// ---- limits ----

#[test]
fn tier_quotas_match_sot() {
    assert_eq!(Tier::Development.per_second(), 5);
    assert_eq!(Tier::Development.per_day(), 1_000);
    assert_eq!(Tier::Service.per_second(), 500);
    assert_eq!(Tier::Service.per_day(), 20_000_000);
}

// ---- error codes ----

#[test]
fn code_names_round_trip_and_exclude_00008() {
    assert_eq!(NexonCode::all().count(), 10);
    for c in NexonCode::all() {
        assert_eq!(NexonCode::from_name(c.name()), Some(c));
        assert!(c.name().starts_with("OPENAPI000"));
    }
    assert_eq!(NexonCode::from_name("OPENAPI00008"), None);
    assert_eq!(NexonCode::from_name("OPENAPI00012"), None);
}

#[test]
fn http_status_table() {
    use NexonCode::*;
    assert_eq!(OpenApi00001.http_status(), 500);
    assert_eq!(OpenApi00002.http_status(), 403);
    assert_eq!(OpenApi00007.http_status(), 429);
    assert_eq!(OpenApi00011.http_status(), 503);
    for c in [OpenApi00003, OpenApi00004, OpenApi00005, OpenApi00006, OpenApi00009, OpenApi00010] {
        assert_eq!(c.http_status(), 400, "{}", c.name());
    }
}

#[test]
fn retry_class_table() {
    use NexonCode::*;
    assert_eq!(OpenApi00001.retry(), RetryClass::Backoff);
    assert_eq!(OpenApi00007.retry(), RetryClass::Backoff);
    assert_eq!(OpenApi00009.retry(), RetryClass::Once);
    assert_eq!(OpenApi00010.retry(), RetryClass::Backoff);
    assert_eq!(OpenApi00011.retry(), RetryClass::Backoff);
    for c in [OpenApi00002, OpenApi00003, OpenApi00004, OpenApi00005, OpenApi00006] {
        assert_eq!(c.retry(), RetryClass::Never, "{}", c.name());
    }
}

#[test]
fn kind_table() {
    use NexonCode::*;
    assert_eq!(OpenApi00001.kind(), ErrorKind::Internal);
    assert_eq!(OpenApi00002.kind(), ErrorKind::Forbidden);
    assert_eq!(OpenApi00003.kind(), ErrorKind::InvalidId);
    assert_eq!(OpenApi00004.kind(), ErrorKind::InvalidParam);
    assert_eq!(OpenApi00005.kind(), ErrorKind::InvalidKey);
    assert_eq!(OpenApi00006.kind(), ErrorKind::InvalidPath);
    assert_eq!(OpenApi00007.kind(), ErrorKind::RateLimited);
    assert_eq!(OpenApi00009.kind(), ErrorKind::DataPreparing);
    assert_eq!(OpenApi00010.kind(), ErrorKind::GameMaintenance);
    assert_eq!(OpenApi00011.kind(), ErrorKind::ApiMaintenance);
}

#[test]
fn parses_api_error_body() {
    let body = r#"{"error":{"name":"OPENAPI00004","message":"Please input valid parameter"}}"#;
    let e = parse_error_body(body).expect("parsed");
    assert_eq!(e.code, NexonCode::OpenApi00004);
    assert_eq!(e.message, "Please input valid parameter");
}

#[test]
fn rejects_unknown_or_malformed_error_body() {
    assert_eq!(parse_error_body(r#"{"error":{"name":"OPENAPI00008","message":"x"}}"#), None);
    assert_eq!(parse_error_body("<html>502</html>"), None);
}

// ---- retry ----

fn policy() -> RetryPolicy {
    RetryPolicy { max_attempts: 5, base_ms: 100, cap_ms: 1_000 }
}

#[test]
fn never_class_does_not_retry() {
    assert_eq!(policy().next_delay(1, RetryClass::Never, 1), None);
}

#[test]
fn once_class_retries_only_first_attempt() {
    let d = policy().next_delay(1, RetryClass::Once, 42).expect("retry once");
    assert!(d <= Duration::from_millis(100));
    assert_eq!(policy().next_delay(2, RetryClass::Once, 42), None);
}

#[test]
fn backoff_stops_at_max_attempts() {
    for a in 1..5 {
        assert!(policy().next_delay(a, RetryClass::Backoff, 9).is_some(), "attempt {a}");
    }
    assert_eq!(policy().next_delay(5, RetryClass::Backoff, 9), None);
}

#[test]
fn backoff_is_bounded_by_exponential_and_cap() {
    let p = RetryPolicy { max_attempts: 100, base_ms: 100, cap_ms: 1_000 };
    for seed in 0..200u64 {
        let d1 = p.next_delay(1, RetryClass::Backoff, seed).unwrap();
        assert!(d1 <= Duration::from_millis(100));
        let d3 = p.next_delay(3, RetryClass::Backoff, seed).unwrap();
        assert!(d3 <= Duration::from_millis(400));
        let d50 = p.next_delay(50, RetryClass::Backoff, seed).unwrap();
        assert!(d50 <= Duration::from_millis(1_000));
    }
}

#[test]
fn backoff_is_deterministic_per_seed_and_jittered_across_seeds() {
    let p = policy();
    assert_eq!(
        p.next_delay(3, RetryClass::Backoff, 7),
        p.next_delay(3, RetryClass::Backoff, 7)
    );
    let distinct: std::collections::BTreeSet<_> =
        (0..50u64).filter_map(|s| p.next_delay(3, RetryClass::Backoff, s)).collect();
    assert!(distinct.len() > 1, "jitter must vary across seeds");
}

#[test]
fn huge_attempt_does_not_overflow() {
    let p = RetryPolicy { max_attempts: u32::MAX, base_ms: u64::MAX / 2, cap_ms: 5_000 };
    let d = p.next_delay(u32::MAX - 1, RetryClass::Backoff, 3).unwrap();
    assert!(d <= Duration::from_millis(5_000));
}

// ---- rate limit (fake clock) ----

#[derive(Clone)]
struct FakeClock(Rc<Cell<Duration>>);
impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.0.get()
    }
}

struct FakeSleeper(Rc<Cell<Duration>>);
impl Sleeper for FakeSleeper {
    fn sleep(&self, d: Duration) {
        self.0.set(self.0.get() + d);
    }
}

fn bucket(rate: u32, cap: u32) -> (TokenBucket<FakeClock, FakeSleeper>, Rc<Cell<Duration>>) {
    let t = Rc::new(Cell::new(Duration::ZERO));
    let b = TokenBucket::new(rate, cap, FakeClock(t.clone()), FakeSleeper(t.clone()));
    (b, t)
}

#[test]
fn burst_up_to_capacity_is_immediate() {
    let (mut b, t) = bucket(5, 5);
    for _ in 0..5 {
        b.acquire();
    }
    assert_eq!(t.get(), Duration::ZERO);
}

#[test]
fn sixth_call_waits_one_interval() {
    let (mut b, t) = bucket(5, 5);
    for _ in 0..6 {
        b.acquire();
    }
    assert_eq!(t.get(), Duration::from_millis(200));
}

#[test]
fn sustained_rate_is_exact() {
    let (mut b, t) = bucket(5, 5);
    for _ in 0..10 {
        b.acquire();
    }
    assert_eq!(t.get(), Duration::from_secs(1));
}

#[test]
fn idle_time_refills_only_up_to_capacity() {
    let (mut b, t) = bucket(5, 5);
    for _ in 0..5 {
        b.acquire();
    }
    t.set(t.get() + Duration::from_secs(60));
    let before = t.get();
    for _ in 0..5 {
        b.acquire();
    }
    assert_eq!(t.get(), before, "refill capped at capacity");
    b.acquire();
    assert_eq!(t.get(), before + Duration::from_millis(200));
}

#[test]
fn tier_bucket_uses_development_rate() {
    let t = Rc::new(Cell::new(Duration::ZERO));
    let mut b = TokenBucket::for_tier(Tier::Development, FakeClock(t.clone()), FakeSleeper(t.clone()));
    for _ in 0..6 {
        b.acquire();
    }
    assert_eq!(t.get(), Duration::from_millis(200));
}
