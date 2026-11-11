//! Tier quotas. SOT for rate/day numbers (briefing.md §limits).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Development,
    Service,
}

impl Tier {
    pub const fn per_second(self) -> u32 {
        match self {
            Tier::Development => 5,
            Tier::Service => 500,
        }
    }
    pub const fn per_day(self) -> u32 {
        match self {
            Tier::Development => 1_000,
            Tier::Service => 20_000_000,
        }
    }
}
