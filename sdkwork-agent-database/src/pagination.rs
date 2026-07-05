//! List pagination bounds aligned with `PAGINATION_SPEC.md` via `sdkwork-utils-rust`.

use sdkwork_utils_rust::{DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE};

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
}
