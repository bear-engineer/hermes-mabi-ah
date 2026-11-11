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

type Row = (NexonCode, &'static str, u16, RetryClass, ErrorKind);

/// The single table: code, wire name, http status, retry class, kind.
const TABLE: [Row; 10] = [
    (
        NexonCode::OpenApi00001,
        "OPENAPI00001",
        500,
        RetryClass::Backoff,
        ErrorKind::Internal,
    ),
    (
        NexonCode::OpenApi00002,
        "OPENAPI00002",
        403,
        RetryClass::Never,
        ErrorKind::Forbidden,
    ),
    (
        NexonCode::OpenApi00003,
        "OPENAPI00003",
        400,
        RetryClass::Never,
        ErrorKind::InvalidId,
    ),
    (
        NexonCode::OpenApi00004,
        "OPENAPI00004",
        400,
        RetryClass::Never,
        ErrorKind::InvalidParam,
    ),
    (
        NexonCode::OpenApi00005,
        "OPENAPI00005",
        400,
        RetryClass::Never,
        ErrorKind::InvalidKey,
    ),
    (
        NexonCode::OpenApi00006,
        "OPENAPI00006",
        400,
        RetryClass::Never,
        ErrorKind::InvalidPath,
    ),
    (
        NexonCode::OpenApi00007,
        "OPENAPI00007",
        429,
        RetryClass::Backoff,
        ErrorKind::RateLimited,
    ),
    (
        NexonCode::OpenApi00009,
        "OPENAPI00009",
        400,
        RetryClass::Once,
        ErrorKind::DataPreparing,
    ),
    (
        NexonCode::OpenApi00010,
        "OPENAPI00010",
        400,
        RetryClass::Backoff,
        ErrorKind::GameMaintenance,
    ),
    (
        NexonCode::OpenApi00011,
        "OPENAPI00011",
        503,
        RetryClass::Backoff,
        ErrorKind::ApiMaintenance,
    ),
];

impl NexonCode {
    pub fn all() -> impl Iterator<Item = NexonCode> {
        TABLE.iter().map(|r| r.0)
    }
    pub fn from_name(name: &str) -> Option<NexonCode> {
        TABLE.iter().find(|r| r.1 == name).map(|r| r.0)
    }
    pub fn name(self) -> &'static str {
        self.row().1
    }
    pub fn http_status(self) -> u16 {
        self.row().2
    }
    pub fn retry(self) -> RetryClass {
        self.row().3
    }
    pub fn kind(self) -> ErrorKind {
        self.row().4
    }
    fn row(self) -> &'static Row {
        // invariant: TABLE has one row for every NexonCode variant.
        TABLE
            .iter()
            .find(|r| r.0 == self)
            .expect("every NexonCode has a table row")
    }
}

/// Parses `{"error":{"name":..,"message":..}}`. Unknown name or bad JSON -> None.
pub fn parse_error_body(body: &str) -> Option<ApiError> {
    #[derive(Deserialize)]
    struct Envelope {
        error: Inner,
    }
    #[derive(Deserialize)]
    struct Inner {
        name: String,
        #[serde(default)]
        message: String,
    }
    let env: Envelope = serde_json::from_str(body).ok()?;
    let code = NexonCode::from_name(&env.error.name)?;
    Some(ApiError {
        code,
        message: env.error.message,
    })
}
