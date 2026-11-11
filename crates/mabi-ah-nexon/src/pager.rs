//! Cursor walking.
use mabi_ah_app::error::AppError;
use mabi_ah_app::ports::Page;

#[derive(Debug, Clone, PartialEq)]
pub struct Paged<T> {
    pub items: Vec<T>,
    /// More pages existed but `max_pages` was reached.
    pub truncated: bool,
}

/// Follows `next_cursor` starting at `first` for at most `max_pages` fetches.
pub fn collect_pages<T>(
    first: Option<String>,
    max_pages: usize,
    fetch: impl FnMut(Option<&str>) -> Result<Page<T>, AppError>,
) -> Result<Paged<T>, AppError> {
    let _ = (first, max_pages, fetch);
    todo!("node n2 green")
}
