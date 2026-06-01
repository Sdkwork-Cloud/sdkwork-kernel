use sdkwork_agent_kernel::{KernelError, KernelResult};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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
}
