use sdkwork_agent_kernel::{
    AgentSkillDescriptor, AgentSkillInvocationMode, AgentSkillRequest, AgentSkillResult,
    AgentSkillStatus, ApprovedToolExecution, ContextFrame, KernelError, KernelErrorKind,
    KernelEvent, KernelEventRedaction, KernelEventSeverity, KernelResult, McpProvider,
    McpServerDescriptor, McpToolExecutionRequest, McpToolExecutionService, McpTransportKind,
    ModelCancellationRequest, ModelExecutionRequest, ModelExecutionService, ModelProvider,
    ModelRequest, ModelResponse, ModelResponseFormat, ModelStatus, ModelStreamChunk,
    ModelStructuredOutputValidation, ModelUsage, PolicyCategory, PolicyDecision,
    PolicyDecisionValue, PolicyProvider, PolicyRequest, ProviderHealth, ProviderManifest,
    RedactionClassification, SideEffectLevel, ToolCall, ToolCallStatus, ToolCancellationRequest,
    ToolDescriptor, ToolExecutionRequest, ToolExecutionService, ToolProvider, ToolResult,
    ToolSchema, ToolStreamChunk, TraceContext, TrustLevel,
};
use std::sync::{Arc, Mutex};

#[test]
fn model_request_carries_runtime_context_policy_trace_timeout_and_response_format() {
    let request = ModelRequest::new("model-request.1", vec!["hello".to_string()])
        .for_session("session.1")
        .for_task("task.1")
        .for_run("run.1")
        .for_step("step.1")
        .with_response_format(ModelResponseFormat::json_schema(
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
        Some(ModelResponseFormat::json_schema(
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
    let response = ModelResponse::text("model-request.1", "provider.fake", "hello")
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
fn model_execution_service_evaluates_invoke_and_sensitive_context_policy_before_provider_call() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.model.execution",
        model_execution_agent_manifest("agent.model.execution", "Model Execution"),
    )
    .register_model_provider(
        "provider.execution",
        "0.1.0",
        CountingModelProvider::new(captured_model_requests.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        RecordingStaticPolicyProvider::allow(captured_policy_requests.clone()),
    )
    .bootstrap()
    .expect("model execution runtime bootstraps")
    .runtime;

    let response = ModelExecutionService::new()
        .invoke(
            &runtime,
            ModelExecutionRequest::new(
                "model.execution.1",
                ModelRequest::new(
                    "model-request.1",
                    vec!["Use tenant knowledge safely.".to_string()],
                )
                .with_model_id("model.secure")
                .for_session("session.model")
                .for_task("task.model")
                .for_run("run.model")
                .for_step("step.model")
                .with_context_frame_payload(ContextFrame::new(
                    "context.tenant.1",
                    "session.model",
                    "knowledge",
                    "tenant-sensitive answer source",
                    TrustLevel::RetrievedExternal,
                    RedactionClassification::TenantSensitive,
                )),
            )
            .with_provider_id("provider.execution"),
        )
        .expect("allowed model execution invokes provider");

    assert_eq!(response.provider_id, "provider.execution");
    assert_eq!(
        response.invoke_policy_decision.decision,
        PolicyDecisionValue::Allow
    );
    assert_eq!(
        response
            .sensitive_context_policy_decision
            .as_ref()
            .expect("sensitive context policy evaluated")
            .decision,
        PolicyDecisionValue::Allow
    );
    assert_eq!(response.model_response.status, ModelStatus::Succeeded);

    let policy_requests = captured_policy_requests.lock().unwrap();
    assert_eq!(policy_requests.len(), 2);
    assert!(policy_requests
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::ModelInvoke)));
    let sensitive_request = policy_requests
        .iter()
        .find(|request| request.typed_category == Some(PolicyCategory::ModelSendSensitiveContext))
        .expect("sensitive context send is policy-gated");
    assert_eq!(sensitive_request.resource, "model.secure");
    assert_eq!(
        sensitive_request.context_value("provider_id"),
        Some("provider.execution")
    );

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(model_requests[0].model_id.as_deref(), Some("model.secure"));
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.model.provider_id"),
        Some("provider.execution")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.model.policy_decision_id"),
        Some(response.invoke_policy_decision.decision_id.as_str())
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.model.sensitive_context_policy_decision_id"),
        response
            .sensitive_context_policy_decision
            .as_ref()
            .map(|decision| decision.decision_id.as_str())
    );
}

#[test]
fn model_execution_service_fails_closed_when_invoke_policy_denies_before_provider_call() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.model.denied",
        model_execution_agent_manifest("agent.model.denied", "Model Denied"),
    )
    .register_model_provider(
        "provider.execution",
        "0.1.0",
        CountingModelProvider::new(captured_model_requests.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        RecordingStaticPolicyProvider::deny_category(
            captured_policy_requests.clone(),
            PolicyCategory::ModelInvoke,
            "model.denied",
        ),
    )
    .bootstrap()
    .expect("model denied runtime bootstraps")
    .runtime;

    let error = ModelExecutionService::new()
        .invoke(
            &runtime,
            ModelExecutionRequest::new(
                "model.execution.denied",
                ModelRequest::new("model-request.denied", vec!["hello".to_string()])
                    .with_model_id("model.secure"),
            ),
        )
        .expect_err("model invoke policy blocks provider invocation");

    assert_eq!(error.kind(), KernelErrorKind::PolicyDenied);
    assert!(captured_model_requests.lock().unwrap().is_empty());
    assert_eq!(captured_policy_requests.lock().unwrap().len(), 1);
}

