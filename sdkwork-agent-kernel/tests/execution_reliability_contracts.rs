//! Contract tests for tool schema documents and execution reliability.
//!
//! `ToolSchema` carries the JSON Schema body so tool and skill contracts are
//! self-describing; `AgentExecutionRequest` supports cooperative
//! cancellation, relative deadlines, and retry policies enforced at phase
//! boundaries.

use sdkwork_agent_kernel::{
    AgentExecutionRequest, AgentExecutionService, AgentExecutionStatus, AgentManifest,
    AgentStreamEvent, AgentStreamSink, CancellationToken, InMemoryAgentStreamSink, KernelError,
    KernelErrorKind, KernelResult, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    ModelUsage, ProviderHealth, ProviderManifest, RetryConfig, RuntimeBuilder, ToolSchema,
};
use std::time::Duration;

/// Model provider that fails the first N invocations then succeeds, so the
/// retry contract can be observed.
#[derive(Clone)]
struct FlakyModelProvider {
    provider_id: String,
    failures_before_success: std::sync::Arc<std::sync::Mutex<u32>>,
}

impl FlakyModelProvider {
    fn new(provider_id: &str, failures: u32) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            failures_before_success: std::sync::Arc::new(std::sync::Mutex::new(failures)),
        }
    }
}

impl ModelProvider for FlakyModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "flaky-model",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let mut remaining = self.failures_before_success.lock().unwrap();
        if *remaining > 0 {
            *remaining -= 1;
            return Err(KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            });
        }
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            "recovered response",
        )
        .with_usage(ModelUsage::new(5, 2)))
    }
}

const RELIABILITY_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.reliability",
  "name": "sdkwork-reliability-agent",
  "display_name": "SDKWork Reliability Agent",
  "description": "Agent used to prove execution reliability contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "policy.evaluate",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.execution.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

fn reliability_runtime(failures: u32) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.reliability",
        AgentManifest::from_json(RELIABILITY_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.flaky",
        "0.1.0",
        FlakyModelProvider::new("provider.flaky", failures),
    )
    .register_policy_provider("provider.policy.reliability", "0.1.0", AllowPolicyProvider)
    .bootstrap()
    .expect("reliability runtime bootstraps")
    .runtime
}

#[derive(Clone)]
struct AllowPolicyProvider;

impl sdkwork_agent_kernel::PolicyProvider for AllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.reliability",
            "policy",
            "allow-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn evaluate(
        &self,
        request: sdkwork_agent_kernel::PolicyRequest,
    ) -> KernelResult<sdkwork_agent_kernel::PolicyDecision> {
        Ok(sdkwork_agent_kernel::PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            "provider.policy.reliability",
        ))
    }
}

#[test]
fn tool_schema_carries_json_schema_document() {
    let schema = ToolSchema::json_schema("tool.schema.search.v1")
        .with_document(serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1}
            },
            "required": ["query"]
        }))
        .with_dialect("https://json-schema.org/draft/2020-12/schema");

    assert_eq!(schema.schema_id, "tool.schema.search.v1");
    assert!(schema.has_document());
    assert_eq!(
        schema.dialect.as_deref(),
        Some("https://json-schema.org/draft/2020-12/schema")
    );

    let document = schema.document_json().expect("document parses");
    assert_eq!(document["type"], "object");
    assert_eq!(document["required"][0], "query");
}

#[test]
fn tool_schema_without_document_is_a_registry_reference() {
    let schema = ToolSchema::json_schema("tool.schema.registry-only");
    assert!(!schema.has_document());
    assert!(schema.document_json().is_none());
}

#[test]
fn tool_schema_round_trips_raw_document_text() {
    let schema = ToolSchema::json_schema("tool.schema.raw")
        .with_document_text(r#"{"type":"object","properties":{"path":{"type":"string"}}}"#);
    let document = schema.document_json().expect("raw text parses");
    assert_eq!(document["properties"]["path"]["type"], "string");
}

#[test]
fn model_response_format_carries_schema_document() {
    let format = ModelResponseFormat::json_schema_with_document(
        "output.schema.v1",
        serde_json::json!({"type": "object"}),
    );
    assert_eq!(format.schema_id(), Some("output.schema.v1"));
    assert_eq!(format.document(), Some(r#"{"type":"object"}"#));

    let reference = ModelResponseFormat::json_schema("output.schema.v2");
    assert_eq!(reference.schema_id(), Some("output.schema.v2"));
    assert_eq!(reference.document(), None);
}

#[test]
fn execution_aborts_before_model_round_when_cancelled() {
    let runtime = reliability_runtime(0);
    let token = CancellationToken::new("token.exec.pre");
    token.cancel();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("exec.cancel.pre", vec!["hello".to_string()])
                .with_cancellation(token),
        )
        .expect("cancelled execution returns a report");

    assert_eq!(report.status, AgentExecutionStatus::Cancelled);
    assert!(report.model_response.is_none());
}

#[test]
fn execution_aborts_before_model_round_when_deadline_passed() {
    let runtime = reliability_runtime(0);

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("exec.deadline.pre", vec!["hello".to_string()])
                .with_deadline_ms(0),
        )
        .expect("timed-out execution returns a report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
    let error = report.error.as_ref().expect("report carries error");
    assert_eq!(error.kind(), KernelErrorKind::Timeout);
}

#[test]
fn execution_retry_recovers_from_transient_provider_failures() {
    let runtime = reliability_runtime(2);
    let config = RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(5),
        ..Default::default()
    };

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("exec.retry.1", vec!["hello".to_string()])
                .with_retry(config),
        )
        .expect("retry recovers and returns a report");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    let model_response = report.model_response.expect("model responded");
    assert_eq!(model_response.messages, vec!["recovered response"]);
}

#[test]
fn execution_without_retry_fails_on_transient_provider_failures() {
    let runtime = reliability_runtime(2);

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("exec.retry.0", vec!["hello".to_string()]),
        )
        .expect("failed execution returns a report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
}

#[test]
fn streaming_execution_emits_cancelled_terminal_sequence() {
    let runtime = reliability_runtime(0);
    let token = CancellationToken::new("token.exec.stream");
    token.cancel();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            AgentExecutionRequest::new("exec.stream.cancel", vec!["hello".to_string()])
                .with_cancellation(token)
                .for_session("session.cancel"),
            &mut sink,
        )
        .expect("cancelled stream returns");

    let types: Vec<&str> = sink.events().iter().map(|e| e.event_type()).collect();
    assert_eq!(types[0], "agent.stream.session.init");
    assert_eq!(types[1], "agent.stream.error");
    assert_eq!(types[2], "agent.stream.result");
    assert_eq!(types[3], "agent.stream.ended");

    match &sink.events()[2] {
        AgentStreamEvent::Result(result) => {
            assert!(result.is_error);
            assert!(result.result.contains("cancelled"));
        }
        other => panic!("expected Result, got {:?}", other.event_type()),
    }
}

#[test]
fn execution_request_builders_cover_reliability_options() {
    let token = CancellationToken::new("token.builder");
    let config = RetryConfig::no_retry();

    let request = AgentExecutionRequest::new("exec.builder", vec!["hi".to_string()])
        .with_cancellation(token)
        .with_deadline_ms(5_000)
        .with_retry(config);

    assert!(request.cancellation_token.is_some());
    assert_eq!(request.deadline_ms, Some(5_000));
    assert!(request.retry.is_some());
}
