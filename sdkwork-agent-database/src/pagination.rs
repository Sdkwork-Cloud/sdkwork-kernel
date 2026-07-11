//! List pagination bounds aligned with `PAGINATION_SPEC.md` via `sdkwork-utils-rust`.

use crate::error::{DatabaseError, DatabaseResult};
use sdkwork_utils_rust::{DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE};

pub const MAX_SESSION_HISTORY_MESSAGES: i64 = 512;

/// Resolve SQL `LIMIT` with spec default and maximum when callers omit an explicit cap.
pub fn resolve_list_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(i64::from(DEFAULT_LIST_PAGE_SIZE))
        .clamp(1, i64::from(MAX_LIST_PAGE_SIZE))
}

/// Resolve SQL `OFFSET` (non-negative).
pub fn resolve_list_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

/// Validate the bounded session-history hydration window.
pub fn resolve_history_limit(limit: i64) -> DatabaseResult<i64> {
    if !(1..=MAX_SESSION_HISTORY_MESSAGES).contains(&limit) {
        return Err(DatabaseError::Query(format!(
            "recent message limit must be between 1 and {MAX_SESSION_HISTORY_MESSAGES}"
        )));
    }
    Ok(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit_matches_spec() {
        assert_eq!(resolve_list_limit(None), 20);
    }

    #[test]
    fn limit_is_clamped_to_max() {
        assert_eq!(resolve_list_limit(Some(500)), 200);
    }

    #[test]
    fn history_limit_rejects_unbounded_requests() {
        assert_eq!(resolve_history_limit(512).expect("max"), 512);
        assert!(resolve_history_limit(0).is_err());
        assert!(resolve_history_limit(513).is_err());
    }
}