#[test]
fn model_execution_service_streams_through_policy_gate() {
    let captured_stream_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.model.stream",
        model_execution_agent_manifest("agent.model.stream", "Model Stream"),
    )
    .register_model_provider(
        "provider.complete",
        "0.1.0",
        CompleteModelProvider::new(
            captured_stream_requests.clone(),
            Arc::new(Mutex::new(Vec::new())),
        ),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        RecordingStaticPolicyProvider::allow(captured_policy_requests.clone()),
    )
    .bootstrap()
    .expect("model stream runtime bootstraps")
    .runtime;

    let response = ModelExecutionService::new()
        .stream(
            &runtime,
            ModelExecutionRequest::new(
                "model.stream.1",
                ModelRequest::new("model-request.stream", vec!["stream".to_string()])
                    .with_model_id("model.complete"),
            )
            .with_provider_id("provider.complete"),
        )
        .expect("allowed model stream executes");

    assert_eq!(response.provider_id, "provider.complete");
    assert_eq!(response.chunks.len(), 2);
    assert_eq!(response.chunks[0].sequence, 1);
    assert_eq!(
        response.invoke_policy_decision.decision,
        PolicyDecisionValue::Allow
    );
    assert_eq!(captured_policy_requests.lock().unwrap().len(), 1);
    assert_eq!(
        captured_policy_requests.lock().unwrap()[0].typed_category,
        Some(PolicyCategory::ModelInvoke)
    );
    assert_eq!(captured_stream_requests.lock().unwrap().len(), 1);
    assert_eq!(
        captured_stream_requests.lock().unwrap()[0]
            .metadata_value("sdkwork.model.policy_decision_id"),
        Some(response.invoke_policy_decision.decision_id.as_str())
    );
}

#[test]
fn model_execution_service_cancels_through_selected_provider() {
    let captured_cancellations = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.model.cancel",
        model_execution_agent_manifest("agent.model.cancel", "Model Cancel"),
    )
    .register_model_provider(
        "provider.complete",
        "0.1.0",
        CompleteModelProvider::new(
            Arc::new(Mutex::new(Vec::new())),
            captured_cancellations.clone(),
        ),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::allow(),
    )
    .bootstrap()
    .expect("model cancel runtime bootstraps")
    .runtime;

    let response = ModelExecutionService::new()
        .cancel(
            &runtime,
            ModelCancellationRequest::new("model.cancel.1", "model-request.cancel")
                .with_provider_id("provider.complete"),
        )
        .expect("model cancellation delegates to provider");

    assert_eq!(response.provider_id, "provider.complete");
    assert_eq!(response.model_response.status, ModelStatus::Cancelled);
    assert_eq!(
        captured_cancellations.lock().unwrap().as_slice(),
        &["model-request.cancel".to_string()]
    );
}

