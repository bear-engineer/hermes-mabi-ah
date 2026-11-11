//! Retry policy: exponential backoff with full jitter.
use std::time::Duration;

use crate::error_code::RetryClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_ms: u64,
    pub cap_ms: u64,
}

impl RetryPolicy {
    /// `attempt` is the 1-based attempt that just failed. `None` = do not retry.
    pub fn next_delay(&self, _attempt: u32, _class: RetryClass, _jitter_seed: u64) -> Option<Duration> {
        todo!()
    }
}
