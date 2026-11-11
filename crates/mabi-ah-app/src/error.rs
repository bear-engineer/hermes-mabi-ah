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

/// Process exit codes; the one definition (contract from the plan).
pub mod exit_code {
    pub const OK: u8 = 0;
    pub const INTERNAL: u8 = 0;
    pub const USAGE: u8 = 0;
    pub const NOT_FOUND: u8 = 0;
    pub const AUTH: u8 = 0;
    pub const RATE_OR_BUDGET: u8 = 0;
    pub const UPSTREAM: u8 = 0;
    pub const STORE: u8 = 0;
}

impl AppError {
    /// Process exit code (0 is success and never returned here).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Internal(_) => 1,
            Self::Usage(_) => 2,
            Self::NotFound { .. } => 3,
            Self::Auth(_) => 4,
            Self::RateLimited { .. } | Self::BudgetExceeded { .. } => 5,
            Self::Upstream { .. } => 6,
            Self::Store(_) => 7,
        }
    }

    /// Whether the same command may succeed if simply run again later today.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::RateLimited { .. } | Self::Upstream { .. })
    }

    /// Stable machine id for the envelope `error.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "not_found",
            Self::Usage(_) => "usage",
            Self::Auth(_) => "auth",
            Self::RateLimited { .. } => "rate_limited",
            Self::BudgetExceeded { .. } => "budget_exceeded",
            Self::Upstream { kind, .. } => match kind {
                UpstreamKind::GameMaintenance => "upstream_game_maintenance",
                UpstreamKind::ApiMaintenance => "upstream_api_maintenance",
                UpstreamKind::DataPreparing => "upstream_data_preparing",
                UpstreamKind::Internal => "upstream_internal",
            },
            Self::Store(_) => "store",
            Self::Internal(_) => "internal",
        }
    }
}
