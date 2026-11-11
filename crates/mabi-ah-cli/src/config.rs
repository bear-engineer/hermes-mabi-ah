//! Resolved global settings (flags + environment).
use std::path::PathBuf;

use mabi_ah_nexon::key::default_key_path;
use mabi_ah_nexon::limits::Tier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Table,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub format: Format,
    pub db_path: PathBuf,
    pub key_file: Option<PathBuf>,
    pub base_url: Option<String>,
    pub tier: Tier,
    pub force_budget: bool,
}

impl Config {
    pub fn tier_name(&self) -> &'static str {
        tier_name(self.tier)
    }

    /// Calls per UTC day the meter enforces: 90% of the tier limit, or the
    /// full limit with `--force-budget`.
    pub fn daily_cap(&self) -> u64 {
        let limit = u64::from(self.tier.per_day());
        if self.force_budget {
            limit
        } else {
            limit * 9 / 10
        }
    }
}

pub fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Development => "development",
        Tier::Service => "service",
    }
}

/// `--key-file` wins, else `$HOME/.config/nexon/openapi_key`.
pub fn resolve_key_file(flag: Option<PathBuf>) -> Option<PathBuf> {
    flag.or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        default_key_path(home.as_deref())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(tier: Tier, force: bool) -> Config {
        Config {
            format: Format::Json,
            db_path: PathBuf::new(),
            key_file: None,
            base_url: None,
            tier,
            force_budget: force,
        }
    }

    #[test]
    fn cap_is_ninety_percent_unless_forced() {
        assert_eq!(cfg(Tier::Development, false).daily_cap(), 900);
        assert_eq!(cfg(Tier::Development, true).daily_cap(), 1000);
        assert_eq!(cfg(Tier::Service, false).daily_cap(), 18_000_000);
    }
}
