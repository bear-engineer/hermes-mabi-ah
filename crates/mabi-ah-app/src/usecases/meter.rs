//! Production `CallMeter` over a `BudgetLedger`.
use std::cell::RefCell;

use crate::error::AppError;
use crate::ports::{BudgetLedger, CallMeter, Clock};

/// Counts one unit per request in the ledger and refuses once today's usage
/// reaches `cap`. The only writer of the ledger.
pub struct LedgerMeter<L, C> {
    ledger: RefCell<L>,
    clock: C,
    cap: u64,
}

impl<L: BudgetLedger, C: Clock> LedgerMeter<L, C> {
    pub fn new(ledger: L, clock: C, cap: u64) -> Self {
        Self {
            ledger: RefCell::new(ledger),
            clock,
            cap,
        }
    }
}

impl<L: BudgetLedger, C: Clock> CallMeter for LedgerMeter<L, C> {
    fn before_request(&self) -> Result<(), AppError> {
        let now = self.clock.now();
        let used = self.ledger.borrow().used_on(now.date())?;
        if used >= self.cap {
            return Err(AppError::BudgetExceeded {
                used,
                limit: self.cap,
            });
        }
        self.ledger.borrow_mut().record_call(now)
    }
}
