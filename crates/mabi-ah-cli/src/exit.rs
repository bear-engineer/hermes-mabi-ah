//! Process exit codes (contract from the plan). `AppError::exit_code` is the
//! mapping; these names are for the places that need a code without an error.
use std::process::ExitCode;

use mabi_ah_app::error::AppError;

#[allow(dead_code)] // contract table; checked against AppError::exit_code in tests
pub const OK: u8 = 0;
pub const INTERNAL: u8 = 1;
#[allow(dead_code)]
pub const USAGE: u8 = 2;
#[allow(dead_code)]
pub const NOT_FOUND: u8 = 3;
#[allow(dead_code)]
pub const AUTH: u8 = 4;
#[allow(dead_code)]
pub const RATE_OR_BUDGET: u8 = 5;
#[allow(dead_code)]
pub const UPSTREAM: u8 = 6;
#[allow(dead_code)]
pub const STORE: u8 = 7;

pub fn of(error: &AppError) -> ExitCode {
    ExitCode::from(error.exit_code())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mabi_ah_app::error::UpstreamKind;

    #[test]
    fn consts_match_app_error() {
        let up = AppError::Upstream {
            kind: UpstreamKind::Internal,
            nexon_code: None,
        };
        let cases = [
            (AppError::Internal(String::new()), INTERNAL),
            (AppError::Usage(String::new()), USAGE),
            (
                AppError::NotFound {
                    what: String::new(),
                    hint: None,
                },
                NOT_FOUND,
            ),
            (AppError::Auth(String::new()), AUTH),
            (AppError::RateLimited { retry_after: None }, RATE_OR_BUDGET),
            (
                AppError::BudgetExceeded { used: 1, limit: 1 },
                RATE_OR_BUDGET,
            ),
            (up, UPSTREAM),
            (AppError::Store(String::new()), STORE),
        ];
        for (e, want) in cases {
            assert_eq!(e.exit_code(), want, "{e}");
        }
    }
}
