//! Contract tests for the unified agent streaming event protocol.
//!
//! `AgentStreamEvent` is the discriminated-union stream protocol aligned with
//! the agent SDKs (`SDKMessage`, codex app-server notifications, opencode part
//! events). These tests pin the vocabulary, the chat/execution event
//! sequences, and the bridge into the generic `KernelEvent` envelope.

use sdkwork_agent_kernel::{
    AgentChatRequest, AgentChatService, AgentExecutionRequest, AgentExecutionService,
    AgentManifest, AgentMessageRole, AgentStreamEvent, AgentStreamSink, CancelledEvent, CostEvent,
    EndedEvent, ErrorEvent, InMemoryAgentStreamSink, KernelEventRedaction, KernelEventSource,
    KernelEventStreamSink, KernelResult, MessageDeltaEvent, MessageDeltaKind, MessageStartEvent,
    MessageStopEvent, ModelChunkKind, ModelProvider, ModelRequest, ModelResponse, ModelStreamChunk,
    ModelStreamSink, ProviderHealth, ProviderManifest, RateLimitEvent, RateLimitStatus,
    ResultEvent, RuntimeBuilder, SessionInitEvent, SideEffectLevel, StatusEvent, ToolCall,
    ToolCallStartEvent, ToolCallStatus, ToolCallStopEvent, ToolDescriptor, ToolProvider,
    ToolResult, ToolResultEvent, TraceContext, UsageEvent,
};
use std::sync::{Arc, Mutex};

const STREAM_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.stream",
  "name": "sdkwork-stream-agent",
  "display_name": "SDKWork Stream Agent",
  "description": "Agent used to prove unified streaming event contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "model.streaming",
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
  "event_families": ["agent.runtime.*", "agent.stream.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

/// Model provider that streams typed chunks: reasoning, text, and tool
/// argument deltas, then reports a completion response with usage.
#[derive(Clone)]
struct StreamingModelProvider {
    provider_id: String,
    chunks: Arc<Mutex<Vec<ModelStreamChunk>>>,
}

impl StreamingModelProvider {
    fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            chunks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ModelProvider for StreamingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "streaming-model",
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

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            "streamed response",
        )
        .with_usage(sdkwork_agent_kernel::ModelUsage::new(10, 5)))
    }

    fn stream(&self, request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        let chunks = self.stream_chunks(&request.model_request_id);
        Ok(chunks)
    }

    fn stream_into(
        &self,
        request: ModelRequest,
        sink: &mut dyn ModelStreamSink,
    ) -> KernelResult<()> {
        let chunks = self.stream_chunks(&request.model_request_id);
        self.chunks.lock().unwrap().extend(chunks.clone());
        for chunk in chunks {
            sink.push_chunk(chunk)?;
        }
        Ok(())
    }
}

impl StreamingModelProvider {
    fn stream_chunks(&self, model_request_id: &str) -> Vec<ModelStreamChunk> {
        vec![
            ModelStreamChunk::reasoning(model_request_id, 1, "thinking"),
            ModelStreamChunk::output(model_request_id, 2, "streamed"),
            ModelStreamChunk::output(model_request_id, 3, " response"),
        ]
    }
}

/// Model provider that returns a tool call from `invoke`, plus a matching
/// tool provider, to exercise the execution tool event lifecycle.
#[derive(Clone)]
struct ToolCallingModelProvider {
    provider_id: String,
}

impl ModelProvider for ToolCallingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "tool-calling-model",
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

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            "tool call requested",
        )
        .with_tool_call(
            ToolCall::new(
                "tool-call.from-model.1",
                "tool.stream.search",
                r#"{"query":"sdkwork"}"#,
            )
            .with_provider("provider.tool.stream"),
        )
        .with_usage(sdkwork_agent_kernel::ModelUsage::new(20, 3)))
    }
}

#[derive(Clone)]
struct StaticToolProvider {
    provider_id: String,
}

impl ToolProvider for StaticToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
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
            "tool.stream.search",
            "provider.tool.stream",
            "search",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn invoke_tool(&self, tool_call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(
            tool_call.tool_call_id,
            "search results",
        ))
    }
}

fn runtime_with_streaming_model() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.stream",
        AgentManifest::from_json(STREAM_AGENT_MANIFEST_JSON).expect("stream manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.model.stream",
        "0.1.0",
        StreamingModelProvider::new("provider.model.stream"),
    )
    .register_policy_provider("provider.policy.stream", "0.1.0", AllowPolicyProvider)
    .bootstrap()
    .expect("stream runtime bootstraps")
    .runtime
}

