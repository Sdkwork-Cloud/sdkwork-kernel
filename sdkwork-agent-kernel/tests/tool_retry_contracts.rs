//! Contract tests: read-only tool call retries in streaming execution.
//!
//! `AgentExecutionRequest.tool_retry` retries kernel-level failures of
//! read-only tools (provider unavailable / resource exhausted / timeout)
//! with backoff, surfacing a warn status event per attempt. Side-effectful
//! or unknown tools never retry to avoid replaying effects; a tool that
//! returns a normal failure result is not retried either.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sdkwork_agent_kernel::{
    AgentExecutionRequest, AgentExecutionService, AgentManifest, AgentStreamEvent,
    InMemoryAgentStreamSink, KernelError, KernelResult, ProviderHealth, ProviderManifest,
    RetryConfig, RuntimeBuilder, SideEffectLevel, ToolCall, ToolDescriptor, ToolProvider,
};

const RETRY_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.retry",
  "name": "sdkwork-retry-agent",
  "display_name": "SDKWork Retry Agent",
  "description": "Agent used to prove read-only tool retry contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    { "capability_id": "model.chat", "min_version": "0.1.0" },
    { "capability_id": "model.streaming", "min_version": "0.1.0" },
    { "capability_id": "model.tool_call", "min_version": "0.1.0" },
    { "capability_id": "policy.evaluate", "min_version": "0.1.0" }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.stream.*"],
  "owner": { "name": "sdkwork-platform" },
  "status": "candidate"
}
"#;

#[derive(Clone)]
struct RetryToolModelProvider {
    provider_id: String,
    tool_calls_before_text: Arc<AtomicUsize>,
}

impl RetryToolModelProvider {
    fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            tool_calls_before_text: Arc::new(AtomicUsize::new(1)),
        }
    }
}

impl sdkwork_agent_kernel::ModelProvider for RetryToolModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "retry-tool-model",
            "0.1.0",
            vec![
                "model.chat".to_string(),
                "model.streaming".to_string(),
                "model.tool_call".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(
        &self,
        request: sdkwork_agent_kernel::ModelRequest,
    ) -> KernelResult<sdkwork_agent_kernel::ModelResponse> {
        let remaining = self
            .tool_calls_before_text
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                Some(v.saturating_sub(1))
            })
            .unwrap_or(0);
        let mut response = sdkwork_agent_kernel::ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            if remaining > 0 {
                "tool call requested"
            } else {
                "final retry answer"
            },
        )
        .with_usage(sdkwork_agent_kernel::ModelUsage::new(20, 3));
        if remaining > 0 {
            response = response.with_tool_call(
                ToolCall::new(
                    "tool-call.retry.1",
                    "tool.retry.search",
                    r#"{"query":"retry"}"#,
                )
                .with_provider("provider.tool.retry"),
            );
        }
        Ok(response)
    }
}

/// Read-only tool that fails with a provider error the first N calls,
/// then succeeds; counts every invocation.
#[derive(Clone)]
struct FlakyReadOnlyToolProvider {
    provider_id: String,
    failures_before_success: Arc<AtomicUsize>,
    invocation_count: Arc<AtomicUsize>,
}

impl FlakyReadOnlyToolProvider {
    fn new(provider_id: &str, failures_before_success: usize) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            failures_before_success: Arc::new(AtomicUsize::new(failures_before_success)),
            invocation_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn invocation_count(&self) -> usize {
        self.invocation_count.load(Ordering::SeqCst)
    }
}

impl ToolProvider for FlakyReadOnlyToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "tool",
            "flaky-tool",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.retry.search",
            self.provider_id.clone(),
            "search",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn invoke_tool(&self, tool_call: ToolCall) -> KernelResult<sdkwork_agent_kernel::ToolResult> {
        self.invocation_count.fetch_add(1, Ordering::SeqCst);
        let remaining = self
            .failures_before_success
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                Some(v.saturating_sub(1))
            })
            .unwrap_or(0);
        if remaining > 0 {
            return Err(KernelError::ProviderUnavailable {
                provider_id: self.provider_id.clone(),
            });
        }
        Ok(sdkwork_agent_kernel::ToolResult::succeeded(
            tool_call.tool_call_id,
            "retried search results",
        ))
    }
}

/// Always-failing tool for exhaustion and side-effect tests.
#[derive(Clone)]
struct AlwaysFailingToolProvider {
    provider_id: String,
    side_effect_level: SideEffectLevel,
    invocation_count: Arc<AtomicUsize>,
}

impl AlwaysFailingToolProvider {
    fn read_only(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            side_effect_level: SideEffectLevel::ReadOnly,
            invocation_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn write(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            side_effect_level: SideEffectLevel::SideEffectful,
            invocation_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn invocation_count(&self) -> usize {
        self.invocation_count.load(Ordering::SeqCst)
    }
}

impl ToolProvider for AlwaysFailingToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "tool",
            "failing-tool",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.retry.search",
            self.provider_id.clone(),
            "search",
            self.side_effect_level,
        )]
    }

    fn invoke_tool(&self, _tool_call: ToolCall) -> KernelResult<sdkwork_agent_kernel::ToolResult> {
        self.invocation_count.fetch_add(1, Ordering::SeqCst);
        Err(KernelError::ProviderUnavailable {
            provider_id: self.provider_id.clone(),
        })
    }
}

