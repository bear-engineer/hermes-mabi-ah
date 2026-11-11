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
    pub const INTERNAL: u8 = 1;
    pub const USAGE: u8 = 2;
    pub const NOT_FOUND: u8 = 3;
    pub const AUTH: u8 = 4;
    pub const RATE_OR_BUDGET: u8 = 5;
    pub const UPSTREAM: u8 = 6;
    pub const STORE: u8 = 7;
}

impl AppError {
    /// Process exit code (0 is success and never returned here).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Internal(_) => exit_code::INTERNAL,
            Self::Usage(_) => exit_code::USAGE,
            Self::NotFound { .. } => exit_code::NOT_FOUND,
            Self::Auth(_) => exit_code::AUTH,
            Self::RateLimited { .. } | Self::BudgetExceeded { .. } => exit_code::RATE_OR_BUDGET,
            Self::Upstream { .. } => exit_code::UPSTREAM,
            Self::Store(_) => exit_code::STORE,
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
