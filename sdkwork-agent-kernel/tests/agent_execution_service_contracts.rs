use sdkwork_agent_kernel::{
    Action, ActionKind, AgentExecutionRequest, AgentExecutionResumeDecision,
    AgentExecutionResumeRequest, AgentExecutionService, AgentExecutionStatus, AgentManifest,
    KernelErrorKind, KernelEventRedaction, KernelEventSeverity, KernelEventSource, KernelResult,
    KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, KnowledgeSearchResult, McpProvider,
    McpServerDescriptor, McpTransportKind, MemoryProvider, MemoryRecord, MemoryScope,
    ModelProvider, ModelRequest, ModelResponse, ModelStatus, Plan, PlanningProvider,
    PolicyCategory, PolicyDecision, PolicyProvider, PolicyRequest, PolicySubject, ProviderHealth,
    ProviderManifest, RedactionClassification, RuntimeBuilder, SideEffectLevel, ToolCall,
    ToolCallStatus, ToolDescriptor, ToolProvider, ToolResult, TraceContext, TrustLevel,
};
use std::sync::{Arc, Mutex};

const EXECUTION_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.execution",
  "name": "sdkwork-execution-agent",
  "display_name": "SDKWork Execution Agent",
  "description": "Agent used to prove bounded execution loop contracts.",
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

const MODEL_REQUIRED_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.execution-missing-model",
  "name": "sdkwork-execution-missing-model-agent",
  "display_name": "SDKWork Execution Missing Model Agent",
  "description": "Agent used to prove failed runtimes stop execution.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
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

#[test]
fn execution_service_rejects_blank_execution_id_before_runtime_work() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_recording_model(captured_model_requests.clone());

    let error = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new(" ", vec!["hello".to_string()]),
        )
        .expect_err("blank execution id is invalid");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid execution request must not invoke model provider"
    );
}

#[test]
fn execution_service_rejects_blank_messages_before_runtime_work() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_recording_model(captured_model_requests.clone());

    let error = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.blank-message", vec![" ".to_string()]),
        )
        .expect_err("blank messages are invalid");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid execution request must not invoke model provider"
    );
}

#[test]
fn execution_service_fails_closed_when_runtime_is_failed() {
    let runtime = RuntimeBuilder::new(
        "runtime.execution.failed",
        AgentManifest::from_json(MODEL_REQUIRED_AGENT_MANIFEST_JSON)
            .expect("missing model manifest parses"),
    )
    .bootstrap()
    .expect("failed runtime bootstrap still returns report")
    .runtime;

    let error = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.failed-runtime", vec!["hello".to_string()]),
        )
        .expect_err("failed runtime stops before execution");

    assert_eq!(error.kind(), KernelErrorKind::CapabilityMissing);
}

#[test]
fn execution_service_creates_plan_and_invokes_selected_model_provider() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_recording_model_and_planner(captured_model_requests.clone());

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new(
                "execution.plan.model",
                vec!["summarize repository".to_string()],
            )
            .with_provider_id("provider.model.recording")
            .for_session("session.execution")
            .for_task("task.execution")
            .for_run("run.execution"),
        )
        .expect("execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    assert_eq!(report.plan.as_ref().unwrap().task_id, "task.execution");
    assert_eq!(captured_model_requests.lock().unwrap().len(), 1);
    assert_eq!(
        report.model_response.as_ref().unwrap().provider_id,
        "provider.model.recording"
    );
}

#[test]
fn execution_service_attaches_memory_and_knowledge_context_through_chat_service() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime =
        runtime_with_memory_knowledge_and_recording_model(captured_model_requests.clone());

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.context", vec!["use known context".to_string()])
                .with_provider_id("provider.model.recording")
                .for_session("session.context")
                .for_task("task.context")
                .with_memory_query(MemoryScope::Session, "session.context")
                .with_knowledge_query("kernel knowledge")
                .with_knowledge_provider_id("provider.knowledge.execution")
                .with_knowledge_namespace("docs"),
        )
        .expect("context execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    let requests = captured_model_requests.lock().unwrap();
    assert_eq!(requests[0].context_frames.len(), 2);
    assert_eq!(
        requests[0].context_frames[0].metadata_value("sdkwork.memory.record_id"),
        Some("memory.execution.1")
    );
    assert_eq!(
        requests[0].context_frames[1].metadata_value("sdkwork.knowledge.document_id"),
        Some("knowledge.execution.1")
    );
}

