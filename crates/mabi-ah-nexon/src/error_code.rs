//! NEXON error codes. Single table = SOT for http status, retry class, kind.
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    Never,
    Backoff,
    Once,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Internal,
    Forbidden,
    InvalidId,
    InvalidParam,
    InvalidKey,
    InvalidPath,
    RateLimited,
    DataPreparing,
    GameMaintenance,
    ApiMaintenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NexonCode {
    OpenApi00001,
    OpenApi00002,
    OpenApi00003,
    OpenApi00004,
    OpenApi00005,
    OpenApi00006,
    OpenApi00007,
    OpenApi00009,
    OpenApi00010,
    OpenApi00011,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub code: NexonCode,
    pub message: String,
}

impl NexonCode {
    pub fn all() -> impl Iterator<Item = NexonCode> {
        std::iter::empty()
    }
    pub fn from_name(_name: &str) -> Option<NexonCode> {
        todo!()
    }
    pub fn name(self) -> &'static str {
        todo!()
    }
    pub fn http_status(self) -> u16 {
        todo!()
    }
    pub fn retry(self) -> RetryClass {
        todo!()
    }
    pub fn kind(self) -> ErrorKind {
        todo!()
    }
}

/// Parses `{"error":{"name":..,"message":..}}`. Unknown name or bad JSON -> None.
pub fn parse_error_body(_body: &str) -> Option<ApiError> {
    todo!()
}
