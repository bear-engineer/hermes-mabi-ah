//! Domain-level errors (pure; no I/O).

use thiserror::Error;

/// Failures when converting wire DTOs into domain types.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// A timestamp was not RFC 3339 / ISO 8601 UTC.
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
}
