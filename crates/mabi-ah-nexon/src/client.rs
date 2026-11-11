//! Blocking HTTP client for the NEXON Open API (GET + JSON).
use std::rc::Rc;
use std::time::{Duration, Instant};

use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::CallMeter;
use serde_json::Value;

use crate::key::ApiKey;
use crate::limits::Tier;
use crate::rate_limit::{Clock, Sleeper, TokenBucket};
use crate::retry::RetryPolicy;

pub const DEFAULT_BASE_URL: &str = "https://open.api.nexon.com";
pub const KEY_HEADER: &str = "x-nxopen-api-key";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Wall clock for the token bucket.
pub struct SystemClock {
    origin: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

pub struct NexonClient<C: Clock = SystemClock, S: Sleeper = ThreadSleeper> {
    pub base_url: String,
    pub key: ApiKey,
    pub bucket: TokenBucket<C, S>,
    pub retry: RetryPolicy,
    pub meter: Rc<dyn CallMeter>,
    backoff: Box<dyn Sleeper>,
}

impl NexonClient {
    /// Development-tier client against the production host.
    pub fn new(key: ApiKey, meter: Rc<dyn CallMeter>) -> Self {
        let bucket =
            TokenBucket::for_tier(Tier::Development, SystemClock::default(), ThreadSleeper);
        Self::with_parts(
            DEFAULT_BASE_URL,
            key,
            bucket,
            RetryPolicy {
                max_attempts: 4,
                base_ms: 500,
                cap_ms: 8_000,
            },
            meter,
            Box::new(ThreadSleeper),
        )
    }
}

impl<C: Clock, S: Sleeper> NexonClient<C, S> {
    pub fn with_parts(
        base_url: &str,
        key: ApiKey,
        bucket: TokenBucket<C, S>,
        retry: RetryPolicy,
        meter: Rc<dyn CallMeter>,
        backoff: Box<dyn Sleeper>,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            key,
            bucket,
            retry,
            meter,
            backoff,
        }
    }

    /// GET `path` with URL-encoded `query`; parses the body as JSON.
    /// Every HTTP request (retries included) passes the meter first.
    pub fn get_json(&mut self, path: &str, query: &[(&str, &str)]) -> Result<Value, AppError> {
        let _ = (path, query, &self.backoff);
        todo!("node n2 green")
    }
}