#[test]
fn model_execution_service_validates_structured_output_when_json_schema_is_requested() {
    let captured_invocations = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.model.structured",
        model_execution_agent_manifest("agent.model.structured", "Model Structured"),
    )
    .register_model_provider(
        "provider.complete",
        "0.1.0",
        CompleteModelProvider::new(
            captured_invocations.clone(),
            Arc::new(Mutex::new(Vec::new())),
        ),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        RecordingStaticPolicyProvider::allow(captured_policy_requests.clone()),
    )
    .bootstrap()
    .expect("structured model runtime bootstraps")
    .runtime;

    let response = ModelExecutionService::new()
        .invoke(
            &runtime,
            ModelExecutionRequest::new(
                "model.structured.1",
                ModelRequest::new("model-request.structured", vec!["json".to_string()])
                    .with_model_id("model.complete")
                    .with_response_format(ModelResponseFormat::json_schema(
                        "sdkwork.answer.schema.v1".to_string(),
                    )),
            )
            .with_provider_id("provider.complete"),
        )
        .expect("structured model response validates");

    let validation = response
        .structured_output_validation
        .expect("structured output validation is attached");
    assert!(validation.valid);
    assert_eq!(validation.model_request_id, "model-request.structured");
    assert_eq!(validation.schema_id, "sdkwork.answer.schema.v1");
    assert_eq!(captured_invocations.lock().unwrap().len(), 1);
    assert_eq!(captured_policy_requests.lock().unwrap().len(), 1);
}

#[test]
fn model_execution_service_returns_validation_error_for_invalid_structured_output() {
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.model.invalid-structured",
        model_execution_agent_manifest(
            "agent.model.invalid-structured",
            "Model Invalid Structured",
        ),
    )
    .register_model_provider(
        "provider.invalid-structured",
        "0.1.0",
        InvalidStructuredModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::allow(),
    )
    .bootstrap()
    .expect("invalid structured model runtime bootstraps")
    .runtime;

    let error = ModelExecutionService::new()
        .invoke(
            &runtime,
            ModelExecutionRequest::new(
                "model.invalid-structured.1",
                ModelRequest::new("model-request.invalid", vec!["json".to_string()])
                    .with_model_id("model.invalid-structured")
                    .with_response_format(ModelResponseFormat::json_schema(
                        "sdkwork.answer.schema.v1".to_string(),
                    )),
            )
            .with_provider_id("provider.invalid-structured"),
        )
        .expect_err("invalid structured output maps to validation error");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    assert!(error
        .message()
        .contains("structured model output did not match sdkwork.answer.schema.v1"));
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
fn tool_execution_service_evaluates_policy_before_invoking_selected_provider() {
    let captured_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.tool.execution",
        tool_execution_agent_manifest("agent.tool.execution", "Tool Execution"),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CountingToolProvider::new(captured_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::allow(),
    )
    .bootstrap()
    .expect("tool execution runtime bootstraps")
    .runtime;

    let response = ToolExecutionService::new()
        .invoke(
            &runtime,
            ToolExecutionRequest::new(
                "tool.execution.1",
                ToolCall::new("tool-call.1", "tool.protected", "{}")
                    .with_provider("provider.tool.execution")
                    .for_session("session.1")
                    .for_task("task.1"),
            ),
        )
        .expect("allowed tool call executes");

    assert_eq!(
        response.policy_decision.decision,
        PolicyDecisionValue::Allow
    );
    assert_eq!(response.result.normalized_status, ToolCallStatus::Succeeded);
    assert_eq!(captured_calls.lock().unwrap().len(), 1);
    assert_eq!(
        captured_calls.lock().unwrap()[0]
            .policy_decision_id
            .as_deref(),
        Some(response.policy_decision.decision_id.as_str())
    );
}

#[test]
fn tool_execution_service_streams_after_same_policy_gate_as_invoke() {
    let captured_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.tool.stream.execution",
        tool_execution_agent_manifest("agent.tool.stream.execution", "Tool Stream Execution"),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CountingToolProvider::new(captured_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::allow(),
    )
    .bootstrap()
    .expect("tool stream runtime bootstraps")
    .runtime;

    let response = ToolExecutionService::new()
        .stream(
            &runtime,
            ToolExecutionRequest::new(
                "tool.execution.stream",
                ToolCall::new("tool-call.stream", "tool.protected", "{}")
                    .with_provider("provider.tool.execution")
                    .for_session("session.stream")
                    .for_task("task.stream"),
            ),
        )
        .expect("allowed tool stream executes");

    assert_eq!(response.provider_id, "provider.tool.execution");
    assert_eq!(
        response.policy_decision.decision,
        PolicyDecisionValue::Allow
    );
    assert_eq!(response.chunks.len(), 1);
    assert_eq!(response.chunks[0].tool_call_id, "tool-call.stream");
    assert_eq!(
        captured_calls.lock().unwrap()[0]
            .policy_decision_id
            .as_deref(),
        Some(response.policy_decision.decision_id.as_str())
    );
}

