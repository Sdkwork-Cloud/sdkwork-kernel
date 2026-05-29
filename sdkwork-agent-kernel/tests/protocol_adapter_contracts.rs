use sdkwork_agent_kernel::{
    AgentTask, KernelError, KernelEvent, KernelEventSeverity, KernelResult, ProtocolAdapter,
    ProtocolAdapterAuthMode, ProtocolAdapterManifest, ProtocolAdapterRequest,
    ProtocolAdapterResponse, ProtocolAdapterStreamingSupport, ProtocolError, ProtocolFamily,
    ProtocolStreamUpdate, ProtocolTransport, ProviderHealth, TraceContext,
};

#[test]
fn protocol_adapter_manifest_declares_protocol_transport_auth_mapping_and_capabilities() {
    let manifest = ProtocolAdapterManifest::new(
        "adapter.mcp.local",
        ProtocolFamily::Mcp,
        "2026-03-26",
        ProtocolTransport::Ipc,
        ProtocolAdapterAuthMode::LocalTrusted,
    )
    .with_exposed_capabilities(vec!["tool.invoke".to_string(), "context.read".to_string()])
    .with_kernel_object_mappings(vec![
        "AgentTask".to_string(),
        "ToolDescriptor".to_string(),
        "KernelEvent".to_string(),
    ])
    .with_streaming_support(ProtocolAdapterStreamingSupport::Ordered)
    .with_trace_support(true)
    .with_security_requirements(vec!["local-only".to_string()]);

    manifest.validate().expect("adapter manifest validates");
    assert_eq!(manifest.provider_family, "protocol_adapter");
    assert_eq!(manifest.adapter_id, "adapter.mcp.local");
    assert_eq!(manifest.protocol, ProtocolFamily::Mcp);
    assert!(manifest.trace_support);
    assert!(manifest.exposes_capability("tool.invoke"));
    assert!(!manifest.exposes_capability("memory.write"));
}

#[test]
fn protocol_adapter_manifest_rejects_invalid_adapter_ids() {
    let manifest = ProtocolAdapterManifest::new(
        "mcp.local",
        ProtocolFamily::Mcp,
        "2026-03-26",
        ProtocolTransport::Ipc,
        ProtocolAdapterAuthMode::LocalTrusted,
    );

    let error = manifest.validate().expect_err("invalid adapter id fails");

    assert!(error.to_string().contains("adapter."));
}

#[test]
fn protocol_adapter_request_preserves_namespaced_metadata_and_trace_context() {
    let request = ProtocolAdapterRequest::new(
        "protocol-request.1",
        ProtocolFamily::A2a,
        "task.create",
        "write tests",
    )
    .with_external_id("a2a-task.1")
    .with_metadata("a2a.priority", "high")
    .with_trace_context(TraceContext {
        trace_id: "trace.1".to_string(),
        span_id: "span.1".to_string(),
        parent_span_id: Some("parent.1".to_string()),
    });

    assert_eq!(request.external_id.as_deref(), Some("a2a-task.1"));
    assert_eq!(request.metadata_value("a2a.priority"), Some("high"));
    assert_eq!(
        request
            .trace_context
            .as_ref()
            .unwrap()
            .parent_span_id
            .as_deref(),
        Some("parent.1")
    );
}

#[test]
fn protocol_adapter_trait_maps_external_requests_and_kernel_events() {
    let adapter = FakeProtocolAdapter;
    let request = ProtocolAdapterRequest::new(
        "protocol-request.1",
        ProtocolFamily::KernelUiClient,
        "task.create",
        "Summarize the repository",
    )
    .with_external_id("ui-task.1")
    .with_trace_context(TraceContext {
        trace_id: "trace.1".to_string(),
        span_id: "span.1".to_string(),
        parent_span_id: None,
    });

    let task = adapter
        .map_request_to_task(request)
        .expect("request maps to kernel task");

    assert_eq!(task.task_id, "task.protocol-request.1");
    assert_eq!(task.instruction, "Summarize the repository");

    let event = KernelEvent::new(
        "event.1",
        "agent.task.created",
        KernelEventSeverity::Info,
        "task_id=task.protocol-request.1",
    )
    .with_trace("trace.1", "span.2");

    let update = adapter
        .map_event_to_stream_update(event)
        .expect("event maps to stream update");

    assert_eq!(update.event_id, "event.1");
    assert_eq!(update.sequence, 1);
    assert_eq!(update.trace_context.as_ref().unwrap().trace_id, "trace.1");
}

#[test]
fn protocol_adapter_maps_kernel_errors_without_leaking_internal_details() {
    let validation = ProtocolError::from_kernel_error(KernelError::validation("bad payload"));
    let denied = ProtocolError::from_kernel_error(KernelError::PolicyDenied {
        reason_code: "policy.denied.secret".to_string(),
    });
    let internal = ProtocolError::from_kernel_error(KernelError::Internal {
        message: "database password was secret".to_string(),
    });

    assert_eq!(validation.code, "validation_error");
    assert_eq!(denied.code, "permission_denied");
    assert_eq!(denied.safe_message, "request denied by policy");
    assert_eq!(internal.code, "internal_error");
    assert!(!internal.safe_message.contains("database password"));
}

struct FakeProtocolAdapter;

impl ProtocolAdapter for FakeProtocolAdapter {
    fn manifest(&self) -> ProtocolAdapterManifest {
        ProtocolAdapterManifest::new(
            "adapter.kernel-ui.fake",
            ProtocolFamily::KernelUiClient,
            "0.1.0",
            ProtocolTransport::WebSocket,
            ProtocolAdapterAuthMode::Bearer,
        )
        .with_exposed_capabilities(vec!["agent.task.create".to_string()])
        .with_kernel_object_mappings(vec!["AgentTask".to_string(), "KernelEvent".to_string()])
        .with_streaming_support(ProtocolAdapterStreamingSupport::Ordered)
        .with_trace_support(true)
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn map_request_to_task(&self, request: ProtocolAdapterRequest) -> KernelResult<AgentTask> {
        Ok(AgentTask::new(
            format!("task.{}", request.protocol_request_id),
            request
                .external_id
                .unwrap_or_else(|| "session.protocol".to_string()),
            request.payload,
        ))
    }

    fn map_event_to_stream_update(&self, event: KernelEvent) -> KernelResult<ProtocolStreamUpdate> {
        Ok(ProtocolStreamUpdate::from_event(event, 1))
    }

    fn map_response(&self, task: AgentTask) -> KernelResult<ProtocolAdapterResponse> {
        Ok(ProtocolAdapterResponse::accepted(
            "protocol-response.1",
            task.task_id,
        ))
    }
}