fn runtime_with_tool_calling() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.stream-tool",
        AgentManifest::from_json(STREAM_AGENT_MANIFEST_JSON).expect("stream manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_model_provider(
        "provider.model.tool",
        "0.1.0",
        ToolCallingModelProvider {
            provider_id: "provider.model.tool".to_string(),
        },
    )
    .register_tool_provider(
        "provider.tool.stream",
        "0.1.0",
        StaticToolProvider {
            provider_id: "provider.tool.stream".to_string(),
        },
    )
    .register_policy_provider("provider.policy.stream", "0.1.0", AllowPolicyProvider)
    .bootstrap()
    .expect("tool stream runtime bootstraps")
    .runtime
}

#[derive(Clone)]
struct AllowPolicyProvider;

impl sdkwork_agent_kernel::PolicyProvider for AllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.stream",
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
            "provider.policy.stream",
        ))
    }
}

#[test]
fn stream_event_vocabulary_is_stable() {
    let cases = vec![
        (
            AgentStreamEvent::SessionInit(sdkwork_agent_kernel::SessionInitEvent::new("e.1")),
            "agent.stream.session.init",
        ),
        (
            AgentStreamEvent::MessageStart(MessageStartEvent::new(
                "e.2",
                "msg.1",
                AgentMessageRole::Agent,
            )),
            "agent.stream.message.start",
        ),
        (
            AgentStreamEvent::MessageDelta(MessageDeltaEvent::text("e.3", "msg.1", "hi")),
            "agent.stream.message.delta",
        ),
        (
            AgentStreamEvent::MessageStop(MessageStopEvent::new("e.4", "msg.1")),
            "agent.stream.message.stop",
        ),
        (
            AgentStreamEvent::ToolCallStart(ToolCallStartEvent::new("e.5", "tc.1", "search")),
            "agent.stream.tool.call.start",
        ),
        (
            AgentStreamEvent::ToolCallStop(ToolCallStopEvent::new("e.6", "tc.1", "search", "{}")),
            "agent.stream.tool.call.stop",
        ),
        (
            AgentStreamEvent::ToolResult(ToolResultEvent::new(
                "e.7",
                "tc.1",
                "search",
                "ok",
                ToolCallStatus::Succeeded,
            )),
            "agent.stream.tool.result",
        ),
        (
            AgentStreamEvent::Usage(UsageEvent::new("e.8", 1, 2)),
            "agent.stream.usage",
        ),
        (
            AgentStreamEvent::Cost(CostEvent::new("e.9", 0, "CNY")),
            "agent.stream.cost",
        ),
        (
            AgentStreamEvent::Error(ErrorEvent::new("e.10", "boom")),
            "agent.stream.error",
        ),
        (
            AgentStreamEvent::RateLimit(RateLimitEvent::new("e.11", RateLimitStatus::Rejected)),
            "agent.stream.rate_limit",
        ),
        (
            AgentStreamEvent::Result(ResultEvent::new("e.12")),
            "agent.stream.result",
        ),
        (
            AgentStreamEvent::Cancelled(CancelledEvent::new("e.13")),
            "agent.stream.cancelled",
        ),
        (
            AgentStreamEvent::Ended(EndedEvent::new("e.14")),
            "agent.stream.ended",
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event.event_type(), expected);
    }
}

#[test]
fn stream_event_session_and_stream_identities_attach() {
    let event = AgentStreamEvent::MessageDelta(MessageDeltaEvent::text("e.1", "msg.1", "hi"))
        .with_session_id("session.contract")
        .with_stream_id("stream.contract");

    assert_eq!(event.session_id(), Some("session.contract"));
    assert_eq!(event.stream_id(), Some("stream.contract"));
    assert_eq!(event.event_id(), "e.1");

    // Optional attachment leaves absent identities absent.
    let none: Option<String> = None;
    let event = AgentStreamEvent::Ended(EndedEvent::new("e.2")).with_session_id_optional(&none);
    assert_eq!(event.session_id(), None);
}

#[test]
fn stream_event_bridges_into_kernel_event_envelope() {
    let event = AgentStreamEvent::MessageDelta(
        MessageDeltaEvent::text("e.1", "msg.1", "hello").with_session_id("session.1"),
    );

    let kernel_event = event.to_kernel_event();
    assert_eq!(kernel_event.event_type, "agent.stream.message.delta");
    assert_eq!(kernel_event.session_id.as_deref(), Some("session.1"));
    assert_eq!(kernel_event.source, KernelEventSource::Model);
    assert!(kernel_event.payload.contains("hello"));
}