#[test]
fn execution_service_maps_full_chat_context_to_model_knowledge_and_policy_requests() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_knowledge_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_recording_model_knowledge_and_policy(
        captured_model_requests.clone(),
        captured_knowledge_requests.clone(),
        captured_policy_requests.clone(),
    );
    let subject = PolicySubject::new("user.execution", "tenant.execution").with_role("developer");
    let trace = TraceContext::new("trace.execution", "span.execution");

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new(
                "execution.full-context",
                vec!["use complete context".to_string()],
            )
            .with_provider_id("provider.model.recording")
            .with_model_id("model.execution.fast")
            .for_session("session.execution.full")
            .for_task("task.execution.full")
            .for_run("run.execution.full")
            .for_step("step.execution.full")
            .with_subject(subject.clone())
            .with_trace_context(trace.clone())
            .with_timeout_ms(30_000)
            .with_metadata("sdkwork.execution.source", "contract")
            .with_knowledge_query("kernel architecture")
            .with_knowledge_provider_id("provider.knowledge.capturing")
            .with_knowledge_tenant_id("tenant.execution")
            .with_knowledge_namespace("docs")
            .with_knowledge_top_k(3)
            .with_knowledge_method(KnowledgeRetrievalMethod::Hybrid)
            .with_knowledge_filter("space_id", "7")
            .include_external_knowledge(),
        )
        .expect("full context execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(
        model_requests[0].model_id.as_deref(),
        Some("model.execution.fast")
    );
    assert_eq!(
        model_requests[0].step_id.as_deref(),
        Some("step.execution.full")
    );
    assert_eq!(model_requests[0].trace_context, Some(trace.clone()));
    assert_eq!(model_requests[0].timeout_ms, Some(30_000));
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.execution.source"),
        Some("contract")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.knowledge.tenant_id"),
        Some("tenant.execution")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.knowledge.top_k"),
        Some("3")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.knowledge.methods"),
        Some("hybrid")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.knowledge.filter.space_id"),
        Some("7")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.knowledge.include_external"),
        Some("true")
    );

    let knowledge_requests = captured_knowledge_requests.lock().unwrap();
    assert_eq!(knowledge_requests.len(), 1);
    assert_eq!(
        knowledge_requests[0].tenant_id.as_deref(),
        Some("tenant.execution")
    );
    assert_eq!(
        knowledge_requests[0].step_id.as_deref(),
        Some("step.execution.full")
    );
    assert_eq!(knowledge_requests[0].top_k, 3);
    assert!(knowledge_requests[0].supports_method(KnowledgeRetrievalMethod::Hybrid));
    assert_eq!(
        knowledge_requests[0].filters,
        vec![("space_id".to_string(), "7".to_string())]
    );
    assert!(knowledge_requests[0].include_external);
    assert_eq!(knowledge_requests[0].trace_context, Some(trace));
    assert_eq!(knowledge_requests[0].timeout_ms, Some(30_000));

    let policy_requests = captured_policy_requests.lock().unwrap();
    let model_policy = policy_requests
        .iter()
        .find(|request| request.category == PolicyCategory::ModelInvoke.as_str())
        .expect("model invoke policy request is captured");
    assert_eq!(model_policy.subject, Some(subject));
    assert_eq!(
        model_policy.context_value("step_id"),
        Some("step.execution.full")
    );
    assert_eq!(
        model_policy.context_value("model_id"),
        Some("model.execution.fast")
    );
}

#[test]
fn execution_service_returns_failed_report_when_model_policy_denies_before_provider_invocation() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_model_policy_denial(captured_model_requests.clone());

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.model.denied", vec!["hello".to_string()])
                .with_provider_id("provider.model.recording"),
        )
        .expect("model policy denial is represented as an execution report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
    assert!(report.model_response.is_none());
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::PolicyDenied
    );
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "policy denial must stop before model provider invocation"
    );
}

#[test]
fn execution_service_returns_permission_report_when_model_policy_requires_approval() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_model_policy_approval(captured_model_requests.clone());

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.model.approval", vec!["hello".to_string()])
                .with_provider_id("provider.model.recording"),
        )
        .expect("model policy approval requirement is represented as an execution report");

    assert_eq!(report.status, AgentExecutionStatus::PermissionRequired);
    assert!(report.model_response.is_none());
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::PermissionRequired
    );
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "policy approval requirement must stop before model provider invocation"
    );
}

#[test]
fn execution_service_maps_non_success_model_statuses_to_execution_reports() {
    let cases = [
        (
            ModelStatus::Failed,
            AgentExecutionStatus::Failed,
            KernelErrorKind::ProviderError,
            "failed",
        ),
        (
            ModelStatus::Cancelled,
            AgentExecutionStatus::Cancelled,
            KernelErrorKind::Cancelled,
            "cancelled",
        ),
        (
            ModelStatus::TimedOut,
            AgentExecutionStatus::Failed,
            KernelErrorKind::Timeout,
            "timed_out",
        ),
        (
            ModelStatus::PolicyDenied,
            AgentExecutionStatus::Failed,
            KernelErrorKind::PolicyDenied,
            "policy_denied",
        ),
    ];

    for (model_status, execution_status, error_kind, observation_status) in cases {
        let runtime = runtime_with_model_status(model_status.clone());

        let report = AgentExecutionService::new()
            .execute(
                &runtime,
                AgentExecutionRequest::new(
                    format!("execution.model.status.{observation_status}"),
                    vec!["invoke model".to_string()],
                )
                .with_provider_id("provider.model.status"),
            )
            .expect("non-success model status is represented as execution report");

        assert_eq!(report.status, execution_status);
        assert_eq!(report.model_response.as_ref().unwrap().status, model_status);
        assert_eq!(report.error.as_ref().unwrap().kind(), error_kind);
        assert_eq!(report.observations.len(), 1);
        assert_eq!(report.observations[0].source_family, "model");
        assert_eq!(report.observations[0].status, observation_status);
        assert!(report.tool_executions.is_empty());
        assert!(report.mcp_tool_executions.is_empty());
    }
}

#[test]
fn execution_service_executes_model_tool_calls_through_tool_service() {
    let runtime = runtime_with_tool_calling_model_and_tool_provider();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.tool", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .include_tool_descriptors(),
        )
        .expect("tool execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    assert_eq!(report.tool_executions.len(), 1);
    assert_eq!(report.tool_executions[0].result.status, "succeeded");
    assert_eq!(
        report
            .observations
            .iter()
            .filter(|observation| observation.source_family == "tool")
            .count(),
        1
    );
}

#[test]
fn execution_service_enriches_model_tool_calls_with_execution_context() {
    let captured_tool_calls = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_context_capturing_tool(
        captured_tool_calls.clone(),
        captured_policy_requests.clone(),
    );
    let trace = TraceContext::new("trace.execution.tool", "span.execution.tool");

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.tool.context", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .for_session("session.tool.context")
                .for_task("task.tool.context")
                .for_run("run.tool.context")
                .for_step("step.tool.context")
                .with_trace_context(trace.clone())
                .with_timeout_ms(15_000),
        )
        .expect("tool context execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);

    let tool_calls = captured_tool_calls.lock().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(
        tool_calls[0].session_id.as_deref(),
        Some("session.tool.context")
    );
    assert_eq!(tool_calls[0].task_id.as_deref(), Some("task.tool.context"));
    assert_eq!(tool_calls[0].run_id.as_deref(), Some("run.tool.context"));
    assert_eq!(tool_calls[0].step_id.as_deref(), Some("step.tool.context"));
    assert_eq!(tool_calls[0].trace_context, Some(trace));
    assert_eq!(tool_calls[0].timeout_ms, Some(15_000));

    let policy_requests = captured_policy_requests.lock().unwrap();
    let tool_policy = policy_requests
        .iter()
        .find(|request| request.category == PolicyCategory::ToolInvoke.as_str())
        .expect("tool invoke policy request is captured");
    assert_eq!(
        tool_policy.session_id.as_deref(),
        Some("session.tool.context")
    );
    assert_eq!(tool_policy.task_id.as_deref(), Some("task.tool.context"));
    assert_eq!(tool_policy.run_id.as_deref(), Some("run.tool.context"));
    assert_eq!(
        tool_policy.context_value("step_id"),
        Some("step.tool.context")
    );
}

