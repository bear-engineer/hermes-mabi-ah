//! Production `CallMeter` over a `BudgetLedger`.
use std::cell::RefCell;

use crate::error::AppError;
use crate::ports::{BudgetLedger, CallMeter, Clock};

pub struct LedgerMeter<L, C> {
    ledger: RefCell<L>,
    clock: C,
    cap: u64,
}

impl<L: BudgetLedger, C: Clock> LedgerMeter<L, C> {
    pub fn new(_ledger: L, _clock: C, _cap: u64) -> Self {
        todo!("node A green: LedgerMeter::new")
    }
}

impl<L: BudgetLedger, C: Clock> CallMeter for LedgerMeter<L, C> {
    fn before_request(&self) -> Result<(), AppError> {
        let _ = (&self.ledger, &self.clock, self.cap);
        todo!("node A green: before_request")
    }
}