#[test]
fn model_chunks_map_into_unified_protocol() {
    let text_chunk = ModelStreamChunk::output("model.1", 1, "streamed")
        .with_redaction(KernelEventRedaction::Public);
    let reasoning_chunk = ModelStreamChunk::reasoning("model.1", 2, "thinking");
    let args_chunk = ModelStreamChunk::tool_arguments("model.1", 3, "tc.1", r#"{"q":""#);

    match AgentStreamEvent::from(&text_chunk) {
        AgentStreamEvent::MessageDelta(delta) => {
            assert_eq!(delta.kind, MessageDeltaKind::Text);
            assert_eq!(delta.delta, "streamed");
        }
        other => panic!(
            "text chunk must map to MessageDelta, got {:?}",
            other.event_type()
        ),
    }

    match AgentStreamEvent::from(&reasoning_chunk) {
        AgentStreamEvent::MessageDelta(delta) => {
            assert_eq!(delta.kind, MessageDeltaKind::Reasoning);
        }
        other => panic!(
            "reasoning chunk must map to MessageDelta, got {:?}",
            other.event_type()
        ),
    }

    match AgentStreamEvent::from(&args_chunk) {
        AgentStreamEvent::ToolCallDelta(delta) => {
            assert_eq!(delta.tool_call_id, "tc.1");
            assert_eq!(delta.delta, r#"{"q":""#);
        }
        other => panic!(
            "tool args chunk must map to ToolCallDelta, got {:?}",
            other.event_type()
        ),
    }
}

#[test]
fn model_chunk_kind_vocabulary_is_stable() {
    assert_eq!(ModelChunkKind::Text.as_str(), "text");
    assert_eq!(ModelChunkKind::Reasoning.as_str(), "reasoning");
    assert_eq!(
        ModelChunkKind::ToolCallArguments.as_str(),
        "tool_call_arguments"
    );
    assert_eq!(ModelChunkKind::Usage.as_str(), "usage");
    assert_eq!(ModelChunkKind::Status.as_str(), "status");
}

#[test]
fn in_memory_sink_preserves_order() {
    let mut sink = InMemoryAgentStreamSink::new();
    for i in 0..5 {
        sink.push_event(AgentStreamEvent::Status(StatusEvent::info(
            format!("e.{i}"),
            format!("step {i}"),
        )))
        .unwrap();
    }
    assert_eq!(sink.count(), 5);
    assert!(!sink.is_empty());
    let events = sink.into_events();
    assert_eq!(events[0].event_id(), "e.0");
    assert_eq!(events[4].event_id(), "e.4");
}

#[test]
fn chat_stream_events_emit_sdk_convention_sequence() {
    let runtime = runtime_with_streaming_model();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentChatService::new()
        .stream_events(
            &runtime,
            AgentChatRequest::new("chat.stream.1", vec!["hello".to_string()])
                .for_session("session.stream"),
            &mut sink,
        )
        .expect("chat stream events succeed");

    let events = sink.into_events();
    let types: Vec<&str> = events.iter().map(|e| e.event_type()).collect();

    // SessionInit -> MessageStart -> MessageDelta* -> MessageStop -> Ended.
    assert_eq!(types[0], "agent.stream.session.init");
    assert_eq!(types[1], "agent.stream.message.start");
    assert_eq!(types[2], "agent.stream.message.delta");
    assert_eq!(types[3], "agent.stream.message.delta");
    assert_eq!(types[4], "agent.stream.message.delta");
    assert_eq!(types[5], "agent.stream.message.stop");
    assert_eq!(types[6], "agent.stream.ended");

    // Session identity propagates to every event.
    for event in &events {
        assert_eq!(event.session_id(), Some("session.stream"));
    }

    // The stop event aggregates the visible text content.
    match &events[5] {
        AgentStreamEvent::MessageStop(stop) => assert_eq!(stop.content, "streamed response"),
        other => panic!("expected MessageStop, got {:?}", other.event_type()),
    }
}

#[test]
fn chat_stream_events_skip_session_init_without_session() {
    let runtime = runtime_with_streaming_model();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentChatService::new()
        .stream_events(
            &runtime,
            AgentChatRequest::new("chat.stream.2", vec!["hello".to_string()]),
            &mut sink,
        )
        .unwrap();

    let events = sink.into_events();
    assert_eq!(events[0].event_type(), "agent.stream.message.start");
    assert_eq!(events.last().unwrap().event_type(), "agent.stream.ended");
}

#[test]
fn execution_stream_emits_tool_lifecycle_and_terminal_result() {
    let runtime = runtime_with_tool_calling();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            AgentExecutionRequest::new("exec.stream.1", vec!["search sdkwork".to_string()])
                .for_session("session.exec"),
            &mut sink,
        )
        .expect("execution stream succeeds");

    let events = sink.into_events();
    let types: Vec<&str> = events.iter().map(|e| e.event_type()).collect();

    assert_eq!(types[0], "agent.stream.session.init");
    assert_eq!(types[1], "agent.stream.message.start");
    assert_eq!(types[2], "agent.stream.message.delta");
    assert_eq!(types[3], "agent.stream.tool.call.start");
    assert_eq!(types[4], "agent.stream.tool.call.stop");
    assert_eq!(types[5], "agent.stream.message.stop");
    assert_eq!(types[6], "agent.stream.tool.result");
    assert_eq!(types[7], "agent.stream.usage");
    assert_eq!(types[8], "agent.stream.result");
    assert_eq!(types[9], "agent.stream.ended");

    // Strongly typed tool result carries the outcome.
    match &events[6] {
        AgentStreamEvent::ToolResult(result) => {
            assert_eq!(result.tool_call_id, "tool-call.from-model.1");
            assert_eq!(result.tool_name, "tool.stream.search");
            assert!(!result.is_error);
            assert_eq!(result.status, ToolCallStatus::Succeeded);
            assert_eq!(result.content, "search results");
        }
        other => panic!("expected ToolResult, got {:?}", other.event_type()),
    }

    // Terminal result aggregates usage and outcome.
    match &events[8] {
        AgentStreamEvent::Result(result) => {
            assert!(!result.is_error);
            assert_eq!(result.num_turns, 1);
            let usage = result.usage.as_ref().expect("result carries usage");
            assert_eq!(usage.input_tokens, 20);
            assert_eq!(usage.output_tokens, 3);
        }
        other => panic!("expected Result, got {:?}", other.event_type()),
    }
}

#[test]
fn kernel_event_stream_sink_forwards_stream_to_envelope() {
    let captured: Arc<Mutex<Vec<sdkwork_agent_kernel::KernelEvent>>> =
        Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    let mut sink = KernelEventStreamSink::new(move |event| {
        captured_clone.lock().unwrap().push(event);
        Ok(())
    });

    sink.push_event(AgentStreamEvent::MessageStart(MessageStartEvent::new(
        "e.1",
        "msg.1",
        AgentMessageRole::Agent,
    )))
    .unwrap();

    let events = captured.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "agent.stream.message.start");
}