#[test]
fn execution_service_stops_before_tool_when_policy_requires_approval() {
    let runtime = runtime_with_tool_calling_model_and_approval_policy();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.tool.approval", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .include_tool_descriptors(),
        )
        .expect("permission required is returned as report");

    assert_eq!(report.status, AgentExecutionStatus::PermissionRequired);
    assert!(report.tool_executions.is_empty());
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::PermissionRequired
    );
}

#[test]
fn execution_service_does_not_fallback_to_mcp_when_default_tool_policy_requires_approval() {
    let captured_mcp_list_calls = Arc::new(Mutex::new(0usize));
    let runtime = runtime_with_default_tool_approval_and_mcp_fallback_candidate(
        captured_mcp_list_calls.clone(),
    );

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.tool.no-fallback", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-no-provider")
                .with_mcp_server_id("mcp.execution"),
        )
        .expect("tool approval requirement is returned as report");

    assert_eq!(report.status, AgentExecutionStatus::PermissionRequired);
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::PermissionRequired
    );
    assert_eq!(
        *captured_mcp_list_calls.lock().unwrap(),
        0,
        "MCP fallback must not run when the default tool was recognized but blocked by policy"
    );
}

#[test]
fn execution_service_executes_mcp_tool_calls_through_mcp_service() {
    let runtime = runtime_with_mcp_tool_calling_model_and_mcp_provider();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.mcp", vec!["call mcp".to_string()])
                .with_provider_id("provider.model.mcp-tool-calling")
                .with_mcp_server_id("mcp.execution"),
        )
        .expect("mcp execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);
    assert_eq!(report.mcp_tool_executions.len(), 1);
    assert_eq!(report.mcp_tool_executions[0].result.status, "succeeded");
    assert_eq!(
        report
            .observations
            .iter()
            .filter(|observation| observation.source_family == "mcp")
            .count(),
        1
    );
}

#[test]
fn execution_service_enriches_model_mcp_tool_calls_with_execution_context() {
    let captured_mcp_tool_calls = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = runtime_with_context_capturing_mcp(
        captured_mcp_tool_calls.clone(),
        captured_policy_requests.clone(),
    );
    let trace = TraceContext::new("trace.execution.mcp", "span.execution.mcp");

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.mcp.context", vec!["call mcp".to_string()])
                .with_provider_id("provider.model.mcp-tool-calling")
                .with_mcp_server_id("mcp.execution")
                .for_session("session.mcp.context")
                .for_task("task.mcp.context")
                .for_run("run.mcp.context")
                .for_step("step.mcp.context")
                .with_trace_context(trace.clone())
                .with_timeout_ms(20_000),
        )
        .expect("mcp context execution succeeds");

    assert_eq!(report.status, AgentExecutionStatus::Completed);

    let mcp_tool_calls = captured_mcp_tool_calls.lock().unwrap();
    assert_eq!(mcp_tool_calls.len(), 1);
    assert_eq!(
        mcp_tool_calls[0].session_id.as_deref(),
        Some("session.mcp.context")
    );
    assert_eq!(
        mcp_tool_calls[0].task_id.as_deref(),
        Some("task.mcp.context")
    );
    assert_eq!(mcp_tool_calls[0].run_id.as_deref(), Some("run.mcp.context"));
    assert_eq!(
        mcp_tool_calls[0].step_id.as_deref(),
        Some("step.mcp.context")
    );
    assert_eq!(mcp_tool_calls[0].trace_context, Some(trace));
    assert_eq!(mcp_tool_calls[0].timeout_ms, Some(20_000));

    let policy_requests = captured_policy_requests.lock().unwrap();
    let mcp_policy = policy_requests
        .iter()
        .find(|request| request.category == PolicyCategory::ToolInvoke.as_str())
        .expect("MCP tool invoke policy request is captured");
    assert_eq!(
        mcp_policy.session_id.as_deref(),
        Some("session.mcp.context")
    );
    assert_eq!(mcp_policy.task_id.as_deref(), Some("task.mcp.context"));
    assert_eq!(mcp_policy.run_id.as_deref(), Some("run.mcp.context"));
    assert_eq!(
        mcp_policy.context_value("step_id"),
        Some("step.mcp.context")
    );
}

#[test]
fn execution_service_stops_before_mcp_tool_when_policy_requires_approval() {
    let runtime = runtime_with_mcp_tool_calling_model_and_approval_policy();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.mcp.approval", vec!["call mcp".to_string()])
                .with_provider_id("provider.model.mcp-tool-calling")
                .with_mcp_server_id("mcp.execution"),
        )
        .expect("mcp permission required is returned as report");

    assert_eq!(report.status, AgentExecutionStatus::PermissionRequired);
    assert!(report.mcp_tool_executions.is_empty());
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::PermissionRequired
    );
}

#[test]
fn execution_service_preserves_prior_observations_when_later_tool_fails() {
    let runtime = runtime_with_two_tool_calls_second_fails();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.partial", vec!["call tools".to_string()])
                .with_provider_id("provider.model.two-tools"),
        )
        .expect("tool failure is represented in report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
    let tool_observations: Vec<_> = report
        .observations
        .iter()
        .filter(|observation| observation.source_family == "tool")
        .collect();
    assert_eq!(tool_observations.len(), 2);
    assert_eq!(tool_observations[0].status, "succeeded");
    assert_eq!(tool_observations[1].status, "failed");
    assert!(report.error.is_some());
}