#[test]
fn tool_execution_service_cancel_routes_to_selected_provider() {
    let captured_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.tool.cancel.execution",
        tool_execution_agent_manifest("agent.tool.cancel.execution", "Tool Cancel Execution"),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CountingToolProvider::new(captured_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::allow(),
    )
    .bootstrap()
    .expect("tool cancel runtime bootstraps")
    .runtime;

    let response = ToolExecutionService::new()
        .cancel(
            &runtime,
            ToolCancellationRequest::new("tool.cancellation.1", "tool-call.cancel")
                .with_provider_id("provider.tool.execution"),
        )
        .expect("tool cancellation routes to provider");

    assert_eq!(response.tool_cancellation_id, "tool.cancellation.1");
    assert_eq!(response.provider_id, "provider.tool.execution");
    assert_eq!(response.result.tool_call_id, "tool-call.cancel");
    assert_eq!(response.result.normalized_status, ToolCallStatus::Cancelled);
    assert_eq!(
        captured_calls.lock().unwrap()[0].tool_call_id,
        "tool-call.cancel"
    );
}

#[test]
fn tool_execution_service_fails_closed_when_policy_denies_or_requires_approval() {
    let denied_calls = Arc::new(Mutex::new(Vec::new()));
    let denied_runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.tool.denied",
        tool_execution_agent_manifest("agent.tool.denied", "Tool Denied"),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CountingToolProvider::new(denied_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::deny("tool.denied"),
    )
    .bootstrap()
    .expect("denied tool runtime bootstraps")
    .runtime;

    let denied_error = ToolExecutionService::new()
        .invoke(
            &denied_runtime,
            ToolExecutionRequest::new(
                "tool.execution.denied",
                ToolCall::new("tool-call.denied", "tool.protected", "{}"),
            ),
        )
        .expect_err("denied policy blocks tool execution");
    assert_eq!(denied_error.kind(), KernelErrorKind::PolicyDenied);
    assert!(denied_calls.lock().unwrap().is_empty());

    let approval_calls = Arc::new(Mutex::new(Vec::new()));
    let approval_runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.tool.approval",
        tool_execution_agent_manifest("agent.tool.approval", "Tool Approval"),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CountingToolProvider::new(approval_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::needs_approval(),
    )
    .bootstrap()
    .expect("approval tool runtime bootstraps")
    .runtime;

    let approval_error = ToolExecutionService::new()
        .invoke(
            &approval_runtime,
            ToolExecutionRequest::new(
                "tool.execution.approval",
                ToolCall::new("tool-call.approval", "tool.protected", "{}"),
            ),
        )
        .expect_err("approval policy blocks tool execution until approved");
    assert_eq!(approval_error.kind(), KernelErrorKind::PermissionRequired);
    assert!(approval_calls.lock().unwrap().is_empty());
}

#[test]
fn approved_tool_execution_is_one_shot_and_revision_bound() {
    let captured_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.tool.approved",
        tool_execution_agent_manifest("agent.tool.approved", "Tool Approved"),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CountingToolProvider::new(captured_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticPolicyProvider::needs_approval(),
    )
    .bootstrap()
    .expect("approved tool runtime bootstraps")
    .runtime;

    let service = ToolExecutionService::new();
    let request = || {
        ToolExecutionRequest::new(
            "tool.execution.approved",
            ToolCall::new("tool-call.approved", "tool.protected", "{}"),
        )
    };
    let approval = ApprovedToolExecution::new(
        "policy-request.tool-call.approved",
        "provider.tool.execution",
        "0.1.0",
        "0.1.0",
    );
    let response = service
        .invoke_approved(&runtime, request(), &approval)
        .expect("matching one-shot approval invokes tool");
    assert!(response.policy_decision.is_allow());
    assert_eq!(captured_calls.lock().unwrap().len(), 1);

    let mismatched = ApprovedToolExecution::new(
        "policy-request.other",
        "provider.tool.execution",
        "0.1.0",
        "0.1.0",
    );
    let error = service
        .invoke_approved(&runtime, request(), &mismatched)
        .expect_err("mismatched permission identity fails closed");
    assert_eq!(error.kind(), KernelErrorKind::PolicyDenied);

    let stale = ApprovedToolExecution::new(
        "policy-request.tool-call.approved",
        "provider.tool.execution",
        "0.0.9",
        "0.1.0",
    );
    let error = service
        .invoke_approved(&runtime, request(), &stale)
        .expect_err("stale descriptor revision fails closed");
    assert_eq!(error.kind(), KernelErrorKind::PolicyDenied);
    assert_eq!(captured_calls.lock().unwrap().len(), 1);
}

