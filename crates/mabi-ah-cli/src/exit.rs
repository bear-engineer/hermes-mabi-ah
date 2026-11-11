//! Process exit codes (contract from the plan). `AppError::exit_code` is the
//! mapping; these names are for the places that need a code without an error.
use std::process::ExitCode;

use mabi_ah_app::error::AppError;

#[allow(unused_imports)] // the full table is re-exported; callers use what they need
pub use mabi_ah_app::error::exit_code::*;

pub fn of(error: &AppError) -> ExitCode {
    ExitCode::from(error.exit_code())
}
