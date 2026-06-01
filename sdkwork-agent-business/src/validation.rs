use sdkwork_agent_kernel::{KernelError, KernelResult};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub(crate) fn parse_int64_string_field(value: &str, field_name: &str) -> KernelResult<u64> {
    value
        .parse::<u64>()
        .map_err(|_| KernelError::validation(format!("{field_name} must be int64 string")))
}

pub(crate) fn validate_rfc3339_datetime(value: &str, field_name: &str) -> KernelResult<()> {
    let _ = parse_rfc3339_datetime(value, field_name)?;
    Ok(())
}

pub(crate) fn parse_rfc3339_datetime(
    value: &str,
    field_name: &str,
) -> KernelResult<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| KernelError::validation(format!("{field_name} must be RFC3339 date-time: {error}")))
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
}