#[test]
fn stream_event_helpers_cover_tool_and_usage_fields() {
    let usage = UsageEvent::new("u.1", 10, 20)
        .with_cached_input_tokens(5)
        .with_reasoning_tokens(3);
    assert_eq!(usage.total_tokens, 38);
    assert_eq!(usage.cached_input_tokens, 5);

    let cost = CostEvent::new("c.1", 12, "CNY").with_total_cost_usd(0.0017);
    assert_eq!(cost.cost_cents, 12);
    assert!(cost.total_cost_usd.is_some());

    let rate_limit = RateLimitEvent::new("r.1", RateLimitStatus::AllowedWarning)
        .with_utilization(0.85)
        .with_limit_type("five_hour");
    assert_eq!(rate_limit.status, RateLimitStatus::AllowedWarning);
    assert_eq!(rate_limit.utilization, Some(0.85));

    let delta = MessageDeltaEvent::reasoning("d.1", "msg.1", "think");
    assert_eq!(delta.kind, MessageDeltaKind::Reasoning);

    let tool_result =
        ToolResultEvent::new("t.1", "tc.1", "search", "ok", ToolCallStatus::Succeeded)
            .with_duration_ms(12)
            .with_metadata("provider", "p.1");
    assert_eq!(tool_result.duration_ms, Some(12));
    assert_eq!(
        tool_result.metadata,
        vec![("provider".to_string(), "p.1".to_string())]
    );

    let _trace = TraceContext::new("trace.1", "span.1");
    assert!(cost.total_cost_usd.is_some());
}
