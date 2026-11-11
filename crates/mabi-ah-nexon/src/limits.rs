//! Tier quotas. SOT for rate/day numbers (briefing.md §limits).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Development,
    Service,
}

impl Tier {
    pub const fn per_second(self) -> u32 {
        todo!()
    }
    pub const fn per_day(self) -> u32 {
        todo!()
    }
}
