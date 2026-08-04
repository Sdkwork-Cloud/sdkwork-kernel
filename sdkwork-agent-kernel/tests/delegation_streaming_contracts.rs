//! Contract tests for streaming sub-agent delegation.
//!
//! `AgentDelegationService` runs a delegated task on a sub-agent session
//! through the kernel execution loop and relays the child stream to the
//! parent, tagging child messages with the parent tool call id
//! (agent-as-tool semantics: codex `spawn_agent`, claude `Task` tool,
//! hermes `delegate_task`).

use sdkwork_agent_kernel::{
    AgentDelegationService, AgentDelegationStreamRequest, AgentExecutionRequest,
    AgentExecutionService, AgentManifest, AgentStreamEvent, AgentStreamSink,
    InMemoryAgentStreamSink, KernelResult, ModelProvider, ModelRequest, ModelResponse, ModelUsage,
    ProviderHealth, ProviderManifest, RuntimeBuilder, ToolCall, ToolCallStatus, ToolDescriptor,
    ToolProvider, ToolResult,
};

const DELEGATION_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.delegation",
  "name": "sdkwork-delegation-agent",
  "display_name": "SDKWork Delegation Agent",
  "description": "Agent used to prove streaming delegation contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "model.tool_call",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "policy.evaluate",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.stream.*", "agent.collaboration.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

/// Model provider: first invoke requests a tool call (so the child run
/// exercises the tool loop), then returns a plain final answer.
#[derive(Clone)]
struct DelegatingModelProvider {
    provider_id: String,
    invocations: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl DelegatingModelProvider {
    fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            invocations: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl ModelProvider for DelegatingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "delegating-model",
            "0.1.0",
            vec![
                "model.chat".to_string(),
                "model.tool_call".to_string(),
                "model.streaming".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let call = self
            .invocations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut response = ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            if call == 0 {
                "child tool round"
            } else {
                "child final answer"
            },
        )
        .with_usage(ModelUsage::new(10, 5));
        if call == 0 {
            response = response.with_tool_call(
                ToolCall::new(
                    "child-tool-call.1",
                    "tool.delegation.search",
                    r#"{"query":"x"}"#,
                )
                .with_provider("provider.tool.delegation"),
            );
        }
        Ok(response)
    }
}

#[derive(Clone)]
struct DelegationToolProvider {
    provider_id: String,
}

impl ToolProvider for DelegationToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "tool",
            "delegation-tool",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.delegation.search",
            "provider.tool.delegation",
            "search",
            sdkwork_agent_kernel::SideEffectLevel::ReadOnly,
        )]
    }

    fn invoke_tool(&self, tool_call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(
            tool_call.tool_call_id,
            "child search results",
        ))
    }
}

#[derive(Clone)]
struct AllowPolicyProvider;

impl sdkwork_agent_kernel::PolicyProvider for AllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.delegation",
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
            "provider.policy.delegation",
        ))
    }
}

fn delegation_runtime() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.delegation",
        AgentManifest::from_json(DELEGATION_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.delegation",
        "0.1.0",
        DelegatingModelProvider::new("provider.delegation"),
    )
    .register_tool_provider(
        "provider.tool.delegation",
        "0.1.0",
        DelegationToolProvider {
            provider_id: "provider.tool.delegation".to_string(),
        },
    )
    .register_policy_provider("provider.policy.delegation", "0.1.0", AllowPolicyProvider)
    .bootstrap()
    .expect("delegation runtime bootstraps")
    .runtime
}

#[test]
fn delegation_stream_request_builders_cover_tool_call_origin() {
    let request = AgentDelegationStreamRequest::from_tool_call(
        "delegation.1",
        "session.parent",
        "tool-call.delegate.7",
        "review the change",
    )
    .with_provider_id("provider.delegation")
    .with_model_id("claude-sonnet-4")
    .with_timeout_ms(30_000);

    assert_eq!(request.delegation_id, "delegation.1");
    assert_eq!(request.source_session_id, "session.parent");
    assert_eq!(
        request.tool_call_id.as_deref(),
        Some("tool-call.delegate.7")
    );
    assert_eq!(request.provider_id.as_deref(), Some("provider.delegation"));
    assert_eq!(request.model_id.as_deref(), Some("claude-sonnet-4"));
    assert_eq!(request.timeout_ms, Some(30_000));
}

