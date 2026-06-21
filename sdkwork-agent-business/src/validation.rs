use sdkwork_agent_kernel::{KernelError, KernelResult};
use sdkwork_utils_rust::{is_blank, trim};
use std::collections::HashSet;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub(crate) fn require_non_blank(value: &str, field_name: &str) -> KernelResult<()> {
    if is_blank(Some(value)) {
        return Err(KernelError::validation(format!("{field_name} is required")));
    }
    Ok(())
}

pub(crate) fn require_trimmed_non_blank(value: &str, field_name: &str) -> KernelResult<()> {
    if is_blank(Some(value)) {
        return Err(KernelError::validation(format!("{field_name} is required")));
    }
    if trim(value) != value {
        return Err(KernelError::validation(format!(
            "{field_name} must not contain leading or trailing whitespace"
        )));
    }
    Ok(())
}

pub(crate) fn optional_non_blank(value: String) -> Option<String> {
    if is_blank(Some(value.as_str())) {
        None
    } else {
        Some(value)
    }
}

pub(crate) fn parse_int64_string_field(value: &str, field_name: &str) -> KernelResult<u64> {
    value
        .parse::<u64>()
        .map_err(|_| KernelError::validation(format!("{field_name} must be int64 string")))
}

pub(crate) fn parse_tenant_id(value: &str) -> KernelResult<u64> {
    parse_int64_string_field(value, "tenant_id")
}

pub(crate) fn parse_organization_id(value: &str) -> KernelResult<u64> {
    parse_int64_string_field(value, "organization_id")
}

pub(crate) fn parse_owner_user_id(value: &str) -> KernelResult<u64> {
    parse_int64_string_field(value, "owner_user_id")
}

pub(crate) fn parse_expected_version(value: &str) -> KernelResult<u64> {
    parse_int64_string_field(value, "expectedVersion")
}

pub(crate) fn validate_requested_at(value: &str) -> KernelResult<()> {
    validate_rfc3339_datetime(value, "requestedAt")
}

