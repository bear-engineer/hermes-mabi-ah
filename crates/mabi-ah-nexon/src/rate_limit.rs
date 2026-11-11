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
}

impl<C: Clock, S: Sleeper> TokenBucket<C, S> {
    pub fn new(_rate_per_sec: u32, _capacity: u32, clock: C, sleeper: S) -> Self {
        Self { clock, sleeper }
    }
    pub fn for_tier(tier: Tier, clock: C, sleeper: S) -> Self {
        Self::new(tier.per_second(), tier.per_second(), clock, sleeper)
    }
    /// Blocks (via Sleeper) until one token is available, then takes it.
    pub fn acquire(&mut self) {
        todo!()
    }
}
