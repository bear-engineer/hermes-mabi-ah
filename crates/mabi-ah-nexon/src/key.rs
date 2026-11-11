//! API key newtype: masked Debug/Display, file/env loading, redaction.
use std::fmt;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum KeyError {
    #[error("API 키가 비어 있습니다")]
    Empty,
    #[error("API 키를 찾을 수 없습니다 (NEXON_OPEN_API_KEY 또는 ~/.config/nexon/openapi_key)")]
    Missing,
    #[error("API 키 파일을 읽을 수 없습니다: {0}")]
    Io(String),
}

#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(_raw: &str) -> Result<Self, KeyError> {
        todo!()
    }
    /// env (NEXON_OPEN_API_KEY) wins over file.
    pub fn load(_file: Option<&Path>, _env: Option<String>) -> Result<Self, KeyError> {
        todo!()
    }
    pub fn expose_secret(&self) -> &str {
        todo!()
    }
    pub fn redact(&self, _text: &str) -> String {
        todo!()
    }
}

/// `$HOME/.config/nexon/openapi_key`; `None` when no home is given.
pub fn default_key_path(_home: Option<&Path>) -> Option<PathBuf> {
    todo!()
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
