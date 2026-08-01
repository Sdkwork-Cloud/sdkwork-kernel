//! Contract tests: sandbox-aware streaming execution.
//!
//! `AgentExecutionService::execute_streaming_sandboxed` runs the
//! synchronous streaming core inside the bound sandbox session lifecycle
//! and surfaces `agent.stream.sandbox` events (pending -> completed /
//! failed) in the stream. Without a binding it is the plain synchronous
//! path; with a missing session it refuses execution before any model
//! work runs.

use std::sync::Arc;
use std::sync::Mutex;

use sdkwork_agent_kernel::{
    AgentExecutionRequest, AgentExecutionService, AgentManifest, AgentStreamEvent, AgentStreamSink,
    InMemoryAgentStreamSink, KernelResult, PolicyRequest, ProviderHealth, ProviderManifest,
    RuntimeBuilder, SandboxEventPhase, SandboxExecutionBinding, SandboxSessionCommandRequest,
    SandboxSessionRuntimeProjection, SandboxSessionState, SandboxedExecutionCoordinator,
    SandboxedSessionPort, SideEffectLevel, ToolCall, ToolDescriptor, ToolProvider,
};

const SANDBOX_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.sandbox",
  "name": "sdkwork-sandbox-agent",
  "display_name": "SDKWork Sandbox Agent",
  "description": "Agent used to prove sandboxed streaming execution contracts.",
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

/// Stateful model provider: emits tool calls until exhausted, then a
/// final answer, so the multi-turn loop terminates deterministically.
#[derive(Clone)]
struct SandboxToolModelProvider {
    provider_id: String,
    tool_calls_before_text: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl SandboxToolModelProvider {
    fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            tool_calls_before_text: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        }
    }
}

impl sdkwork_agent_kernel::ModelProvider for SandboxToolModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "sandbox-tool-model",
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
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |value| Some(value.saturating_sub(1)),
            )
            .unwrap_or(0);
        let mut response = sdkwork_agent_kernel::ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            if remaining > 0 {
                "tool call requested"
            } else {
                "final sandboxed answer"
            },
        )
        .with_usage(sdkwork_agent_kernel::ModelUsage::new(20, 3));
        if remaining > 0 {
            response = response.with_tool_call(
                ToolCall::new(
                    "tool-call.sandbox.1",
                    "tool.sandbox.search",
                    r#"{"query":"sandboxed"}"#,
                )
                .with_provider("provider.tool.sandbox"),
            );
        }
        Ok(response)
    }
}

#[derive(Clone)]
struct SandboxStaticToolProvider;

impl ToolProvider for SandboxStaticToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.sandbox",
            "tool",
            "static-tool",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.sandbox.search",
            "provider.tool.sandbox",
            "search",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn invoke_tool(&self, tool_call: ToolCall) -> KernelResult<sdkwork_agent_kernel::ToolResult> {
        Ok(sdkwork_agent_kernel::ToolResult::succeeded(
            tool_call.tool_call_id,
            "sandboxed search result",
        ))
    }
}

struct SandboxAllowPolicyProvider;

impl sdkwork_agent_kernel::PolicyProvider for SandboxAllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.sandbox",
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
        request: PolicyRequest,
    ) -> KernelResult<sdkwork_agent_kernel::PolicyDecision> {
        Ok(sdkwork_agent_kernel::PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            "provider.policy.sandbox",
        ))
    }
}

fn sandbox_runtime() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.sandbox-stream",
        AgentManifest::from_json(SANDBOX_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.model.sandbox",
        "0.1.0",
        SandboxToolModelProvider::new("provider.model.sandbox"),
    )
    .register_tool_provider("provider.tool.sandbox", "0.1.0", SandboxStaticToolProvider)
    .register_policy_provider(
        "provider.policy.sandbox",
        "0.1.0",
        SandboxAllowPolicyProvider,
    )
    .bootstrap()
    .expect("sandbox runtime bootstraps")
    .runtime
}

/// Recording double for the kernel-side sandbox lifecycle port.
struct RecordingSandboxPort {
    calls: Mutex<Vec<String>>,
    session_state: SandboxSessionState,
    session_found: bool,
}

impl RecordingSandboxPort {
    fn running() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            session_state: SandboxSessionState::Running,
            session_found: true,
        }
    }

    fn created() -> Self {
        Self {
            session_state: SandboxSessionState::Created,
            ..Self::running()
        }
    }

    fn missing() -> Self {
        Self {
            session_found: false,
            ..Self::running()
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .clone()
    }
}

fn projection(state: SandboxSessionState) -> SandboxSessionRuntimeProjection {
    SandboxSessionRuntimeProjection::new(
        "agent-workspace-1",
        "agent-session-1",
        state,
        Some("sandbox-1"),
        Some("binding-1"),
        Some("provider.sandbox.acme"),
        Some("location-1"),
    )
}

#[async_trait::async_trait]
impl sdkwork_agent_kernel::SandboxedSessionPort for RecordingSandboxPort {
    async fn get_sandbox_session(
        &self,
        tenant_id: String,
        agent_session_id: String,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .push(format!("get:{tenant_id}:{agent_session_id}"));
        if !self.session_found {
            return Err(sdkwork_agent_kernel::KernelError::validation(
                "sandbox session was not found",
            ));
        }
        Ok(projection(self.session_state))
    }

    async fn start_sandbox_session(
        &self,
        request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .push(format!(
                "start:{}:{}",
                request.tenant_id, request.agent_session_id
            ));
        Ok(projection(SandboxSessionState::Running))
    }