struct RetryAllowPolicyProvider;

impl sdkwork_agent_kernel::PolicyProvider for RetryAllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.retry",
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
            "provider.policy.retry",
        ))
    }
}

fn tool_retry_config() -> RetryConfig {
    RetryConfig {
        max_retries: 3,
        initial_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
        backoff_factor: 2.0,
        jitter: false,
        retryable_errors: Vec::new(),
    }
}

fn execution_request() -> AgentExecutionRequest {
    AgentExecutionRequest::new("exec.retry.1", vec!["retry me".to_string()])
        .for_session("session-1")
        .with_provider_id("provider.retry")
}

#[tokio::test]
async fn flaky_read_only_tool_is_retried_until_success() {
    let tool_provider = FlakyReadOnlyToolProvider::new("provider.tool.retry", 2);
    let runtime = RuntimeBuilder::new(
        "runtime.retry",
        AgentManifest::from_json(RETRY_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.retry",
        "0.1.0",
        RetryToolModelProvider::new("provider.retry"),
    )
    .register_tool_provider("provider.tool.retry", "0.1.0", tool_provider.clone())
    .register_policy_provider("provider.policy.retry", "0.1.0", RetryAllowPolicyProvider)
    .bootstrap()
    .expect("retry runtime bootstraps")
    .runtime;

    let mut sink = InMemoryAgentStreamSink::default();
    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            execution_request().with_tool_retry(tool_retry_config()),
            &mut sink,
        )
        .expect("streaming succeeds");

    assert_eq!(
        tool_provider.invocation_count(),
        3,
        "two failures then a success"
    );
    assert!(
        sink.events()
            .iter()
            .filter(|event| matches!(event, AgentStreamEvent::Status(_)))
            .count()
            >= 2,
        "each retry surfaces a warn status event"
    );
    // The retried tool ultimately succeeded, so the run is not tool-failed.
    let result = sink
        .events()
        .iter()
        .find_map(|event| match event {
            AgentStreamEvent::Result(result) => Some(result),
            _ => None,
        })
        .expect("result event");
    assert!(!result.is_error);
}

#[tokio::test]
async fn tool_retry_exhausts_attempts_on_persistent_failure() {
    let tool_provider = AlwaysFailingToolProvider::read_only("provider.tool.retry");
    let runtime = RuntimeBuilder::new(
        "runtime.retry",
        AgentManifest::from_json(RETRY_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.retry",
        "0.1.0",
        RetryToolModelProvider::new("provider.retry"),
    )
    .register_tool_provider("provider.tool.retry", "0.1.0", tool_provider.clone())
    .register_policy_provider("provider.policy.retry", "0.1.0", RetryAllowPolicyProvider)
    .bootstrap()
    .expect("retry runtime bootstraps")
    .runtime;

    let mut sink = InMemoryAgentStreamSink::default();
    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            execution_request().with_tool_retry(tool_retry_config()),
            &mut sink,
        )
        .expect("streaming completes with the tool failure in history");

    assert_eq!(
        tool_provider.invocation_count(),
        1 + 3,
        "initial attempt plus three retries"
    );
}

#[tokio::test]
async fn write_tool_is_never_retried() {
    let tool_provider = AlwaysFailingToolProvider::write("provider.tool.retry");
    let runtime = RuntimeBuilder::new(
        "runtime.retry",
        AgentManifest::from_json(RETRY_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.retry",
        "0.1.0",
        RetryToolModelProvider::new("provider.retry"),
    )
    .register_tool_provider("provider.tool.retry", "0.1.0", tool_provider.clone())
    .register_policy_provider("provider.policy.retry", "0.1.0", RetryAllowPolicyProvider)
    .bootstrap()
    .expect("retry runtime bootstraps")
    .runtime;

    let mut sink = InMemoryAgentStreamSink::default();
    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            execution_request().with_tool_retry(tool_retry_config()),
            &mut sink,
        )
        .expect("streaming completes");

    assert_eq!(
        tool_provider.invocation_count(),
        1,
        "write tools must not be replayed"
    );
}

#[tokio::test]
async fn without_tool_retry_policy_there_is_no_retry() {
    let tool_provider = FlakyReadOnlyToolProvider::new("provider.tool.retry", 2);
    let runtime = RuntimeBuilder::new(
        "runtime.retry",
        AgentManifest::from_json(RETRY_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.retry",
        "0.1.0",
        RetryToolModelProvider::new("provider.retry"),
    )
    .register_tool_provider("provider.tool.retry", "0.1.0", tool_provider.clone())
    .register_policy_provider("provider.policy.retry", "0.1.0", RetryAllowPolicyProvider)
    .bootstrap()
    .expect("retry runtime bootstraps")
    .runtime;

    let mut sink = InMemoryAgentStreamSink::default();
    AgentExecutionService::new()
        .execute_streaming(&runtime, execution_request(), &mut sink)
        .expect("streaming completes");

    assert_eq!(
        tool_provider.invocation_count(),
        1,
        "no retry policy means a single attempt"
    );
    assert!(
        !sink
            .events()
            .iter()
            .any(|event| matches!(event, AgentStreamEvent::Status(_))),
        "no retry status events without a tool retry policy"
    );
}
