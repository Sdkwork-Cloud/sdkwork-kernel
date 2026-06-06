use sdkwork_agent_kernel::{
    AgentSkillDescriptor, AgentSkillInvocationMode, AgentSkillRequest, AgentSkillResult,
    AgentSkillStatus, KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity,
    KernelResult, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat, ModelStatus,
    ModelStreamChunk, ModelUsage, PolicyCategory, ProviderHealth, ProviderManifest,
    SideEffectLevel, ToolCall, ToolCallStatus, ToolDescriptor, ToolProvider, ToolResult,
    ToolSchema, ToolStreamChunk, TraceContext,
};

#[test]
fn model_request_carries_runtime_context_policy_trace_timeout_and_response_format() {
    let request = ModelRequest::new("model-request.1", vec!["hello".to_string()])
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .for_step("step.1")
        .with_response_format(ModelResponseFormat::JsonSchema(
            "sdkwork.answer.schema.v1".to_string(),
        ))
        .with_policy_context("policy-request.1")
        .with_trace_context(TraceContext::new("trace.1", "span.1"))
        .with_timeout_ms(30_000)
        .with_metadata("openai.reasoning_effort", "medium");

    assert_eq!(request.session_id.as_deref(), Some("session.1"));
    assert_eq!(request.task_id.as_deref(), Some("task.1"));
    assert_eq!(request.run_id.as_deref(), Some("run.1"));
    assert_eq!(request.step_id.as_deref(), Some("step.1"));
    assert_eq!(
        request.response_format,
        Some(ModelResponseFormat::JsonSchema(
            "sdkwork.answer.schema.v1".to_string()
        ))
    );
    assert_eq!(
        request.policy_request_id.as_deref(),
        Some("policy-request.1")
    );
    assert_eq!(request.trace_context.as_ref().unwrap().trace_id, "trace.1");
    assert_eq!(request.timeout_ms, Some(30_000));
    assert_eq!(
        request.metadata_value("openai.reasoning_effort"),
        Some("medium")
    );
}