#[test]
fn mcp_tool_execution_service_applies_tool_policy_before_invoking_mcp_provider() {
    let captured_mcp_calls = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.mcp.execution",
        mcp_execution_agent_manifest("agent.mcp.execution", "MCP Execution"),
    )
    .register_mcp_provider(
        "provider.mcp.execution",
        "0.1.0",
        CountingMcpProvider::new(captured_mcp_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        RecordingStaticPolicyProvider::allow(captured_policy_requests.clone()),
    )
    .bootstrap()
    .expect("mcp execution runtime bootstraps")
    .runtime;

    let response = McpToolExecutionService::new()
        .invoke(
            &runtime,
            McpToolExecutionRequest::new(
                "mcp.execution.1",
                "mcp.execution",
                ToolCall::new("mcp-tool-call.1", "mcp.tool.protected", "{}")
                    .with_provider("provider.mcp.execution")
                    .for_session("session.mcp")
                    .for_task("task.mcp"),
            ),
        )
        .expect("allowed mcp tool call executes");

    assert_eq!(response.provider_id, "provider.mcp.execution");
    assert_eq!(response.server_id, "mcp.execution");
    assert_eq!(
        response.policy_decision.decision,
        PolicyDecisionValue::Allow
    );
    assert_eq!(response.result.normalized_status, ToolCallStatus::Succeeded);

    let policy_requests = captured_policy_requests.lock().unwrap();
    assert_eq!(policy_requests.len(), 1);
    assert_eq!(
        policy_requests[0].typed_category,
        Some(PolicyCategory::ProductSpecific("tool.invoke".to_string()))
    );
    assert_eq!(policy_requests[0].resource, "mcp.tool.protected");

    let mcp_calls = captured_mcp_calls.lock().unwrap();
    assert_eq!(mcp_calls.len(), 1);
    assert_eq!(
        mcp_calls[0].policy_decision_id.as_deref(),
        Some(response.policy_decision.decision_id.as_str())
    );
}

#[test]
fn mcp_tool_execution_service_fails_closed_when_policy_requires_approval() {
    let captured_mcp_calls = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = sdkwork_agent_kernel::RuntimeBuilder::new(
        "runtime.mcp.approval",
        mcp_execution_agent_manifest("agent.mcp.approval", "MCP Approval"),
    )
    .register_mcp_provider(
        "provider.mcp.execution",
        "0.1.0",
        CountingMcpProvider::new(captured_mcp_calls.clone()),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        RecordingStaticPolicyProvider::needs_approval(captured_policy_requests.clone()),
    )
    .bootstrap()
    .expect("mcp approval runtime bootstraps")
    .runtime;

    let error = McpToolExecutionService::new()
        .invoke(
            &runtime,
            McpToolExecutionRequest::new(
                "mcp.execution.approval",
                "mcp.execution",
                ToolCall::new("mcp-tool-call.approval", "mcp.tool.protected", "{}"),
            ),
        )
        .expect_err("approval policy blocks mcp tool execution until approved");

    assert_eq!(error.kind(), KernelErrorKind::PermissionRequired);
    assert!(captured_mcp_calls.lock().unwrap().is_empty());
    assert_eq!(captured_policy_requests.lock().unwrap().len(), 1);
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
    assert_eq!(
        request.trace_context.as_ref().unwrap().span_id,
        "span.skill"
    );
    assert_eq!(request.timeout_ms, Some(45_000));
    assert_eq!(
        request.metadata_value("skill.marketplace.package"),
        Some("pkg.code-review")
    );

    let result =
        AgentSkillResult::succeeded(request.skill_request_id, request.skill_id, "reviewed diff")
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
        ProviderManifest::new("provider.basic", "model", "basic", "0.1.0", vec![])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.basic",
            "ok",
        ))
    }
}

struct AdvancedModelProvider;

impl ModelProvider for AdvancedModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.advanced",
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
            "provider.advanced",
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
            "provider.advanced",
        ))
    }
}

