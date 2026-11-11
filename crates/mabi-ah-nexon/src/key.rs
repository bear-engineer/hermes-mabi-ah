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
    pub fn new(raw: &str) -> Result<Self, KeyError> {
        let t = raw.trim();
        if t.is_empty() {
            return Err(KeyError::Empty);
        }
        Ok(Self(t.to_owned()))
    }
    /// env (NEXON_OPEN_API_KEY) wins over file.
    pub fn load(file: Option<&Path>, env: Option<String>) -> Result<Self, KeyError> {
        if let Some(v) = env {
            return Self::new(&v);
        }
        match file {
            Some(p) => {
                let raw = std::fs::read_to_string(p).map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => KeyError::Missing,
                    k => KeyError::Io(format!("{k}")),
                })?;
                Self::new(&raw)
            }
            None => Err(KeyError::Missing),
        }
    }
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
    pub fn redact(&self, text: &str) -> String {
        let mut out = text.replace(self.0.as_str(), "<redacted>");
        let n = self.0.chars().count();
        if n > 32 {
            let tail: String = self.0.chars().skip(n - 32).collect();
            out = out.replace(tail.as_str(), "<redacted>");
        }
        out
    }
}

/// `$HOME/.config/nexon/openapi_key`; `None` when no home is given.
pub fn default_key_path(home: Option<&Path>) -> Option<PathBuf> {
    home.map(|h| h.join(".config").join("nexon").join("openapi_key"))
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_masked(&self.0, f)
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_masked(&self.0, f)
    }
}

fn write_masked(key: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let n = key.chars().count();
    if n <= 4 {
        return f.write_str("ApiKey(****)");
    }
    let last4: String = key.chars().skip(n - 4).collect();
    write!(f, "ApiKey(****{last4})")
}
