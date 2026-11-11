//! Blocking HTTP client for the NEXON Open API (GET + JSON).
use std::rc::Rc;
use std::time::{Duration, Instant};

use mabi_ah_app::error::{AppError, UpstreamKind};
use mabi_ah_app::ports::CallMeter;
use serde_json::Value;

use crate::error_code::{ErrorKind, NexonCode, RetryClass, parse_error_body};
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
    agent: ureq::Agent,
}

/// One failed attempt: the mapped error plus how it may be retried.
struct Failure {
    error: AppError,
    class: RetryClass,
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
            RetryPolicy::default(),
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
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(REQUEST_TIMEOUT))
                .build()
                .into(),
        }
    }

    /// GET `path` with URL-encoded `query`; parses the body as JSON.
    /// Every HTTP request (retries included) passes the meter first.
    pub fn get_json(&mut self, path: &str, query: &[(&str, &str)]) -> Result<Value, AppError> {
        let mut attempt = 1u32;
        loop {
            self.bucket.acquire();
            self.meter.before_request()?;
            let failure = match self.send_once(path, query) {
                Ok(v) => return Ok(v),
                Err(f) => f,
            };
            match self
                .retry
                .next_delay(attempt, failure.class, fastrand::u64(..))
            {
                Some(d) => {
                    self.backoff.sleep(d);
                    attempt += 1;
                }
                None => return Err(failure.error),
            }
        }
    }

    fn send_once(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, Failure> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .agent
            .get(&url)
            .header(KEY_HEADER, self.key.expose_secret());
        for (k, v) in query {
            req = req.query(k, v);
        }
        let mut resp = req.call().map_err(|_| Failure {
            // Transport errors can embed URLs/headers; keep only a fixed text.
            error: AppError::Upstream {
                kind: UpstreamKind::Internal,
                nexon_code: None,
            },
            class: RetryClass::Backoff,
        })?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().map_err(|_| Failure {
            error: AppError::Upstream {
                kind: UpstreamKind::Internal,
                nexon_code: None,
            },
            class: RetryClass::Backoff,
        })?;
        if (200..300).contains(&status) {
            return serde_json::from_str(&body).map_err(|_| Failure {
                error: AppError::Internal(format!("{path}: response is not valid JSON")),
                class: RetryClass::Never,
            });
        }
        Err(self.map_error(path, status, &body))
    }

    fn map_error(&self, path: &str, status: u16, body: &str) -> Failure {
        let Some(api) = parse_error_body(body) else {
            return Self::map_status(status);
        };
        let name = api.code.name().to_owned();
        let message = self.key.redact(&api.message);
        let error = match api.code.kind() {
            ErrorKind::Internal => AppError::Upstream {
                kind: UpstreamKind::Internal,
                nexon_code: Some(name),
            },
            ErrorKind::Forbidden | ErrorKind::InvalidKey => {
                AppError::Auth(format!("{} {message}", api.code.name()))
            }
            ErrorKind::InvalidId => AppError::Usage(format!("{name}: {message}")),
            ErrorKind::InvalidParam => AppError::NotFound {
                what: format!("{path}: {message}"),
                hint: Some(format!("{name}: 이름·서버·채널 값을 확인하세요")),
            },
            ErrorKind::InvalidPath => AppError::Internal(format!("{name}: {message}")),
            ErrorKind::RateLimited => AppError::RateLimited { retry_after: None },
            ErrorKind::DataPreparing => upstream(UpstreamKind::DataPreparing, api.code),
            ErrorKind::GameMaintenance => upstream(UpstreamKind::GameMaintenance, api.code),
            ErrorKind::ApiMaintenance => upstream(UpstreamKind::ApiMaintenance, api.code),
        };
        Failure {
            error,
            class: api.code.retry(),
        }
    }

    /// Error body missing or unknown: decide by HTTP status alone.
    fn map_status(status: u16) -> Failure {
        let (error, class) = match status {
            401 | 403 => (AppError::Auth(format!("HTTP {status}")), RetryClass::Never),
            404 => (
                AppError::NotFound {
                    what: "HTTP 404".to_owned(),
                    hint: None,
                },
                RetryClass::Never,
            ),
            429 => (
                AppError::RateLimited { retry_after: None },
                RetryClass::Backoff,
            ),
            500..=599 => (
                AppError::Upstream {
                    kind: UpstreamKind::Internal,
                    nexon_code: None,
                },
                RetryClass::Backoff,
            ),
            _ => (
                AppError::Internal(format!("HTTP {status}")),
                RetryClass::Never,
            ),
        };
        Failure { error, class }
    }
}

fn upstream(kind: UpstreamKind, code: NexonCode) -> AppError {
    AppError::Upstream {
        kind,
        nexon_code: Some(code.name().to_owned()),
    }
}