#[test]
fn execution_service_fails_closed_with_observation_for_unknown_tool_call() {
    let runtime = runtime_with_unknown_tool_calling_model();

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.unknown-tool", vec!["call tool".to_string()])
                .with_provider_id("provider.model.unknown-tool"),
        )
        .expect("unknown tool is represented in report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
    assert!(report.tool_executions.is_empty());
    let tool_observations: Vec<_> = report
        .observations
        .iter()
        .filter(|observation| observation.source_family == "tool")
        .collect();
    assert_eq!(tool_observations.len(), 1);
    assert_eq!(tool_observations[0].status, "failed");
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::CapabilityMissing
    );
}

#[test]
fn execution_report_projects_completed_run_to_kernel_events_with_context() {
    let runtime = runtime_with_tool_calling_model_and_tool_provider();
    let trace = TraceContext::new("trace.execution.events", "span.execution.events");

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.events", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .for_session("session.execution.events")
                .for_task("task.execution.events")
                .for_run("run.execution.events")
                .for_step("step.execution.events")
                .with_trace_context(trace.clone()),
        )
        .expect("execution succeeds");

    let events = report.to_events("event.execution.events");

    assert_eq!(events.len(), report.observations.len() + 1);
    assert_eq!(events[0].event_id, "event.execution.events.report");
    assert_eq!(events[0].event_type, "agent.execution.completed");
    assert_eq!(events[0].source, KernelEventSource::Runtime);
    assert_eq!(events[0].severity, KernelEventSeverity::Info);
    assert_eq!(
        events[0].session_id.as_deref(),
        Some("session.execution.events")
    );
    assert_eq!(events[0].task_id.as_deref(), Some("task.execution.events"));
    assert_eq!(events[0].run_id.as_deref(), Some("run.execution.events"));
    assert_eq!(events[0].step_id.as_deref(), Some("step.execution.events"));
    assert_eq!(events[0].trace_context, Some(trace.clone()));
    assert_eq!(
        events[0].redaction_classification,
        KernelEventRedaction::Internal
    );
    assert_eq!(
        events[0].payload_schema.as_deref(),
        Some("sdkwork.agent.execution.report.v1")
    );
    assert!(events[0].payload.contains("execution_id=execution.events"));
    assert!(events[0].payload.contains("status=completed"));
    assert!(events[0].payload.contains("observations=2"));

    assert_eq!(events[1].event_id, "event.execution.events.observation.1");
    assert_eq!(events[1].event_type, "agent.execution.observation.model");
    assert_eq!(events[1].source, KernelEventSource::Model);
    assert_eq!(
        events[1].causation_id.as_deref(),
        Some("event.execution.events.report")
    );
    assert_eq!(events[1].trace_context, Some(trace.clone()));
    assert!(events[1].payload.contains("source_family=model"));

    assert_eq!(events[2].event_id, "event.execution.events.observation.2");
    assert_eq!(events[2].event_type, "agent.execution.observation.tool");
    assert_eq!(events[2].source, KernelEventSource::Tool);
    assert_eq!(
        events[2].payload_schema.as_deref(),
        Some("sdkwork.agent.execution.observation.v1")
    );
    assert!(events[2].payload.contains("status=succeeded"));
}

#[test]
fn execution_report_projects_failure_and_permission_status_to_event_severity() {
    let failed_report = AgentExecutionService::new()
        .execute(
            &runtime_with_two_tool_calls_second_fails(),
            AgentExecutionRequest::new("execution.events.failed", vec!["call tools".to_string()])
                .with_provider_id("provider.model.two-tools"),
        )
        .expect("tool failure is represented in report");
    let failed_event = failed_report.to_event("event.execution.failed.report");

    assert_eq!(failed_event.event_type, "agent.execution.failed");
    assert_eq!(failed_event.source, KernelEventSource::Runtime);
    assert_eq!(failed_event.severity, KernelEventSeverity::Error);
    assert!(failed_event.payload.contains("error_kind=provider_error"));

    let permission_report = AgentExecutionService::new()
        .execute(
            &runtime_with_tool_calling_model_and_approval_policy(),
            AgentExecutionRequest::new(
                "execution.events.permission",
                vec!["call tool".to_string()],
            )
            .with_provider_id("provider.model.tool-calling"),
        )
        .expect("approval requirement is represented in report");
    let permission_event = permission_report.to_event("event.execution.permission.report");

    assert_eq!(
        permission_event.event_type,
        "agent.execution.permission_required"
    );
    assert_eq!(permission_event.source, KernelEventSource::Runtime);
    assert_eq!(permission_event.severity, KernelEventSeverity::Warn);
    assert!(permission_event
        .payload
        .contains("error_kind=permission_required"));
}

