//! Structured commercial usage facts for model token consumption (`OBSERVABILITY_SPEC.md`).

use sdkwork_agent_kernel::ModelUsage;
use tracing::info;

/// Record a durable usage fact for downstream billing aggregation.
pub fn record_model_token_usage(
    tenant_id: Option<&str>,
    user_id: Option<&str>,
    session_id: &str,
    provider_id: &str,
    usage: &ModelUsage,
) {
    info!(
        target: "usage_meter",
        event = "model.tokens",
        tenant_id = tenant_id.unwrap_or(""),
        user_id = user_id.unwrap_or(""),
        session_id = session_id,
        provider_id = provider_id,
        input_tokens = usage.input_tokens,
        output_tokens = usage.output_tokens,
        total_tokens = usage.total_tokens(),
        "commercial usage fact"
    );
}
