//! AppError (node P): the one error type that crosses the application boundary.
//!
//! Adapters convert their own errors into `AppError` inside their port
//! implementations; the cli maps it to an exit code and the envelope `error`.

use std::time::Duration;

/// Why the NEXON Open API could not serve a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamKind {
    GameMaintenance,
    ApiMaintenance,
    DataPreparing,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    #[error("not found: {what}")]
    NotFound { what: String, hint: Option<String> },
    #[error("usage: {0}")]
    Usage(String),
    #[error("api key or permission: {0}")]
    Auth(String),
    #[error("rate limited by the API")]
    RateLimited { retry_after: Option<Duration> },
    #[error("daily call budget exceeded: {used}/{limit}")]
    BudgetExceeded { used: u64, limit: u64 },
    #[error("NEXON Open API unavailable: {kind:?}")]
    Upstream {
        kind: UpstreamKind,
        nexon_code: Option<String>,
    },
    #[error("store: {0}")]
    Store(String),
    #[error("internal: {0}")]
    Internal(String),
}

impl AppError {
    /// Process exit code (0 is success and never returned here).
    pub fn exit_code(&self) -> u8 {
        todo!("node P green")
    }

    /// Whether the same command may succeed if simply run again later today.
    pub fn retryable(&self) -> bool {
        todo!("node P green")
    }

    /// Stable machine id for the envelope `error.code`.
    pub fn code(&self) -> &'static str {
        todo!("node P green")
    }
}