#[test]
fn permission_report_builds_approval_resume_request_with_context_and_audit_event() {
    let trace = TraceContext::new("trace.execution.resume", "span.execution.resume");
    let report = AgentExecutionService::new()
        .execute(
            &runtime_with_tool_calling_model_and_approval_policy(),
            AgentExecutionRequest::new("execution.resume", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling")
                .for_session("session.execution.resume")
                .for_task("task.execution.resume")
                .for_run("run.execution.resume")
                .for_step("step.execution.resume")
                .with_trace_context(trace.clone()),
        )
        .expect("approval requirement is represented in report");

    let resume_request = report
        .approval_resume_request(
            "resume.execution.approved",
            AgentExecutionResumeDecision::Approved,
            "operator.execution",
        )
        .expect("permission report can build approval resume request")
        .with_comment("approved for contract test");

    assert_eq!(
        resume_request.resume_request_id,
        "resume.execution.approved"
    );
    assert_eq!(resume_request.execution_id, "execution.resume");
    assert_eq!(
        resume_request.decision,
        AgentExecutionResumeDecision::Approved
    );
    assert_eq!(
        resume_request.approved_by.as_deref(),
        Some("operator.execution")
    );
    assert_eq!(
        resume_request.session_id.as_deref(),
        Some("session.execution.resume")
    );
    assert_eq!(
        resume_request.task_id.as_deref(),
        Some("task.execution.resume")
    );
    assert_eq!(
        resume_request.run_id.as_deref(),
        Some("run.execution.resume")
    );
    assert_eq!(
        resume_request.step_id.as_deref(),
        Some("step.execution.resume")
    );
    assert_eq!(resume_request.trace_context, Some(trace.clone()));
    assert_eq!(
        resume_request.permission_error_kind.as_deref(),
        Some("permission_required")
    );

    let event = resume_request.to_event("event.execution.resume.approved");

    assert_eq!(event.event_type, "agent.execution.resume.approved");
    assert_eq!(event.source, KernelEventSource::Policy);
    assert_eq!(event.severity, KernelEventSeverity::Info);
    assert_eq!(
        event.session_id.as_deref(),
        Some("session.execution.resume")
    );
    assert_eq!(event.task_id.as_deref(), Some("task.execution.resume"));
    assert_eq!(event.run_id.as_deref(), Some("run.execution.resume"));
    assert_eq!(event.step_id.as_deref(), Some("step.execution.resume"));
    assert_eq!(event.trace_context, Some(trace));
    assert_eq!(
        event.payload_schema.as_deref(),
        Some("sdkwork.agent.execution.resume_request.v1")
    );
    assert!(event.payload.contains("execution_id=execution.resume"));
    assert!(event.payload.contains("approved_by=operator.execution"));
}

#[test]
fn non_permission_report_rejects_approval_resume_request() {
    let report = AgentExecutionService::new()
        .execute(
            &runtime_with_tool_calling_model_and_tool_provider(),
            AgentExecutionRequest::new("execution.no-resume", vec!["call tool".to_string()])
                .with_provider_id("provider.model.tool-calling"),
        )
        .expect("execution succeeds");

    let error = report
        .approval_resume_request(
            "resume.execution.invalid",
            AgentExecutionResumeDecision::Approved,
            "operator.execution",
        )
        .expect_err("completed execution cannot build approval resume request");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn resume_request_rejects_blank_approval_actor_for_approved_decision() {
    let error = AgentExecutionResumeRequest::new(
        "resume.execution.blank-actor",
        "execution.resume",
        AgentExecutionResumeDecision::Approved,
    )
    .expect_err("approval actor is required for approved resume");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
}

fn runtime_with_recording_model(
    captured_model_requests: Arc<Mutex<Vec<ModelRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.recording",
        "0.1.0",
        RecordingModelProvider::new("provider.model.recording", captured_model_requests),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .bootstrap()
    .expect("execution runtime bootstraps")
    .runtime
}

fn runtime_with_unknown_tool_calling_model() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.unknown-tool",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.unknown-tool",
        "0.1.0",
        UnknownToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .register_tool_provider("provider.tool.execution", "0.1.0", RecordingToolProvider)
    .bootstrap()
    .expect("execution runtime with unknown tool model bootstraps")
    .runtime
}

fn runtime_with_two_tool_calls_second_fails() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.partial",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.two-tools",
        "0.1.0",
        TwoToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .register_tool_provider(
        "provider.tool.partial",
        "0.1.0",
        PartiallyFailingToolProvider,
    )
    .bootstrap()
    .expect("execution runtime with partial tool provider bootstraps")
    .runtime
}

fn runtime_with_mcp_tool_calling_model_and_mcp_provider() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.mcp",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.mcp-tool-calling",
        "0.1.0",
        McpToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .register_mcp_provider("provider.mcp.execution", "0.1.0", RecordingMcpProvider)
    .bootstrap()
    .expect("execution runtime with mcp provider bootstraps")
    .runtime
}

fn runtime_with_mcp_tool_calling_model_and_approval_policy() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.mcp.approval",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.mcp-tool-calling",
        "0.1.0",
        McpToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticNeedsApprovalPolicyProvider,
    )
    .register_mcp_provider("provider.mcp.execution", "0.1.0", RecordingMcpProvider)
    .bootstrap()
    .expect("execution runtime with mcp approval policy bootstraps")
    .runtime
}

fn runtime_with_tool_calling_model_and_approval_policy() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.tool.approval",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.tool-calling",
        "0.1.0",
        ToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticNeedsApprovalPolicyProvider,
    )
    .register_tool_provider("provider.tool.execution", "0.1.0", RecordingToolProvider)
    .bootstrap()
    .expect("execution runtime with approval policy bootstraps")
    .runtime
}

fn runtime_with_default_tool_approval_and_mcp_fallback_candidate(
    captured_mcp_list_calls: Arc<Mutex<usize>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.tool.no-fallback",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.tool-no-provider",
        "0.1.0",
        ToolCallingModelWithoutProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticNeedsApprovalPolicyProvider,
    )
    .register_tool_provider("provider.tool.execution", "0.1.0", RecordingToolProvider)
    .register_mcp_provider(
        "provider.mcp.execution",
        "0.1.0",
        CountingMcpProvider::new(captured_mcp_list_calls),
    )
    .bootstrap()
    .expect("execution runtime with approval-blocked default tool bootstraps")
    .runtime
}

fn runtime_with_context_capturing_tool(
    captured_tool_calls: Arc<Mutex<Vec<ToolCall>>>,
    captured_policy_requests: Arc<Mutex<Vec<PolicyRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.tool.context",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.tool-calling",
        "0.1.0",
        ToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.recording",
        "0.1.0",
        RecordingAllowPolicyProvider::new(captured_policy_requests),
    )
    .register_tool_provider(
        "provider.tool.execution",
        "0.1.0",
        CapturingToolProvider::new(captured_tool_calls),
    )
    .bootstrap()
    .expect("execution runtime with context-capturing tool provider bootstraps")
    .runtime
}

