//! Token bucket with injectable clock/sleeper.
use std::time::Duration;

use crate::limits::Tier;

pub trait Clock {
    /// Monotonic time since an arbitrary origin.
    fn now(&self) -> Duration;
}

pub trait Sleeper {
    fn sleep(&self, d: Duration);
}

pub struct TokenBucket<C: Clock, S: Sleeper> {
    clock: C,
    sleeper: S,
    interval_ns: u128,
    capacity_ns: u128,
    credit_ns: u128,
    last: Duration,
}

impl<C: Clock, S: Sleeper> TokenBucket<C, S> {
    pub fn new(rate_per_sec: u32, capacity: u32, clock: C, sleeper: S) -> Self {
        let interval_ns = 1_000_000_000u128 / u128::from(rate_per_sec.max(1));
        let capacity_ns = interval_ns * u128::from(capacity.max(1));
        let last = clock.now();
        Self {
            clock,
            sleeper,
            interval_ns,
            capacity_ns,
            credit_ns: capacity_ns,
            last,
        }
    }
    pub fn for_tier(tier: Tier, clock: C, sleeper: S) -> Self {
        Self::new(tier.per_second(), tier.per_second(), clock, sleeper)
    }
    /// Blocks (via Sleeper) until one token is available, then takes it.
    pub fn acquire(&mut self) {
        let now = self.clock.now();
        let elapsed = now.saturating_sub(self.last).as_nanos();
        self.last = now;
        self.credit_ns = (self.credit_ns + elapsed).min(self.capacity_ns);
        if self.credit_ns >= self.interval_ns {
            self.credit_ns -= self.interval_ns;
            return;
        }
        let wait = self.interval_ns - self.credit_ns;
        self.sleeper.sleep(Duration::from_nanos(
            u64::try_from(wait).unwrap_or(u64::MAX),
        ));
        self.credit_ns = 0;
        self.last = self.clock.now();
    }
}