#[derive(Clone)]
struct CountingModelProvider {
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl CountingModelProvider {
    fn new(captured_requests: Arc<Mutex<Vec<ModelRequest>>>) -> Self {
        Self { captured_requests }
    }
}

impl ModelProvider for CountingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.execution",
            "model",
            "counting-model",
            "0.1.0",
            vec!["model.catalog".to_string(), "model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<sdkwork_agent_kernel::ModelDescriptor> {
        vec![sdkwork_agent_kernel::ModelDescriptor::new(
            "model.secure",
            "provider.execution",
            "Secure Model",
            "test",
        )
        .with_capability("chat")
        .with_policy_category("model.invoke")
        .with_policy_category("model.send_sensitive_context")]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.execution",
            "executed",
        ))
    }
}

#[derive(Clone)]
struct CompleteModelProvider {
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
    captured_cancellations: Arc<Mutex<Vec<String>>>,
}

impl CompleteModelProvider {
    fn new(
        captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
        captured_cancellations: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            captured_requests,
            captured_cancellations,
        }
    }
}

impl ModelProvider for CompleteModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.complete",
            "model",
            "complete-model",
            "0.1.0",
            vec![
                "model.catalog".to_string(),
                "model.chat".to_string(),
                "model.streaming".to_string(),
                "model.cancellation".to_string(),
                "model.structured_output".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<sdkwork_agent_kernel::ModelDescriptor> {
        vec![sdkwork_agent_kernel::ModelDescriptor::new(
            "model.complete",
            "provider.complete",
            "Complete Model",
            "test",
        )
        .with_capability("model.chat")
        .with_capability("model.streaming")
        .with_capability("model.cancellation")
        .with_capability("model.structured_output")
        .with_response_format(ModelResponseFormat::Text)
        .with_response_format(ModelResponseFormat::json_schema(
            "sdkwork.answer.schema.v1".to_string(),
        ))
        .with_policy_category("model.invoke")]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.complete",
            "{\"answer\":\"ok\"}",
        ))
    }

    fn stream(&self, request: ModelRequest) -> KernelResult<Vec<ModelStreamChunk>> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(vec![
            ModelStreamChunk::output(request.model_request_id.clone(), 1, "part 1"),
            ModelStreamChunk::output(request.model_request_id, 2, "part 2"),
        ])
    }

    fn cancel(&self, model_request_id: &str) -> KernelResult<ModelResponse> {
        self.captured_cancellations
            .lock()
            .unwrap()
            .push(model_request_id.to_string());
        Ok(ModelResponse::cancelled(
            model_request_id,
            "provider.complete",
        ))
    }

    fn validate_structured_output(
        &self,
        request: &ModelRequest,
        _response: &ModelResponse,
    ) -> KernelResult<ModelStructuredOutputValidation> {
        let schema_id = match &request.response_format {
            Some(ModelResponseFormat::JsonSchema { schema_id, .. }) => schema_id.clone(),
            _ => "text".to_string(),
        };
        Ok(ModelStructuredOutputValidation::valid(
            request.model_request_id.clone(),
            schema_id,
        ))
    }
}

struct InvalidStructuredModelProvider;

impl ModelProvider for InvalidStructuredModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.invalid-structured",
            "model",
            "invalid-structured-model",
            "0.1.0",
            vec![
                "model.catalog".to_string(),
                "model.chat".to_string(),
                "model.structured_output".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<sdkwork_agent_kernel::ModelDescriptor> {
        vec![sdkwork_agent_kernel::ModelDescriptor::new(
            "model.invalid-structured",
            "provider.invalid-structured",
            "Invalid Structured Model",
            "test",
        )
        .with_capability("model.chat")
        .with_capability("model.structured_output")
        .with_response_format(ModelResponseFormat::json_schema(
            "sdkwork.answer.schema.v1".to_string(),
        ))
        .with_policy_category("model.invoke")]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.invalid-structured",
            "not-json",
        ))
    }

    fn validate_structured_output(
        &self,
        request: &ModelRequest,
        _response: &ModelResponse,
    ) -> KernelResult<ModelStructuredOutputValidation> {
        Ok(ModelStructuredOutputValidation::invalid(
            request.model_request_id.clone(),
            "sdkwork.answer.schema.v1",
            vec!["missing required property answer".to_string()],
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

fn tool_execution_agent_manifest(
    agent_id: &str,
    display_name: &str,
) -> sdkwork_agent_kernel::AgentManifest {
    sdkwork_agent_kernel::AgentManifest::from_json(&format!(
        r#"{{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "{agent_id}",
  "name": "{agent_id}",
  "display_name": "{display_name}",
  "description": "Agent used to verify policy-gated tool execution.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {{ "capability_id": "tool.invoke", "min_version": "0.1.0" }},
    {{ "capability_id": "policy.evaluate", "min_version": "0.1.0" }}
  ],
  "optional_capabilities": [],
  "event_families": ["agent.tool.*", "agent.policy.*"],
  "owner": {{ "name": "sdkwork-platform" }},
  "status": "candidate"
}}"#
    ))
    .expect("tool execution agent manifest parses")
}

