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
    pub fn next_delay(
        &self,
        attempt: u32,
        class: RetryClass,
        jitter_seed: u64,
    ) -> Option<Duration> {
        let allowed = match class {
            RetryClass::Never => false,
            RetryClass::Once => attempt == 1 && self.max_attempts > 1,
            RetryClass::Backoff => attempt < self.max_attempts,
        };
        if !allowed {
            return None;
        }
        let shift = attempt.saturating_sub(1);
        let factor = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
        let ceiling = self.base_ms.saturating_mul(factor).min(self.cap_ms);
        let ms = fastrand::Rng::with_seed(jitter_seed).u64(0..=ceiling);
        Some(Duration::from_millis(ms))
    }
}