#[test]
fn delegation_stream_relays_child_events_with_parent_chain() {
    let runtime = delegation_runtime();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentDelegationService::new()
        .delegate_streaming(
            &runtime,
            AgentDelegationStreamRequest::from_tool_call(
                "delegation.stream.1",
                "session.parent",
                "tool-call.delegate.7",
                "review the change",
            ),
            &mut sink,
        )
        .expect("delegation stream succeeds");

    let types: Vec<&str> = sink.events().iter().map(|e| e.event_type()).collect();

    // Task lifecycle: spawned -> child session init -> child stream ->
    // completed.
    assert_eq!(types[0], "agent.stream.progress");
    assert_eq!(types[1], "agent.stream.session.init");
    assert_eq!(types[2], "agent.stream.message.start");
    assert!(types.contains(&"agent.stream.tool.result"));
    assert!(types.contains(&"agent.stream.result"));
    assert_eq!(types.last().unwrap(), &"agent.stream.progress");

    // Spawned/completed notices carry the parent session identity.
    match &sink.events()[0] {
        AgentStreamEvent::Progress(progress) => {
            assert_eq!(progress.label, "delegate.task_spawned");
        }
        other => panic!("expected spawned progress, got {:?}", other.event_type()),
    }
    match sink.events().last().unwrap() {
        AgentStreamEvent::Progress(progress) => {
            assert_eq!(progress.label, "delegate.task_completed");
        }
        other => panic!("expected completed progress, got {:?}", other.event_type()),
    }

    // Child message starts link back to the parent tool call.
    match &sink.events()[2] {
        AgentStreamEvent::MessageStart(start) => {
            assert_eq!(
                start.parent_message_id.as_deref(),
                Some("tool-call.delegate.7")
            );
        }
        other => panic!("expected message start, got {:?}", other.event_type()),
    }

    // Child terminal result reports its own turn count.
    let child_result = sink
        .events()
        .iter()
        .find_map(|event| match event {
            AgentStreamEvent::Result(result) => Some(result),
            _ => None,
        })
        .expect("child result present");
    assert_eq!(child_result.num_turns, 2);
    assert_eq!(child_result.result, "child final answer");
}

#[test]
fn delegation_stream_without_tool_call_origin_has_no_parent_link() {
    let runtime = delegation_runtime();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentDelegationService::new()
        .delegate_streaming(
            &runtime,
            AgentDelegationStreamRequest::new(
                "delegation.stream.2",
                "session.parent",
                "plain task",
            ),
            &mut sink,
        )
        .expect("delegation stream succeeds");

    match &sink.events()[2] {
        AgentStreamEvent::MessageStart(start) => {
            assert!(start.parent_message_id.is_none());
        }
        other => panic!("expected message start, got {:?}", other.event_type()),
    }
}

#[test]
fn delegation_uses_child_session_identity() {
    let runtime = delegation_runtime();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentDelegationService::new()
        .delegate_streaming(
            &runtime,
            AgentDelegationStreamRequest::new("delegation.session.1", "session.parent", "task"),
            &mut sink,
        )
        .expect("delegation stream succeeds");

    let init = sink
        .events()
        .iter()
        .find_map(|event| match event {
            AgentStreamEvent::SessionInit(init) => Some(init),
            _ => None,
        })
        .expect("child session init present");
    assert_eq!(
        init.session_id.as_deref(),
        Some("session.subagent.delegation.session.1")
    );
}

#[test]
fn execution_service_is_reusable_for_direct_child_runs() {
    // The child path is the standard execution loop; direct calls remain
    // supported for hosts that manage sub-sessions explicitly.
    let runtime = delegation_runtime();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            AgentExecutionRequest::new("exec.child.1", vec!["child task".to_string()])
                .for_session("session.subagent.explicit")
                .with_provider_id("provider.delegation"),
            &mut sink,
        )
        .expect("direct child run succeeds");

    let types: Vec<&str> = sink.events().iter().map(|e| e.event_type()).collect();
    assert_eq!(types[0], "agent.stream.session.init");
    assert_eq!(types.last().unwrap(), &"agent.stream.ended");
}