#[test]
fn model_response_preserves_status_usage_tool_calls_redaction_and_diagnostics() {
    let response = ModelResponse::text("model-request.1", "provider.model.fake", "hello")
        .with_status(ModelStatus::Succeeded)
        .with_usage(ModelUsage::new(10, 20))
        .with_tool_call(ToolCall::new(
            "tool-call.1",
            "tool.echo",
            "{\"value\":\"hello\"}",
        ))
        .with_finish_reason("stop")
        .with_redaction(KernelEventRedaction::Internal)
        .with_diagnostic("latency_ms=12");

    assert_eq!(response.status, ModelStatus::Succeeded);
    assert_eq!(response.usage.as_ref().unwrap().total_tokens(), 30);
    assert_eq!(response.tool_calls[0].tool_id, "tool.echo");
    assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    assert_eq!(
        response.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(response.diagnostics, ["latency_ms=12"]);
}

#[test]
fn model_stream_chunks_map_to_ordered_kernel_events() {
    let chunk = ModelStreamChunk::output("model-request.1", 2, "partial")
        .with_trace_context(TraceContext::new("trace.1", "span.2"))
        .with_redaction(KernelEventRedaction::Internal);

    let event = chunk.to_event("event.model.1");

    assert_eq!(event.event_type, "agent.model.output.streamed");
    assert_eq!(event.severity, KernelEventSeverity::Info);
    assert_eq!(event.trace_context.as_ref().unwrap().span_id, "span.2");
    assert_eq!(
        event.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert!(event.payload.contains("sequence=2"));
    assert!(event.payload.contains("chunk=partial"));
}

#[test]
fn model_provider_default_stream_and_cancel_report_capability_missing() {
    let provider = BasicModelProvider;

    let stream_error = provider
        .stream(ModelRequest::new("model-request.1", vec![]))
        .expect_err("stream unsupported");
    let cancel_error = provider
        .cancel("model-request.1")
        .expect_err("cancel unsupported");

    assert!(stream_error.to_string().contains("model.streaming"));
    assert!(cancel_error.to_string().contains("model.cancellation"));
}

#[test]
fn advanced_model_provider_can_stream_and_cancel() {
    let provider = AdvancedModelProvider;
    let chunks = provider
        .stream(ModelRequest::new(
            "model-request.1",
            vec!["hello".to_string()],
        ))
        .expect("stream supported");
    let cancelled = provider
        .cancel("model-request.1")
        .expect("cancel supported");

    assert_eq!(chunks[0].sequence, 1);
    assert_eq!(chunks[0].content, "hello");
    assert_eq!(cancelled.status, ModelStatus::Cancelled);
}

#[test]
fn tool_descriptor_declares_schema_timeout_cancellation_and_audit_requirements() {
    let descriptor = ToolDescriptor::new(
        "tool.shell.run",
        "provider.tool.fake",
        "Run Shell Command",
        SideEffectLevel::SideEffectful,
    )
    .with_name("shell_run")
    .with_version("0.1.0")
    .with_description("Run a shell command")
    .with_input_schema(ToolSchema::json_schema("sdkwork.tool.shell.input.v1"))
    .with_output_schema(ToolSchema::json_schema("sdkwork.tool.shell.output.v1"))
    .with_timeout_ms(60_000)
    .supports_cancellation(true)
    .require_audit()
    .with_policy_categories(vec!["host.process.execute".to_string()]);

    assert_eq!(descriptor.name.as_deref(), Some("shell_run"));
    assert_eq!(descriptor.version.as_deref(), Some("0.1.0"));
    assert_eq!(
        descriptor.input_schema.as_ref().unwrap().schema_id,
        "sdkwork.tool.shell.input.v1"
    );
    assert_eq!(descriptor.timeout_ms, Some(60_000));
    assert!(descriptor.cancellation_supported);
    assert!(descriptor.audit_required);
    assert!(descriptor.requires_policy());
}

#[test]
fn tool_call_carries_context_trace_policy_timeout_and_created_at() {
    let call = ToolCall::new("tool-call.1", "tool.echo", "{\"value\":\"hello\"}")
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .for_step("step.1")
        .with_provider("provider.tool.fake")
        .with_policy_decision("policy-decision.1")
        .with_trace_context(TraceContext::new("trace.1", "span.1"))
        .with_timeout_ms(10_000)
        .created_at("2026-05-27T12:00:00Z");

    assert_eq!(call.session_id.as_deref(), Some("session.1"));
    assert_eq!(call.task_id.as_deref(), Some("task.1"));
    assert_eq!(call.run_id.as_deref(), Some("run.1"));
    assert_eq!(call.step_id.as_deref(), Some("step.1"));
    assert_eq!(call.provider_id.as_deref(), Some("provider.tool.fake"));
    assert_eq!(
        call.policy_decision_id.as_deref(),
        Some("policy-decision.1")
    );
    assert_eq!(call.trace_context.as_ref().unwrap().trace_id, "trace.1");
    assert_eq!(call.timeout_ms, Some(10_000));
    assert_eq!(call.created_at.as_deref(), Some("2026-05-27T12:00:00Z"));
}

#[test]
fn tool_descriptor_builds_policy_request_for_protected_call() {
    let descriptor = ToolDescriptor::new(
        "tool.shell.run",
        "provider.tool.fake",
        "Run Shell Command",
        SideEffectLevel::SideEffectful,
    )
    .with_policy_categories(vec!["host.process.execute".to_string()]);
    let call = ToolCall::new("tool-call.1", "tool.shell.run", "cargo test")
        .for_session("session.1")
        .for_task("task.1");

    let request = descriptor.policy_request("policy-request.1", &call);

    assert_eq!(
        request.typed_category,
        Some(PolicyCategory::ProductSpecific(
            "host.process.execute".to_string()
        ))
    );
    assert_eq!(request.resource, "tool.shell.run");
    assert_eq!(request.session_id.as_deref(), Some("session.1"));
    assert_eq!(request.task_id.as_deref(), Some("task.1"));
    assert_eq!(
        request.side_effect_level,
        Some(SideEffectLevel::SideEffectful)
    );
}

#[test]
fn tool_result_preserves_status_timing_redaction_audit_and_error() {
    let result = ToolResult::failed("tool-call.1", "permission denied")
        .with_status(ToolCallStatus::Denied)
        .started_at("2026-05-27T12:00:00Z")
        .completed_at("2026-05-27T12:00:01Z")
        .with_duration_ms(1000)
        .with_redaction(KernelEventRedaction::Internal)
        .with_audit_ref("audit.1");

    assert_eq!(result.status, "denied");
    assert_eq!(result.normalized_status, ToolCallStatus::Denied);
    assert_eq!(result.error.as_deref(), Some("permission denied"));
    assert_eq!(result.duration_ms, Some(1000));
    assert_eq!(result.audit_refs, ["audit.1"]);
    assert_eq!(
        result.redaction_classification,
        KernelEventRedaction::Internal
    );
}

#[test]
fn tool_stream_chunk_maps_to_kernel_event() {
    let chunk = ToolStreamChunk::output("tool-call.1", 1, "line 1")
        .with_trace_context(TraceContext::new("trace.1", "span.2"))
        .with_redaction(KernelEventRedaction::Internal);

    let event = chunk.to_event("event.tool.1");

    assert_eq!(event.event_type, "agent.tool.call.output_streamed");
    assert_eq!(event.severity, KernelEventSeverity::Info);
    assert!(event.payload.contains("tool_call_id=tool-call.1"));
    assert!(event.payload.contains("sequence=1"));
}

#[test]
fn tool_provider_default_authorize_stream_and_cancel_are_standardized() {
    let provider = BasicToolProvider;
    let descriptor = provider.describe_tool("tool.echo").expect("tool exists");
    let call = ToolCall::new("tool-call.1", "tool.echo", "hello");

    let policy_request = provider
        .authorize_tool_call(&descriptor, &call)
        .expect("authorization request is generated");
    let stream_error = provider
        .stream_tool_call(call.clone())
        .expect_err("stream unsupported");
    let cancel_error = provider
        .cancel_tool_call("tool-call.1")
        .expect_err("cancel unsupported");

    assert_eq!(policy_request.resource, "tool.echo");
    assert!(stream_error.to_string().contains("tool.streaming"));
    assert!(cancel_error.to_string().contains("tool.cancellation"));
}

#[test]
fn skill_request_and_result_preserve_runtime_governance_context() {
    let request = AgentSkillRequest::new("skill-request.1", "skill.code-review")
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .for_step("step.1")
        .with_argument("scope", "diff")
        .with_policy_context("policy-decision.1")
        .with_trace_context(TraceContext::new("trace.1", "span.skill"))
        .with_timeout_ms(45_000)
        .with_metadata("skill.marketplace.package", "pkg.code-review");

    assert_eq!(request.session_id.as_deref(), Some("session.1"));
    assert_eq!(request.task_id.as_deref(), Some("task.1"));
    assert_eq!(request.run_id.as_deref(), Some("run.1"));
    assert_eq!(request.step_id.as_deref(), Some("step.1"));
    assert_eq!(
        request.policy_decision_id.as_deref(),
        Some("policy-decision.1")
    );
    assert_eq!(request.trace_context.as_ref().unwrap().span_id, "span.skill");
    assert_eq!(request.timeout_ms, Some(45_000));
    assert_eq!(
        request.metadata_value("skill.marketplace.package"),
        Some("pkg.code-review")
    );

    let result = AgentSkillResult::succeeded(
        request.skill_request_id,
        request.skill_id,
        "reviewed diff",
    )
    .with_status(AgentSkillStatus::Succeeded)
    .started_at("2026-05-27T12:00:00Z")
    .completed_at("2026-05-27T12:00:02Z")
    .with_duration_ms(2_000)
    .with_trace_context(TraceContext::new("trace.1", "span.skill"))
    .with_redaction(KernelEventRedaction::Internal)
    .with_audit_ref("audit.skill.1")
    .with_diagnostic("provider=provider.skill.local");

    assert_eq!(result.status, AgentSkillStatus::Succeeded);
    assert_eq!(result.output, "reviewed diff");
    assert_eq!(result.duration_ms, Some(2_000));
    assert_eq!(result.trace_context.as_ref().unwrap().trace_id, "trace.1");
    assert_eq!(
        result.redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(result.audit_refs, ["audit.skill.1"]);
    assert_eq!(result.diagnostics, ["provider=provider.skill.local"]);
}

#[test]
fn skill_descriptor_declares_provider_schema_timeout_cancellation_audit_and_metadata() {
    let descriptor = AgentSkillDescriptor::new(
        "skill.code-review",
        "provider.skill.local",
        "Code Review",
        "Review code changes and return actionable risks.",
        AgentSkillInvocationMode::Workflow,
    )
    .with_version("0.2.0")
    .with_model_hint("claude-sonnet")
    .with_allowed_tool("tool.repo.read")
    .with_input_schema(ToolSchema::json_schema(
        "sdkwork.agent.skill.code_review.input.v1",
    ))
    .with_output_schema(ToolSchema::json_schema(
        "sdkwork.agent.skill.code_review.output.v1",
    ))
    .with_timeout_ms(120_000)
    .supports_cancellation(true)
    .require_audit()
    .with_side_effect_level(SideEffectLevel::ReadOnly)
    .with_policy_category("repo.read")
    .with_metadata("marketplace.category", "engineering");

    assert_eq!(descriptor.provider_id, "provider.skill.local");
    assert_eq!(descriptor.version.as_deref(), Some("0.2.0"));
    assert_eq!(descriptor.model_hint.as_deref(), Some("claude-sonnet"));
    assert_eq!(descriptor.allowed_tools, ["tool.repo.read"]);
    assert_eq!(
        descriptor.input_schema.as_ref().unwrap().schema_id,
        "sdkwork.agent.skill.code_review.input.v1"
    );
    assert_eq!(
        descriptor.output_schema.as_ref().unwrap().schema_id,
        "sdkwork.agent.skill.code_review.output.v1"
    );
    assert_eq!(descriptor.timeout_ms, Some(120_000));
    assert!(descriptor.cancellation_supported);
    assert!(descriptor.audit_required);
    assert!(descriptor.requires_policy());
    assert_eq!(
        descriptor.metadata_value("marketplace.category"),
        Some("engineering")
    );
}

struct BasicModelProvider;

impl ModelProvider for BasicModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new("provider.model.basic", "model", "basic", "0.1.0", vec![])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.basic",
            "ok",
        ))
    }
}

struct AdvancedModelProvider;

impl ModelProvider for AdvancedModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.advanced",
            "model",
            "advanced",
            "0.1.0",
            vec!["streaming".to_string(), "cancellation".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.advanced",
            "ok",
        ))
    }

    fn stream(&self, request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        Ok(vec![ModelStreamChunk::output(
            request.model_request_id,
            1,
            "hello",
        )])
    }

    fn cancel(&self, model_request_id: &str) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::cancelled(
            model_request_id,
            "provider.model.advanced",
        ))
    }
}

struct BasicToolProvider;

impl ToolProvider for BasicToolProvider {
    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.echo",
            "provider.tool.basic",
            "Echo",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, call.arguments))
    }
}

#[test]
fn keep_kernel_error_import_reachable_for_model_tool_contracts() {
    let error = KernelError::CapabilityMissing {
        capability_id: "tool.streaming".to_string(),
    };

    assert!(error.to_string().contains("tool.streaming"));
}

#[test]
fn keep_kernel_event_import_reachable_for_stream_contracts() {
    let event: KernelEvent = ToolStreamChunk::output("tool-call.1", 1, "line").to_event("event.1");

    assert_eq!(event.event_id, "event.1");
}