pub(crate) fn validate_standard_id(
    value: &str,
    field_name: &str,
    required_prefix: Option<&str>,
) -> KernelResult<()> {
    require_trimmed_non_blank(value, field_name)?;
    if value.chars().count() > 128 {
        return Err(KernelError::validation(format!(
            "{field_name} must be at most 128 characters"
        )));
    }
    if !value.chars().all(is_standard_id_character) {
        return Err(KernelError::validation(format!(
            "{field_name} must use lowercase standard id characters"
        )));
    }
    if !has_non_empty_dot_segments(value) {
        return Err(KernelError::validation(format!(
            "{field_name} must use non-empty dot-delimited segments"
        )));
    }
    if let Some(prefix) = required_prefix {
        if !value.starts_with(prefix) {
            return Err(KernelError::validation(format!(
                "{field_name} must start with {prefix}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_capabilities(capabilities: &[String], field_name: &str) -> KernelResult<()> {
    let mut seen = HashSet::new();
    for capability in capabilities {
        if is_blank(Some(capability.as_str())) {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain empty capability ids"
            )));
        }
        if capability.chars().count() > 128 {
            return Err(KernelError::validation(format!(
                "{field_name} capability ids must be at most 128 characters"
            )));
        }
        if trim(capability) != *capability || !is_valid_capability_id(capability.as_str()) {
            return Err(KernelError::validation(format!(
                "{field_name} must use lowercase namespaced capability ids"
            )));
        }
        if !seen.insert(capability.as_str()) {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain duplicate capability id: {capability}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_rfc3339_datetime(value: &str, field_name: &str) -> KernelResult<()> {
    let _ = parse_rfc3339_datetime(value, field_name)?;
    Ok(())
}

pub(crate) fn parse_rfc3339_datetime(
    value: &str,
    field_name: &str,
) -> KernelResult<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
        KernelError::validation(format!("{field_name} must be RFC3339 date-time: {error}"))
    })
}

#[cfg(feature = "http-axum")]
pub(crate) fn parse_optional_rfc3339_datetime(
    value: Option<&str>,
    field_name: &str,
) -> KernelResult<Option<OffsetDateTime>> {
    let Some(value) = value else {
        return Ok(None);
    };
    parse_rfc3339_datetime(value, field_name).map(Some)
}

fn is_valid_capability_id(capability_id: &str) -> bool {
    capability_id.chars().all(is_standard_id_character) && has_non_empty_dot_segments(capability_id)
}

fn is_standard_id_character(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
}

fn has_non_empty_dot_segments(value: &str) -> bool {
    let mut segments = value.split('.');
    let mut segment_count = 0;
    for segment in &mut segments {
        segment_count += 1;
        if segment.is_empty() {
            return false;
        }
    }
    segment_count >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::KernelError;

    #[test]
    fn validate_rfc3339_datetime_accepts_valid_timestamp() {
        let result = validate_rfc3339_datetime("2026-06-01T00:00:00Z", "requestedAt");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_rfc3339_datetime_rejects_invalid_timestamp() {
        let error = validate_rfc3339_datetime("2026-06-01", "requestedAt")
            .expect_err("invalid timestamp should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("requestedAt"));
                assert!(message.contains("RFC3339"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn parse_int64_string_field_accepts_unsigned_integer_text() {
        let value = parse_int64_string_field("12345", "tenant_id")
            .expect("valid int64 string should parse");
        assert_eq!(value, 12345);
    }

    #[test]
    fn parse_int64_string_field_rejects_non_numeric_text() {
        let error = parse_int64_string_field("12x45", "tenant_id")
            .expect_err("invalid int64 string should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("tenant_id"));
                assert!(message.contains("int64 string"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn parse_tenant_id_uses_tenant_specific_error() {
        let error = parse_tenant_id("bad").expect_err("invalid tenant id should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("tenant_id"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn validate_requested_at_uses_api_field_name() {
        let error =
            validate_requested_at("2026-06-01").expect_err("invalid requestedAt should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("requestedAt"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn validate_standard_id_rejects_non_standard_values() {
        let error = validate_standard_id("Provider.Model", "providerId", Some("provider."))
            .expect_err("uppercase standard id should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("providerId"));
                assert!(message.contains("lowercase standard id characters"));
            }
            _ => panic!("expected validation error"),
        }

        let error = validate_standard_id("provider.", "providerId", Some("provider."))
            .expect_err("prefix-only standard id should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("providerId"));
                assert!(message.contains("dot-delimited"));
            }
            _ => panic!("expected validation error"),
        }

        let error = validate_standard_id("provider..rig", "providerId", Some("provider."))
            .expect_err("empty standard id segment should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("providerId"));
                assert!(message.contains("dot-delimited"));
            }
            _ => panic!("expected validation error"),
        }

        let error = validate_standard_id("model.rig", "providerId", Some("provider."))
            .expect_err("missing standard prefix should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("providerId"));
                assert!(message.contains("provider."));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn validate_capabilities_rejects_non_standard_values() {
        let capabilities = vec!["model.chat".to_string(), "model.chat".to_string()];
        let error = validate_capabilities(&capabilities, "capabilities")
            .expect_err("duplicate capability should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("capabilities"));
                assert!(message.contains("duplicate"));
            }
            _ => panic!("expected validation error"),
        }

        let capabilities = vec!["chat".to_string()];
        let error = validate_capabilities(&capabilities, "capabilities")
            .expect_err("unnamespaced capability should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("capabilities"));
                assert!(message.contains("namespaced"));
            }
            _ => panic!("expected validation error"),
        }

        let capabilities = vec!["model.".to_string()];
        let error = validate_capabilities(&capabilities, "capabilities")
            .expect_err("capability with empty segment should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("capabilities"));
                assert!(message.contains("namespaced"));
            }
            _ => panic!("expected validation error"),
        }

        let capabilities = vec![format!("model.{}", "a".repeat(123))];
        let error = validate_capabilities(&capabilities, "capabilities")
            .expect_err("capability over 128 characters should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("capabilities"));
                assert!(message.contains("at most 128"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn parse_expected_version_uses_api_field_name() {
        let error = parse_expected_version("1x").expect_err("invalid expectedVersion should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("expectedVersion"));
            }
            _ => panic!("expected validation error"),
        }
    }
}
