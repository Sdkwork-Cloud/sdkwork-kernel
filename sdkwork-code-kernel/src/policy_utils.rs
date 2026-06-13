/// Returns the first policy category from the list, or the fallback value.
pub fn first_policy_category(policy_categories: &[String], fallback: &str) -> String {
    policy_categories
        .first()
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_policy_category_returns_first_item() {
        let categories = vec!["cat.1".to_string(), "cat.2".to_string()];
        assert_eq!(first_policy_category(&categories, "fallback"), "cat.1");
    }

    #[test]
    fn first_policy_category_returns_fallback_when_empty() {
        let categories = vec![];
        assert_eq!(first_policy_category(&categories, "fallback"), "fallback");
    }
}