    async fn stop_sandbox_session(
        &self,
        request: SandboxSessionCommandRequest,
    ) -> KernelResult<SandboxSessionRuntimeProjection> {
        self.calls
            .lock()
            .unwrap_or_else(|poisoned_state| poisoned_state.into_inner())
            .push(format!(
                "stop:{}:{}",
                request.tenant_id, request.agent_session_id
            ));
        Ok(projection(SandboxSessionState::Stopped))
    }
}

fn sandbox_events(events: &[AgentStreamEvent]) -> Vec<(String, SandboxEventPhase)> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentStreamEvent::Sandbox(sandbox_event) => Some((
                sandbox_event.sandbox_session_id.clone(),
                sandbox_event.phase,
            )),
            _ => None,
        })
        .collect()
}

fn execution_request() -> AgentExecutionRequest {
    AgentExecutionRequest::new("exec.sandbox.1", vec!["run in sandbox".to_string()])
        .for_session("session-1")
        .with_provider_id("provider.model.sandbox")
}

#[tokio::test]
async fn sandboxed_streaming_emits_lifecycle_events_around_rounds() {
    let port = Arc::new(RecordingSandboxPort::created());
    let runtime = sandbox_runtime();
    let mut sink = InMemoryAgentStreamSink::default();
    let binding = SandboxExecutionBinding::new("agent-session-1")
        .with_auto_start()
        .with_auto_stop();

    AgentExecutionService::new()
        .execute_streaming_sandboxed(
            &runtime,
            execution_request().with_sandbox_binding(binding),
            &mut sink,
            port.clone(),
        )
        .await
        .expect("sandboxed streaming succeeds");

    let lifecycle = sandbox_events(sink.events());
    assert_eq!(
        lifecycle,
        vec![
            ("agent-session-1".to_string(), SandboxEventPhase::Pending),
            ("agent-session-1".to_string(), SandboxEventPhase::Completed),
        ],
        "pending -> completed lifecycle in stream order"
    );

    // The rounds core still emits the full model/tool/result surface.
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(event, AgentStreamEvent::Result(_))));
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(event, AgentStreamEvent::ToolResult(_))));
    assert_eq!(
        port.calls(),
        vec![
            "get:system:agent-session-1".to_string(),
            "start:system:agent-session-1".to_string(),
            "stop:system:agent-session-1".to_string(),
        ]
    );
}

#[tokio::test]
async fn sandboxed_streaming_with_running_session_skips_start() {
    let port = Arc::new(RecordingSandboxPort::running());
    let runtime = sandbox_runtime();
    let mut sink = InMemoryAgentStreamSink::default();
    let binding = SandboxExecutionBinding::new("agent-session-1").with_auto_stop();

    AgentExecutionService::new()
        .execute_streaming_sandboxed(
            &runtime,
            execution_request().with_sandbox_binding(binding),
            &mut sink,
            port.clone(),
        )
        .await
        .expect("sandboxed streaming succeeds");

    assert_eq!(
        port.calls(),
        vec![
            "get:system:agent-session-1".to_string(),
            "stop:system:agent-session-1".to_string(),
        ],
        "running session skips start"
    );
}

#[tokio::test]
async fn missing_sandbox_session_refuses_execution_with_failed_event() {
    let port = Arc::new(RecordingSandboxPort::missing());
    let runtime = sandbox_runtime();
    let mut sink = InMemoryAgentStreamSink::default();
    let binding = SandboxExecutionBinding::new("agent-session-1").with_auto_start();

    let result = AgentExecutionService::new()
        .execute_streaming_sandboxed(
            &runtime,
            execution_request().with_sandbox_binding(binding),
            &mut sink,
            port.clone(),
        )
        .await;

    assert!(result.is_err(), "missing session must refuse execution");
    assert_eq!(
        sandbox_events(sink.events()),
        vec![
            ("agent-session-1".to_string(), SandboxEventPhase::Pending),
            ("agent-session-1".to_string(), SandboxEventPhase::Failed),
        ]
    );
    assert!(
        !sink
            .events()
            .iter()
            .any(|event| matches!(event, AgentStreamEvent::Result(_))),
        "no model round may run when the session is missing"
    );
    assert!(
        !sink
            .events()
            .iter()
            .any(|event| matches!(event, AgentStreamEvent::MessageStart(_))),
        "no message may start when the session is missing"
    );
}

#[tokio::test]
async fn without_binding_sandboxed_entry_is_plain_streaming() {
    let runtime = sandbox_runtime();
    let mut sink = InMemoryAgentStreamSink::default();

    AgentExecutionService::new()
        .execute_streaming_sandboxed(
            &runtime,
            execution_request(),
            &mut sink,
            Arc::new(RecordingSandboxPort::running()),
        )
        .await
        .expect("plain streaming succeeds");

    assert!(
        sandbox_events(sink.events()).is_empty(),
        "no sandbox events without a binding"
    );
    assert!(sink
        .events()
        .iter()
        .any(|event| matches!(event, AgentStreamEvent::Result(_))));
}

#[tokio::test]
async fn tenant_id_flows_from_subject_into_sandbox_commands() {
    let port = Arc::new(RecordingSandboxPort::created());
    let runtime = sandbox_runtime();
    let mut sink = InMemoryAgentStreamSink::default();
    let binding = SandboxExecutionBinding::new("agent-session-1")
        .with_auto_start()
        .with_auto_stop();

    let request = execution_request()
        .with_subject(sdkwork_agent_kernel::PolicySubject::new(
            "subject-1",
            "tenant-42",
        ))
        .with_sandbox_binding(binding);

    AgentExecutionService::new()
        .execute_streaming_sandboxed(&runtime, request, &mut sink, port.clone())
        .await
        .expect("sandboxed streaming succeeds");

    assert!(
        port.calls().iter().all(|call| call.contains("tenant-42")),
        "tenant id flows from the subject"
    );
}