fn runtime_with_tool_calling_model_and_tool_provider() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.tool",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.tool-calling",
        "0.1.0",
        ToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .register_tool_provider("provider.tool.execution", "0.1.0", RecordingToolProvider)
    .bootstrap()
    .expect("execution runtime with tool provider bootstraps")
    .runtime
}

fn runtime_with_context_capturing_mcp(
    captured_mcp_tool_calls: Arc<Mutex<Vec<ToolCall>>>,
    captured_policy_requests: Arc<Mutex<Vec<PolicyRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.mcp.context",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.mcp-tool-calling",
        "0.1.0",
        McpToolCallingModelProvider,
    )
    .register_policy_provider(
        "provider.policy.recording",
        "0.1.0",
        RecordingAllowPolicyProvider::new(captured_policy_requests),
    )
    .register_mcp_provider(
        "provider.mcp.execution",
        "0.1.0",
        CapturingMcpProvider::new(captured_mcp_tool_calls),
    )
    .bootstrap()
    .expect("execution runtime with context-capturing mcp provider bootstraps")
    .runtime
}

fn runtime_with_memory_knowledge_and_recording_model(
    captured_model_requests: Arc<Mutex<Vec<ModelRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.context",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.recording",
        "0.1.0",
        RecordingModelProvider::new("provider.model.recording", captured_model_requests),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .register_memory_provider(
        "provider.memory.execution",
        "0.1.0",
        RecordingMemoryProvider,
    )
    .register_knowledge_provider(
        "provider.knowledge.execution",
        "0.1.0",
        RecordingKnowledgeProvider,
    )
    .bootstrap()
    .expect("execution runtime with context providers bootstraps")
    .runtime
}

fn runtime_with_recording_model_knowledge_and_policy(
    captured_model_requests: Arc<Mutex<Vec<ModelRequest>>>,
    captured_knowledge_requests: Arc<Mutex<Vec<KnowledgeSearchRequest>>>,
    captured_policy_requests: Arc<Mutex<Vec<PolicyRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.full-context",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.recording",
        "0.1.0",
        RecordingModelProvider::new("provider.model.recording", captured_model_requests),
    )
    .register_policy_provider(
        "provider.policy.recording",
        "0.1.0",
        RecordingAllowPolicyProvider::new(captured_policy_requests),
    )
    .register_knowledge_provider(
        "provider.knowledge.capturing",
        "0.1.0",
        CapturingKnowledgeProvider::new(captured_knowledge_requests),
    )
    .bootstrap()
    .expect("execution runtime with capturing context providers bootstraps")
    .runtime
}

fn runtime_with_model_policy_denial(
    captured_model_requests: Arc<Mutex<Vec<ModelRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.model-denied",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.recording",
        "0.1.0",
        RecordingModelProvider::new("provider.model.recording", captured_model_requests),
    )
    .register_policy_provider(
        "provider.policy.model-deny",
        "0.1.0",
        ModelInvokeDenyPolicyProvider,
    )
    .bootstrap()
    .expect("execution runtime with model-deny policy bootstraps")
    .runtime
}

fn runtime_with_model_policy_approval(
    captured_model_requests: Arc<Mutex<Vec<ModelRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.model-approval",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.recording",
        "0.1.0",
        RecordingModelProvider::new("provider.model.recording", captured_model_requests),
    )
    .register_policy_provider(
        "provider.policy.model-approval",
        "0.1.0",
        ModelInvokeNeedsApprovalPolicyProvider,
    )
    .bootstrap()
    .expect("execution runtime with model-approval policy bootstraps")
    .runtime
}

fn runtime_with_model_status(model_status: ModelStatus) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.model-status",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.status",
        "0.1.0",
        StatusModelProvider::new(model_status),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .bootstrap()
    .expect("execution runtime with status model provider bootstraps")
    .runtime
}

fn runtime_with_recording_model_and_planner(
    captured_model_requests: Arc<Mutex<Vec<ModelRequest>>>,
) -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.execution.planning",
        AgentManifest::from_json(EXECUTION_AGENT_MANIFEST_JSON).expect("execution manifest parses"),
    )
    .with_generated_at("2026-06-10T00:00:00Z")
    .register_model_provider(
        "provider.model.recording",
        "0.1.0",
        RecordingModelProvider::new("provider.model.recording", captured_model_requests),
    )
    .register_policy_provider(
        "provider.policy.execution",
        "0.1.0",
        StaticAllowPolicyProvider,
    )
    .register_planning_provider(
        "provider.planning.execution",
        "0.1.0",
        ExecutionPlanningProvider,
    )
    .bootstrap()
    .expect("execution runtime with planner bootstraps")
    .runtime
}

#[derive(Clone)]
struct RecordingModelProvider {
    provider_id: String,
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl RecordingModelProvider {
    fn new(
        provider_id: impl Into<String>,
        captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            captured_requests,
        }
    }
}

impl ModelProvider for RecordingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "recording-execution-model",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            "recorded",
        ))
    }
}

#[derive(Clone)]
struct StatusModelProvider {
    model_status: ModelStatus,
}

impl StatusModelProvider {
    fn new(model_status: ModelStatus) -> Self {
        Self { model_status }
    }
}

impl ModelProvider for StatusModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.status",
            "model",
            "status-execution-model",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.status",
            "status response",
        )
        .with_status(self.model_status.clone()))
    }
}

#[derive(Clone)]
struct ToolCallingModelProvider;

impl ModelProvider for ToolCallingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.tool-calling",
            "model",
            "tool-calling-execution-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.tool_call".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.tool-calling",
            "tool call requested",
        )
        .with_tool_call(
            ToolCall::new(
                "tool-call.execution.1",
                "tool.execution.echo",
                r#"{"value":"hello"}"#,
            )
            .with_provider("provider.tool.execution"),
        ))
    }
}

#[derive(Clone)]
struct ToolCallingModelWithoutProvider;