fn model_execution_agent_manifest(
    agent_id: &str,
    display_name: &str,
) -> sdkwork_agent_kernel::AgentManifest {
    sdkwork_agent_kernel::AgentManifest::from_json(&format!(
        r#"{{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "{agent_id}",
  "name": "{agent_id}",
  "display_name": "{display_name}",
  "description": "Agent used to verify policy-gated model execution.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {{ "capability_id": "model.chat", "min_version": "0.1.0" }},
    {{ "capability_id": "policy.evaluate", "min_version": "0.1.0" }}
  ],
  "optional_capabilities": [],
  "event_families": ["agent.model.*", "agent.policy.*"],
  "owner": {{ "name": "sdkwork-platform" }},
  "status": "candidate"
}}"#
    ))
    .expect("model execution agent manifest parses")
}

fn mcp_execution_agent_manifest(
    agent_id: &str,
    display_name: &str,
) -> sdkwork_agent_kernel::AgentManifest {
    sdkwork_agent_kernel::AgentManifest::from_json(&format!(
        r#"{{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "{agent_id}",
  "name": "{agent_id}",
  "display_name": "{display_name}",
  "description": "Agent used to verify policy-gated MCP tool execution.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {{ "capability_id": "mcp.tools", "min_version": "0.1.0" }},
    {{ "capability_id": "policy.evaluate", "min_version": "0.1.0" }}
  ],
  "optional_capabilities": [],
  "event_families": ["agent.tool.*", "agent.policy.*"],
  "owner": {{ "name": "sdkwork-platform" }},
  "status": "candidate"
}}"#
    ))
    .expect("mcp execution agent manifest parses")
}

#[derive(Clone)]
struct CountingToolProvider {
    captured_calls: Arc<Mutex<Vec<ToolCall>>>,
}

impl CountingToolProvider {
    fn new(captured_calls: Arc<Mutex<Vec<ToolCall>>>) -> Self {
        Self { captured_calls }
    }
}

impl ToolProvider for CountingToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.execution",
            "tool",
            "counting-tool",
            "0.1.0",
            vec!["tool.invoke".to_string(), "tool.discovery".to_string()],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.protected",
            "provider.tool.execution",
            "Protected Tool",
            SideEffectLevel::SideEffectful,
        )
        .with_policy_categories(vec![PolicyCategory::ToolInvoke.as_str().to_string()])
        .require_audit()]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        self.captured_calls.lock().unwrap().push(call.clone());
        Ok(ToolResult::succeeded(call.tool_call_id, "executed"))
    }

    fn stream_tool_call(&self, call: ToolCall) -> KernelResult<Vec<ToolStreamChunk>> {
        self.captured_calls.lock().unwrap().push(call.clone());
        Ok(vec![ToolStreamChunk::output(
            call.tool_call_id,
            1,
            "streamed",
        )])
    }

    fn cancel_tool_call(&self, tool_call_id: &str) -> KernelResult<ToolResult> {
        self.captured_calls.lock().unwrap().push(ToolCall::new(
            tool_call_id,
            "tool.protected",
            "{}",
        ));
        Ok(ToolResult::succeeded(tool_call_id, "cancelled").with_status(ToolCallStatus::Cancelled))
    }
}

#[derive(Clone)]
struct StaticPolicyProvider {
    decision: PolicyDecisionValue,
    reason_code: String,
}

impl StaticPolicyProvider {
    fn allow() -> Self {
        Self {
            decision: PolicyDecisionValue::Allow,
            reason_code: "allowed".to_string(),
        }
    }

    fn deny(reason_code: impl Into<String>) -> Self {
        Self {
            decision: PolicyDecisionValue::Deny,
            reason_code: reason_code.into(),
        }
    }

    fn needs_approval() -> Self {
        Self {
            decision: PolicyDecisionValue::NeedsApproval,
            reason_code: "approval.required".to_string(),
        }
    }
}

