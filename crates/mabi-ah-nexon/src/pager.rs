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
/// A cursor that repeats the previous one ends the walk (guards a stuck server).
pub fn collect_pages<T>(
    first: Option<String>,
    max_pages: usize,
    mut fetch: impl FnMut(Option<&str>) -> Result<Page<T>, AppError>,
) -> Result<Paged<T>, AppError> {
    let mut items = Vec::new();
    let mut cursor = first;
    for _ in 0..max_pages {
        let page = fetch(cursor.as_deref())?;
        items.extend(page.items);
        match page.next_cursor {
            None => {
                return Ok(Paged {
                    items,
                    truncated: false,
                });
            }
            Some(next) if cursor.as_deref() == Some(next.as_str()) => {
                return Ok(Paged {
                    items,
                    truncated: false,
                });
            }
            Some(next) => cursor = Some(next),
        }
    }
    Ok(Paged {
        items,
        truncated: cursor.is_some() && max_pages > 0,
    })
}