impl ModelProvider for ToolCallingModelWithoutProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.tool-no-provider",
            "model",
            "tool-calling-no-provider-execution-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.tool_call".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.tool-no-provider",
            "tool call requested",
        )
        .with_tool_call(ToolCall::new(
            "tool-call.execution.no-provider",
            "tool.execution.echo",
            r#"{"value":"hello"}"#,
        )))
    }
}

#[derive(Clone)]
struct McpToolCallingModelProvider;

impl ModelProvider for McpToolCallingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.mcp-tool-calling",
            "model",
            "mcp-tool-calling-execution-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.tool_call".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.mcp-tool-calling",
            "mcp tool call requested",
        )
        .with_tool_call(
            ToolCall::new(
                "mcp-tool-call.execution.1",
                "mcp.tool.execution.echo",
                r#"{"value":"hello"}"#,
            )
            .with_provider("provider.mcp.execution"),
        ))
    }
}

#[derive(Clone)]
struct TwoToolCallingModelProvider;

impl ModelProvider for TwoToolCallingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.two-tools",
            "model",
            "two-tool-calling-execution-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.tool_call".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.two-tools",
            "two tool calls requested",
        )
        .with_tool_call(
            ToolCall::new(
                "tool-call.partial.1",
                "tool.partial.first",
                r#"{"value":"first"}"#,
            )
            .with_provider("provider.tool.partial"),
        )
        .with_tool_call(
            ToolCall::new(
                "tool-call.partial.2",
                "tool.partial.second",
                r#"{"value":"second"}"#,
            )
            .with_provider("provider.tool.partial"),
        ))
    }
}

#[derive(Clone)]
struct UnknownToolCallingModelProvider;

impl ModelProvider for UnknownToolCallingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.model.unknown-tool",
            "model",
            "unknown-tool-calling-execution-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.tool_call".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            "provider.model.unknown-tool",
            "unknown tool call requested",
        )
        .with_tool_call(
            ToolCall::new(
                "tool-call.unknown.1",
                "tool.execution.unknown",
                r#"{"value":"unknown"}"#,
            )
            .with_provider("provider.tool.execution"),
        ))
    }
}

#[derive(Clone)]
struct StaticAllowPolicyProvider;

impl PolicyProvider for StaticAllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.execution",
            "policy",
            "static-allow-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            "provider.policy.execution",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct StaticNeedsApprovalPolicyProvider;

impl PolicyProvider for StaticNeedsApprovalPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.execution",
            "policy",
            "static-needs-approval-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        let decision_id = format!("decision.{}", request.policy_request_id);
        if request.category == PolicyCategory::ToolInvoke.as_str() {
            return Ok(PolicyDecision::needs_approval(
                decision_id,
                request.policy_request_id,
                "provider.policy.execution",
                "approval.required",
            )
            .with_safe_reason("approval required"));
        }

        Ok(PolicyDecision::allow(
            decision_id,
            request.policy_request_id,
            "provider.policy.execution",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct RecordingAllowPolicyProvider {
    captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
}

impl RecordingAllowPolicyProvider {
    fn new(captured_requests: Arc<Mutex<Vec<PolicyRequest>>>) -> Self {
        Self { captured_requests }
    }
}

impl PolicyProvider for RecordingAllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.recording",
            "policy",
            "recording-allow-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            "provider.policy.recording",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct ModelInvokeDenyPolicyProvider;

impl PolicyProvider for ModelInvokeDenyPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.model-deny",
            "policy",
            "model-invoke-deny-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        let decision_id = format!("decision.{}", request.policy_request_id);
        if request.category == PolicyCategory::ModelInvoke.as_str() {
            return Ok(PolicyDecision::deny(
                decision_id,
                request.policy_request_id,
                "provider.policy.model-deny",
                "model.denied",
            ));
        }

        Ok(PolicyDecision::allow(
            decision_id,
            request.policy_request_id,
            "provider.policy.model-deny",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct ModelInvokeNeedsApprovalPolicyProvider;

impl PolicyProvider for ModelInvokeNeedsApprovalPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.model-approval",
            "policy",
            "model-invoke-approval-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        let decision_id = format!("decision.{}", request.policy_request_id);
        if request.category == PolicyCategory::ModelInvoke.as_str() {
            return Ok(PolicyDecision::needs_approval(
                decision_id,
                request.policy_request_id,
                "provider.policy.model-approval",
                "model.approval_required",
            )
            .with_safe_reason("model approval required"));
        }

        Ok(PolicyDecision::allow(
            decision_id,
            request.policy_request_id,
            "provider.policy.model-approval",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct ExecutionPlanningProvider;

impl PlanningProvider for ExecutionPlanningProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.planning.execution",
            "planning",
            "execution-planning-provider",
            "0.1.0",
            vec!["planning.create".to_string()],
        )
    }

    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> KernelResult<Plan> {
        Ok(
            Plan::new("plan.execution", task_id, run_id, summary).add_action(
                Action::new(
                    "action.execution.model",
                    ActionKind::ModelCall,
                    "invoke selected model provider",
                )
                .with_required_capabilities(vec!["model.chat".to_string()])
                .with_side_effect_level(SideEffectLevel::ReadOnly),
            ),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct RecordingToolProvider;

impl ToolProvider for RecordingToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.execution",
            "tool",
            "recording-execution-tool",
            "0.1.0",
            vec!["tool.invoke".to_string(), "tool.discovery".to_string()],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.execution.echo",
            "provider.tool.execution",
            "Execution Echo",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, "tool output"))
    }
}

#[derive(Clone)]
struct CapturingToolProvider {
    captured_tool_calls: Arc<Mutex<Vec<ToolCall>>>,
}

impl CapturingToolProvider {
    fn new(captured_tool_calls: Arc<Mutex<Vec<ToolCall>>>) -> Self {
        Self {
            captured_tool_calls,
        }
    }
}

impl ToolProvider for CapturingToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.execution",
            "tool",
            "capturing-execution-tool",
            "0.1.0",
            vec!["tool.invoke".to_string(), "tool.discovery".to_string()],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.execution.echo",
            "provider.tool.execution",
            "Execution Echo",
            SideEffectLevel::ReadOnly,
        )]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        self.captured_tool_calls.lock().unwrap().push(call.clone());
        Ok(ToolResult::succeeded(call.tool_call_id, "tool output"))
    }
}

