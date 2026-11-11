//! Pure domain model for the Mabinogi auction house (see PROJECT_TERMS.md).
//! No I/O, no HTTP, no SQL. Owned by node L0 for module declarations.
pub mod attribution;
pub mod category;
pub mod error;
pub mod item_name;
pub mod listing;
pub mod option;
pub mod price;
pub mod rank;
pub mod staleness;
pub mod trade;
pub mod trend;
pub mod watch;