impl PolicyProvider for StaticPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.execution",
            "policy",
            "static-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        let decision_id = format!("decision.{}", request.policy_request_id);
        let decision = match self.decision {
            PolicyDecisionValue::Allow => PolicyDecision::allow(
                decision_id,
                request.policy_request_id,
                "provider.policy.execution",
            ),
            PolicyDecisionValue::Deny => PolicyDecision::deny(
                decision_id,
                request.policy_request_id,
                "provider.policy.execution",
                self.reason_code.clone(),
            ),
            PolicyDecisionValue::NeedsApproval => PolicyDecision::needs_approval(
                decision_id,
                request.policy_request_id,
                "provider.policy.execution",
                self.reason_code.clone(),
            )
            .with_safe_reason("approval required"),
            PolicyDecisionValue::Defer => PolicyDecision::defer(
                decision_id,
                request.policy_request_id,
                "provider.policy.execution",
                self.reason_code.clone(),
            ),
        };
        Ok(decision)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct RecordingStaticPolicyProvider {
    captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
    decision: PolicyDecisionValue,
    denied_category: Option<PolicyCategory>,
    reason_code: String,
}

impl RecordingStaticPolicyProvider {
    fn allow(captured_requests: Arc<Mutex<Vec<PolicyRequest>>>) -> Self {
        Self {
            captured_requests,
            decision: PolicyDecisionValue::Allow,
            denied_category: None,
            reason_code: "allowed".to_string(),
        }
    }

    fn needs_approval(captured_requests: Arc<Mutex<Vec<PolicyRequest>>>) -> Self {
        Self {
            captured_requests,
            decision: PolicyDecisionValue::NeedsApproval,
            denied_category: None,
            reason_code: "approval.required".to_string(),
        }
    }

    fn deny_category(
        captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
        category: PolicyCategory,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            captured_requests,
            decision: PolicyDecisionValue::Allow,
            denied_category: Some(category),
            reason_code: reason_code.into(),
        }
    }
}

impl PolicyProvider for RecordingStaticPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.execution",
            "policy",
            "recording-static-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        self.captured_requests.lock().unwrap().push(request.clone());
        let decision_id = format!("decision.{}", request.policy_request_id);
        let decision = if self.denied_category.as_ref() == request.typed_category.as_ref() {
            PolicyDecision::deny(
                decision_id,
                request.policy_request_id,
                "provider.policy.execution",
                self.reason_code.clone(),
            )
        } else {
            match self.decision {
                PolicyDecisionValue::Allow => PolicyDecision::allow(
                    decision_id,
                    request.policy_request_id,
                    "provider.policy.execution",
                ),
                PolicyDecisionValue::Deny => PolicyDecision::deny(
                    decision_id,
                    request.policy_request_id,
                    "provider.policy.execution",
                    self.reason_code.clone(),
                ),
                PolicyDecisionValue::NeedsApproval => PolicyDecision::needs_approval(
                    decision_id,
                    request.policy_request_id,
                    "provider.policy.execution",
                    self.reason_code.clone(),
                )
                .with_safe_reason("approval required"),
                PolicyDecisionValue::Defer => PolicyDecision::defer(
                    decision_id,
                    request.policy_request_id,
                    "provider.policy.execution",
                    self.reason_code.clone(),
                ),
            }
        };
        Ok(decision)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct CountingMcpProvider {
    captured_calls: Arc<Mutex<Vec<ToolCall>>>,
}

impl CountingMcpProvider {
    fn new(captured_calls: Arc<Mutex<Vec<ToolCall>>>) -> Self {
        Self { captured_calls }
    }
}

impl McpProvider for CountingMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.mcp.execution",
            "mcp",
            "counting-mcp",
            "0.1.0",
            vec!["mcp.tools".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_servers(&self) -> KernelResult<Vec<McpServerDescriptor>> {
        Ok(vec![McpServerDescriptor::new(
            "mcp.execution",
            "provider.mcp.execution",
            McpTransportKind::Sse,
        )
        .with_capability("mcp.tools")])
    }

    fn list_tools(&self, _server_id: &str) -> KernelResult<Vec<ToolDescriptor>> {
        Ok(vec![ToolDescriptor::new(
            "mcp.tool.protected",
            "provider.mcp.execution",
            "Protected MCP Tool",
            SideEffectLevel::SideEffectful,
        )
        .with_policy_categories(vec![PolicyCategory::ToolInvoke.as_str().to_string()])
        .require_audit()])
    }

    fn invoke_tool(&self, _server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        self.captured_calls.lock().unwrap().push(call.clone());
        Ok(ToolResult::succeeded(call.tool_call_id, "mcp executed"))
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