#[derive(Clone)]
struct PartiallyFailingToolProvider;

impl ToolProvider for PartiallyFailingToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.tool.partial",
            "tool",
            "partially-failing-execution-tool",
            "0.1.0",
            vec!["tool.invoke".to_string(), "tool.discovery".to_string()],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "tool.partial.first",
                "provider.tool.partial",
                "Partial First",
                SideEffectLevel::ReadOnly,
            ),
            ToolDescriptor::new(
                "tool.partial.second",
                "provider.tool.partial",
                "Partial Second",
                SideEffectLevel::ReadOnly,
            ),
        ]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        if call.tool_id == "tool.partial.second" {
            return Ok(ToolResult::failed(call.tool_call_id, "second tool failed")
                .with_status(ToolCallStatus::Failed));
        }

        Ok(ToolResult::succeeded(
            call.tool_call_id,
            "first tool output",
        ))
    }
}

#[derive(Clone)]
struct RecordingMcpProvider;

impl McpProvider for RecordingMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.mcp.execution",
            "mcp",
            "recording-execution-mcp",
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
            "mcp.tool.execution.echo",
            "provider.mcp.execution",
            "Execution MCP Echo",
            SideEffectLevel::ReadOnly,
        )])
    }

    fn invoke_tool(&self, _server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, "mcp tool output"))
    }
}

#[derive(Clone)]
struct CapturingMcpProvider {
    captured_tool_calls: Arc<Mutex<Vec<ToolCall>>>,
}

impl CapturingMcpProvider {
    fn new(captured_tool_calls: Arc<Mutex<Vec<ToolCall>>>) -> Self {
        Self {
            captured_tool_calls,
        }
    }
}

impl McpProvider for CapturingMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.mcp.execution",
            "mcp",
            "capturing-execution-mcp",
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
            "mcp.tool.execution.echo",
            "provider.mcp.execution",
            "Execution MCP Echo",
            SideEffectLevel::ReadOnly,
        )])
    }

    fn invoke_tool(&self, _server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        self.captured_tool_calls.lock().unwrap().push(call.clone());
        Ok(ToolResult::succeeded(call.tool_call_id, "mcp tool output"))
    }
}

#[derive(Clone)]
struct CountingMcpProvider {
    captured_list_calls: Arc<Mutex<usize>>,
}

impl CountingMcpProvider {
    fn new(captured_list_calls: Arc<Mutex<usize>>) -> Self {
        Self {
            captured_list_calls,
        }
    }
}

impl McpProvider for CountingMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.mcp.execution",
            "mcp",
            "counting-execution-mcp",
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
        *self.captured_list_calls.lock().unwrap() += 1;
        Ok(vec![ToolDescriptor::new(
            "tool.execution.echo",
            "provider.mcp.execution",
            "Execution MCP Echo",
            SideEffectLevel::ReadOnly,
        )])
    }

    fn invoke_tool(&self, _server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(ToolResult::succeeded(call.tool_call_id, "mcp tool output"))
    }
}

#[derive(Clone)]
struct RecordingMemoryProvider;

impl MemoryProvider for RecordingMemoryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.memory.execution",
            "memory",
            "recording-execution-memory",
            "0.1.0",
            vec!["memory.query".to_string()],
        )
    }

    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        Ok(vec![MemoryRecord::new(
            "memory.execution.1",
            scope,
            owner_context,
            "memory context",
            TrustLevel::TrustedHost,
            RedactionClassification::Internal,
        )])
    }

    fn write(&mut self, _record: MemoryRecord) -> KernelResult<()> {
        Ok(())
    }

    fn delete(&mut self, _memory_record_id: &str) -> KernelResult<()> {
        Ok(())
    }

    fn export(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        self.query(scope, owner_context)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct RecordingKnowledgeProvider;

impl KnowledgeProvider for RecordingKnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.knowledge.execution",
            "knowledge",
            "recording-execution-knowledge",
            "0.1.0",
            vec!["knowledge.search".to_string()],
        )
    }

    fn search(&self, _request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        Ok(vec![KnowledgeSearchResult::new(
            "knowledge.execution.1",
            KnowledgeDocumentKind::Spec,
            "Kernel Knowledge",
            KnowledgeRetrievalMethod::Keyword,
        )
        .with_snippet("knowledge context")
        .with_trust_level(TrustLevel::RetrievedExternal)
        .with_redaction_classification(
            RedactionClassification::Internal,
        )])
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            "Kernel Knowledge",
            "knowledge context",
        ))
    }

    fn list(&self, _filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(Vec::new())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct CapturingKnowledgeProvider {
    captured_requests: Arc<Mutex<Vec<KnowledgeSearchRequest>>>,
}

impl CapturingKnowledgeProvider {
    fn new(captured_requests: Arc<Mutex<Vec<KnowledgeSearchRequest>>>) -> Self {
        Self { captured_requests }
    }
}

impl KnowledgeProvider for CapturingKnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.knowledge.capturing",
            "knowledge",
            "capturing-execution-knowledge",
            "0.1.0",
            vec!["knowledge.search".to_string()],
        )
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(vec![KnowledgeSearchResult::new(
            "knowledge.execution.full",
            KnowledgeDocumentKind::Spec,
            "Kernel Full Context",
            KnowledgeRetrievalMethod::Hybrid,
        )
        .with_snippet("full knowledge context")
        .with_trust_level(TrustLevel::RetrievedExternal)
        .with_redaction_classification(
            RedactionClassification::Internal,
        )])
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            "Kernel Full Context",
            "full knowledge context",
        ))
    }

    fn list(&self, _filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(Vec::new())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
