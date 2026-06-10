use sdkwork_agent_kernel::{
    agent_chat_rpc_adapter_manifest, AgentChatRequest, AgentChatRpcAdapter, AgentChatRpcHandler,
    AgentChatService, AgentManifest, EventStream, EventStreamCursor, EventStreamFilter,
    KernelErrorKind, KernelEventRedaction, KernelResult, KnowledgeDocument,
    KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider, KnowledgeRetrievalMethod,
    KnowledgeSearchRequest, KnowledgeSearchResult, MemoryProvider, MemoryRecord, MemoryScope,
    ModelProvider, ModelRequest, ModelResponse, PolicyCategory, PolicyDecision, PolicyProvider,
    PolicyRequest, PolicySubject, ProtocolAdapter, ProtocolAdapterAuthMode, ProtocolAdapterRequest,
    ProtocolAdapterStreamingSupport, ProtocolFamily, ProtocolObjectKind, ProtocolTransport,
    ProviderHealth, ProviderManifest, RedactionClassification, RuntimeBuilder, SideEffectLevel,
    ToolCall, ToolDescriptor, ToolProvider, ToolResult, TraceContext, TrustLevel,
};
use std::sync::{Arc, Mutex};

const CHAT_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.chat-service",
  "name": "sdkwork-chat-service-agent",
  "display_name": "SDKWork Chat Service Agent",
  "description": "Agent used to prove transport-neutral chat service contracts.",
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
  "event_families": ["agent.runtime.*", "agent.model.*", "agent.policy.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const CHAT_RPC_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.chat-rpc",
  "name": "sdkwork-chat-rpc-agent",
  "display_name": "SDKWork Chat RPC Agent",
  "description": "Agent used to prove typed chat RPC protocol adapter registration.",
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
    },
    {
      "capability_id": "protocol.map",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.model.*", "agent.policy.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const CHAT_OPTIONAL_KNOWLEDGE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.chat-optional-knowledge",
  "name": "sdkwork-chat-optional-knowledge-agent",
  "display_name": "SDKWork Chat Optional Knowledge Agent",
  "description": "Agent used to prove knowledge providers are optional unless selected or required.",
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
  "optional_capabilities": [
    {
      "capability_id": "knowledge.search",
      "min_version": "0.1.0"
    }
  ],
  "event_families": ["agent.runtime.*", "agent.model.*", "agent.policy.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

const CHAT_REQUIRED_KNOWLEDGE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.chat-required-knowledge",
  "name": "sdkwork-chat-required-knowledge-agent",
  "display_name": "SDKWork Chat Required Knowledge Agent",
  "description": "Agent used to prove required knowledge capabilities fail closed.",
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
    },
    {
      "capability_id": "knowledge.search",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.model.*", "agent.policy.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn chat_service_evaluates_policy_and_invokes_selected_model_provider() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
    );

    let trace = TraceContext::new("trace.chat.1", "span.chat.1").with_parent_span("span.parent");
    let subject = PolicySubject::new("user.1", "tenant.1").with_role("developer");
    let request = AgentChatRequest::new(
        "chat-request.1",
        vec!["Summarize the Rig plugin status".to_string()],
    )
    .with_provider_id("provider.model.chat.primary")
    .with_model_id("model.chat.fast")
    .for_session("session.1")
    .for_task("task.1")
    .for_run("run.1")
    .for_step("step.1")
    .with_subject(subject.clone())
    .with_trace_context(trace.clone())
    .with_timeout_ms(30_000)
    .with_metadata("sdkwork.chat.source", "rpc-test");

    let response = AgentChatService::new()
        .invoke(&runtime, request)
        .expect("chat service invokes selected model provider");

    assert_eq!(response.chat_request_id, "chat-request.1");
    assert_eq!(response.provider_id, "provider.model.chat.primary");
    assert_eq!(response.policy_decision.request_id, "policy.chat-request.1");
    assert_eq!(
        response.model_response.messages,
        ["primary: model.chat.fast"]
    );
    assert_eq!(response.model_response.trace_context.as_ref(), Some(&trace));

    let policy_requests = captured_policy_requests.lock().unwrap();
    assert_eq!(policy_requests.len(), 1);
    let policy_request = &policy_requests[0];
    assert_eq!(policy_request.policy_request_id, "policy.chat-request.1");
    assert_eq!(
        policy_request.typed_category,
        Some(PolicyCategory::ModelInvoke)
    );
    assert_eq!(policy_request.action.as_deref(), Some("model.invoke"));
    assert_eq!(policy_request.resource, "model.chat.fast");
    assert_eq!(policy_request.subject.as_ref(), Some(&subject));
    assert_eq!(
        policy_request.side_effect_level,
        Some(SideEffectLevel::ExternalSend)
    );
    assert_eq!(
        policy_request.context_value("provider_id"),
        Some("provider.model.chat.primary")
    );
    assert_eq!(policy_request.context_value("step_id"), Some("step.1"));

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    let model_request = &model_requests[0];
    assert_eq!(model_request.model_request_id, "chat-request.1");
    assert_eq!(model_request.model_id.as_deref(), Some("model.chat.fast"));
    assert_eq!(model_request.session_id.as_deref(), Some("session.1"));
    assert_eq!(model_request.task_id.as_deref(), Some("task.1"));
    assert_eq!(model_request.run_id.as_deref(), Some("run.1"));
    assert_eq!(model_request.step_id.as_deref(), Some("step.1"));
    assert_eq!(model_request.messages, ["Summarize the Rig plugin status"]);
    assert_eq!(
        model_request.policy_request_id.as_deref(),
        Some("policy.chat-request.1")
    );
    assert_eq!(model_request.trace_context.as_ref(), Some(&trace));
    assert_eq!(model_request.timeout_ms, Some(30_000));
    assert_eq!(
        model_request.metadata_value("sdkwork.chat.source"),
        Some("rpc-test")
    );
}

#[test]
fn chat_service_runs_without_knowledge_provider_when_knowledge_is_not_requested() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
    );

    assert!(runtime.knowledge_provider_ids().is_empty());

    AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.no-knowledge",
                vec!["answer without retrieval".to_string()],
            ),
        )
        .expect("chat runs when no knowledge capability is selected");

    assert_eq!(captured_policy_requests.lock().unwrap().len(), 1);
    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert!(model_requests[0].context_frames.is_empty());
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.knowledge.query"),
        None
    );
}

#[test]
fn optional_knowledge_capability_degrades_runtime_but_does_not_block_plain_chat() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let manifest = AgentManifest::from_json(CHAT_OPTIONAL_KNOWLEDGE_AGENT_MANIFEST_JSON)
        .expect("optional knowledge manifest parses");
    let report = RuntimeBuilder::new("runtime.chat.optional-knowledge", manifest)
        .with_generated_at("2026-06-10T00:00:00Z")
        .register_model_provider(
            "provider.model.chat.primary",
            "0.1.0",
            RecordingModelProvider::new(
                "provider.model.chat.primary",
                "primary",
                captured_model_requests.clone(),
            ),
        )
        .register_policy_provider(
            "provider.policy.chat",
            "0.1.0",
            RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
        )
        .bootstrap()
        .expect("runtime bootstraps with optional knowledge degraded");

    assert_eq!(
        report.runtime.state(),
        sdkwork_agent_kernel::RuntimeState::Degraded
    );
    assert_eq!(
        report.runtime.capability_manifest().degraded_capabilities,
        ["knowledge.search"]
    );

    AgentChatService::new()
        .invoke(
            &report.runtime,
            AgentChatRequest::new(
                "chat-request.optional-knowledge",
                vec!["answer without optional retrieval".to_string()],
            ),
        )
        .expect("optional missing knowledge does not block plain chat");

    assert_eq!(captured_model_requests.lock().unwrap().len(), 1);
}

#[test]
fn required_knowledge_capability_missing_fails_closed_before_chat_execution() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let manifest = AgentManifest::from_json(CHAT_REQUIRED_KNOWLEDGE_AGENT_MANIFEST_JSON)
        .expect("required knowledge manifest parses");
    let report = RuntimeBuilder::new("runtime.chat.required-knowledge", manifest)
        .with_generated_at("2026-06-10T00:00:00Z")
        .register_model_provider(
            "provider.model.chat.primary",
            "0.1.0",
            RecordingModelProvider::new(
                "provider.model.chat.primary",
                "primary",
                captured_model_requests.clone(),
            ),
        )
        .register_policy_provider(
            "provider.policy.chat",
            "0.1.0",
            RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
        )
        .bootstrap()
        .expect("runtime bootstraps into failed state when required knowledge is missing");

    assert_eq!(
        report.runtime.state(),
        sdkwork_agent_kernel::RuntimeState::Failed
    );
    assert_eq!(
        report
            .runtime
            .capability_manifest()
            .missing_required_capabilities,
        ["knowledge.search"]
    );

    let error = AgentChatService::new()
        .invoke(
            &report.runtime,
            AgentChatRequest::new(
                "chat-request.required-knowledge",
                vec!["this must not invoke the model".to_string()],
            ),
        )
        .expect_err("failed runtime must stop chat execution");

    assert_eq!(error.kind(), KernelErrorKind::CapabilityMissing);
    assert_eq!(error.message(), "knowledge.search");
    assert!(captured_model_requests.lock().unwrap().is_empty());
}

#[test]
fn chat_service_fails_closed_when_policy_denies_model_invoke() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::deny(
            "provider.policy.chat",
            captured_policy_requests.clone(),
            "model.invoke.denied",
        ),
    );

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new("chat-request.denied", vec!["hello".to_string()])
                .with_model_id("model.chat.fast"),
        )
        .expect_err("policy denial must stop model invocation");

    assert_eq!(error.kind(), KernelErrorKind::PolicyDenied);
    assert_eq!(error.code(), "policy_denied");
    assert_eq!(
        captured_policy_requests.lock().unwrap()[0].resource,
        "model.chat.fast"
    );
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "model provider must not be called after policy denial"
    );
}

#[test]
fn chat_service_maps_policy_approval_requirement_to_permission_required() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::needs_approval(
            "provider.policy.chat",
            captured_policy_requests,
            "model.invoke.requires_approval",
        ),
    );

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new("chat-request.approval", vec!["hello".to_string()])
                .with_model_id("model.chat.frontier"),
        )
        .expect_err("approval requirement must stop model invocation");

    assert_eq!(error.kind(), KernelErrorKind::PermissionRequired);
    assert_eq!(error.code(), "permission_required");
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "model provider must not be called before approval"
    );
}

#[test]
fn chat_service_rejects_blank_messages_before_policy_and_model_invocation() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
    );

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new("chat-request.blank", vec!["  \t ".to_string()]),
        )
        .expect_err("blank chat messages must be rejected before policy and model invocation");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_policy_requests.lock().unwrap().is_empty(),
        "invalid chat requests must not reach policy evaluation"
    );
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid chat requests must not invoke the model provider"
    );
}

#[test]
fn chat_service_rejects_incomplete_policy_subject_before_policy_and_model_invocation() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
    );

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new("chat-request.blank-subject", vec!["hello".to_string()])
                .with_subject(PolicySubject::new(" ", "tenant.1")),
        )
        .expect_err("blank policy subject fields must be rejected");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_policy_requests.lock().unwrap().is_empty(),
        "invalid policy subject must not reach policy evaluation"
    );
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid policy subject must not invoke the model provider"
    );
}

#[test]
fn chat_service_attaches_requested_tool_descriptors_without_invoking_tools() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_tool_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime_with_tool_provider(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
        RecordingToolProvider::new("provider.tool.chat", captured_tool_calls.clone()),
    );

    AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.tools",
                vec!["use a tool if needed".to_string()],
            )
            .include_tool_descriptors(),
        )
        .expect("chat service attaches requested tool descriptors");

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(model_requests[0].tool_descriptors.len(), 1);
    assert_eq!(
        model_requests[0].tool_descriptors[0].tool_id,
        "tool.chat.search"
    );
    assert_eq!(
        model_requests[0].tool_descriptors[0].provider_id,
        "provider.tool.chat"
    );
    assert!(
        captured_tool_calls.lock().unwrap().is_empty(),
        "chat must expose tool descriptors without implicitly invoking side-effectful tools"
    );
    assert_eq!(
        captured_policy_requests.lock().unwrap().len(),
        1,
        "tool descriptor exposure is not a tool invocation policy decision"
    );
}

#[test]
fn chat_service_queries_requested_memory_and_attaches_context_frames() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_memory_queries = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime_with_memory_provider(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
        RecordingMemoryProvider::new(captured_memory_queries.clone()).with_record(
            MemoryRecord::new(
                "memory.chat.session.1",
                MemoryScope::Session,
                "session.memory",
                "remember that the user prefers concise answers",
                TrustLevel::AgentMessage,
                RedactionClassification::Internal,
            )
            .with_source("memory.session")
            .with_policy_decision("decision.memory.write.1"),
        ),
    );

    AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.memory",
                vec!["what should I remember?".to_string()],
            )
            .for_session("session.memory")
            .for_task("task.memory")
            .with_memory_query(MemoryScope::Session, "session.memory"),
        )
        .expect("chat service attaches requested memory context");

    assert_eq!(
        captured_memory_queries.lock().unwrap().as_slice(),
        &[(MemoryScope::Session, "session.memory".to_string())]
    );

    let policy_requests = captured_policy_requests.lock().unwrap();
    assert_eq!(policy_requests.len(), 2);
    assert!(policy_requests
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::ModelInvoke)));
    let memory_policy = policy_requests
        .iter()
        .find(|request| request.typed_category == Some(PolicyCategory::MemoryRead))
        .expect("memory reads are policy-gated before querying provider");
    assert_eq!(memory_policy.action.as_deref(), Some("memory.query"));
    assert_eq!(memory_policy.resource, "session:session.memory");
    assert_eq!(memory_policy.session_id.as_deref(), Some("session.memory"));
    assert_eq!(memory_policy.task_id.as_deref(), Some("task.memory"));

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(
        model_requests[0].context_frame_ids,
        ["context.memory.memory.chat.session.1"]
    );
    assert_eq!(model_requests[0].context_frames.len(), 1);
    let frame = &model_requests[0].context_frames[0];
    assert_eq!(
        frame.context_frame_id,
        "context.memory.memory.chat.session.1"
    );
    assert_eq!(frame.session_id, "session.memory");
    assert_eq!(frame.task_id.as_deref(), Some("task.memory"));
    assert_eq!(
        frame.content,
        "remember that the user prefers concise answers"
    );
    assert_eq!(frame.trust_level, TrustLevel::AgentMessage);
    assert_eq!(
        frame.redaction_classification,
        RedactionClassification::Internal
    );
    assert_eq!(
        frame.metadata_value("sdkwork.memory.record_id"),
        Some("memory.chat.session.1")
    );
}

#[test]
fn chat_service_queries_requested_knowledge_and_attaches_rag_context_frames() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_knowledge_queries = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime_with_knowledge_provider(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
        RecordingKnowledgeProvider::new(captured_knowledge_queries.clone()).with_result(
            KnowledgeSearchResult::new(
                "knowledge.rag.chunk.1",
                KnowledgeDocumentKind::WikiSection,
                "Knowledgebase RAG",
                KnowledgeRetrievalMethod::Hybrid,
            )
            .with_snippet("Knowledgebase retrieval returns bounded context and citations.")
            .with_score(0.91)
            .with_source_uri("kb://space/7/document/11#chunk-1")
            .with_trust_level(TrustLevel::RetrievedExternal)
            .with_redaction_classification(RedactionClassification::Internal)
            .with_metadata("sdkwork.knowledge.citation", "chunk-1"),
        ),
    );
    let subject = PolicySubject::new("user.knowledge", "tenant.knowledge").with_role("developer");
    let trace = TraceContext::new("trace.knowledge.chat", "span.knowledge.chat");

    AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.knowledge",
                vec!["answer from the linked knowledge base".to_string()],
            )
            .for_session("session.knowledge")
            .for_task("task.knowledge")
            .with_subject(subject.clone())
            .with_trace_context(trace.clone())
            .with_timeout_ms(20_000)
            .with_knowledge_query("sdkwork knowledgebase rag")
            .with_knowledge_provider_id("provider.knowledge.chat")
            .with_knowledge_tenant_id("tenant.knowledge")
            .with_knowledge_namespace("sdkwork.knowledgebase")
            .with_knowledge_top_k(3)
            .with_knowledge_method(KnowledgeRetrievalMethod::Hybrid)
            .with_knowledge_filter("space_id", "7"),
        )
        .expect("chat service attaches requested knowledge context");

    let knowledge_queries = captured_knowledge_queries.lock().unwrap();
    assert_eq!(knowledge_queries.len(), 1);
    let knowledge_query = &knowledge_queries[0];
    assert_eq!(knowledge_query.query, "sdkwork knowledgebase rag");
    assert_eq!(
        knowledge_query.namespace.as_deref(),
        Some("sdkwork.knowledgebase")
    );
    assert_eq!(
        knowledge_query.tenant_id.as_deref(),
        Some("tenant.knowledge")
    );
    assert_eq!(
        knowledge_query.session_id.as_deref(),
        Some("session.knowledge")
    );
    assert_eq!(knowledge_query.task_id.as_deref(), Some("task.knowledge"));
    assert_eq!(knowledge_query.top_k, 3);
    assert!(knowledge_query.supports_method(KnowledgeRetrievalMethod::Hybrid));
    assert_eq!(
        knowledge_query.policy_decision_id.as_deref(),
        Some("decision.policy.chat-request.knowledge.knowledge.search")
    );
    assert_eq!(knowledge_query.trace_context.as_ref(), Some(&trace));
    assert_eq!(knowledge_query.timeout_ms, Some(20_000));
    assert_eq!(knowledge_query.metadata_value("space_id"), Some("7"));

    let policy_requests = captured_policy_requests.lock().unwrap();
    assert_eq!(policy_requests.len(), 2);
    assert!(policy_requests
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::ModelInvoke)));
    let knowledge_policy = policy_requests
        .iter()
        .find(|request| request.typed_category == Some(PolicyCategory::KnowledgeSearch))
        .expect("knowledge searches are policy-gated before querying provider");
    assert_eq!(knowledge_policy.action.as_deref(), Some("knowledge.search"));
    assert_eq!(
        knowledge_policy.resource,
        "sdkwork.knowledgebase:sdkwork knowledgebase rag"
    );
    assert_eq!(knowledge_policy.subject.as_ref(), Some(&subject));
    assert_eq!(
        knowledge_policy.side_effect_level,
        Some(SideEffectLevel::ReadOnly)
    );

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(
        model_requests[0].context_frame_ids,
        ["context.knowledge.knowledge.rag.chunk.1"]
    );
    assert_eq!(model_requests[0].context_frames.len(), 1);
    let frame = &model_requests[0].context_frames[0];
    assert_eq!(frame.session_id, "session.knowledge");
    assert_eq!(frame.task_id.as_deref(), Some("task.knowledge"));
    assert_eq!(
        frame.content,
        "Knowledgebase retrieval returns bounded context and citations."
    );
    assert_eq!(
        frame.provenance.as_deref(),
        Some("kb://space/7/document/11#chunk-1")
    );
    assert_eq!(
        frame.metadata_value("sdkwork.knowledge.document_id"),
        Some("knowledge.rag.chunk.1")
    );
    assert_eq!(
        frame.metadata_value("sdkwork.knowledge.retrieval_method"),
        Some("hybrid")
    );
    assert_eq!(
        frame.metadata_value("sdkwork.knowledge.citation"),
        Some("chunk-1")
    );
}

#[test]
fn chat_service_fails_closed_when_knowledge_policy_denies_before_search_and_model() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_knowledge_queries = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime_with_knowledge_provider(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::deny_knowledge_search(
            "provider.policy.chat",
            captured_policy_requests.clone(),
            "knowledge.search.denied",
        ),
        RecordingKnowledgeProvider::new(captured_knowledge_queries.clone()),
    );

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.knowledge-denied",
                vec!["search before answer".to_string()],
            )
            .with_knowledge_query("blocked knowledge query"),
        )
        .expect_err("knowledge search policy denial must stop retrieval and model invocation");

    assert_eq!(error.kind(), KernelErrorKind::PolicyDenied);
    assert!(captured_knowledge_queries.lock().unwrap().is_empty());
    assert!(captured_model_requests.lock().unwrap().is_empty());
    assert!(captured_policy_requests
        .lock()
        .unwrap()
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::KnowledgeSearch)));
}

#[test]
fn chat_service_rejects_zero_knowledge_top_k_before_policy_model_and_search() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_knowledge_queries = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime_with_knowledge_provider(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
        RecordingKnowledgeProvider::new(captured_knowledge_queries.clone()),
    );

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.zero-knowledge-top-k",
                vec!["search before answer".to_string()],
            )
            .with_knowledge_query("knowledge query")
            .with_knowledge_top_k(0),
        )
        .expect_err("zero knowledge top_k must be rejected before policy and providers");

    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_policy_requests.lock().unwrap().is_empty(),
        "invalid knowledge top_k must not reach policy evaluation"
    );
    assert!(captured_knowledge_queries.lock().unwrap().is_empty());
    assert!(captured_model_requests.lock().unwrap().is_empty());
}

#[test]
fn chat_service_preserves_model_tool_calls_without_auto_execution() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_tool_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime_with_tool_provider(
        ToolCallingModelProvider::new(
            "provider.model.chat.primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
        RecordingToolProvider::new("provider.tool.chat", captured_tool_calls.clone()),
    );

    let response = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat-request.tool-call",
                vec!["call the search tool".to_string()],
            )
            .for_session("session.tool-call")
            .for_task("task.tool-call")
            .include_tool_descriptors(),
        )
        .expect("chat service preserves model tool-call output");

    assert_eq!(response.model_response.tool_calls.len(), 1);
    assert_eq!(
        response.model_response.tool_calls[0].tool_call_id,
        "tool-call.from-model.1"
    );
    assert_eq!(
        response.model_response.tool_calls[0].tool_id,
        "tool.chat.search"
    );
    assert_eq!(
        response.model_response.tool_calls[0].provider_id.as_deref(),
        Some("provider.tool.chat")
    );
    assert!(
        captured_tool_calls.lock().unwrap().is_empty(),
        "model tool-call output must be returned to the caller, not executed inside chat"
    );
}

#[test]
fn chat_rpc_adapter_manifest_declares_transport_auth_trace_and_mapping_contract() {
    let manifest = agent_chat_rpc_adapter_manifest();

    manifest
        .validate()
        .expect("chat RPC adapter manifest validates");
    assert_eq!(manifest.adapter_id, "adapter.rpc.agent-chat");
    assert_eq!(manifest.protocol, ProtocolFamily::Rpc);
    assert_eq!(manifest.protocol_version, "sdkwork.agent.rpc.chat.v1");
    assert_eq!(manifest.transport, ProtocolTransport::Rpc);
    assert_eq!(manifest.auth_mode, ProtocolAdapterAuthMode::LocalTrusted);
    assert_eq!(
        manifest.streaming_support,
        ProtocolAdapterStreamingSupport::Ordered
    );
    assert!(manifest.trace_support);
    assert!(manifest.exposes_capability("model.chat"));
    assert!(manifest.exposes_capability("knowledge.search"));
    assert!(manifest.exposes_capability("protocol.map"));
    assert!(manifest.exposes_capability("protocol.stream"));
    assert!(manifest
        .kernel_object_mappings
        .contains(&"AgentChatRequest".to_string()));
    assert!(manifest
        .kernel_object_mappings
        .contains(&"AgentChatKnowledgeQuery".to_string()));
    assert!(manifest
        .kernel_object_mappings
        .contains(&"ModelRequest".to_string()));
    assert!(manifest
        .kernel_object_mappings
        .contains(&"KnowledgeSearchRequest".to_string()));
    assert!(manifest
        .kernel_object_mappings
        .contains(&"KnowledgeSearchResult".to_string()));
    assert!(manifest
        .security_requirements
        .contains(&"policy.evaluate.model.invoke".to_string()));
    assert!(manifest
        .security_requirements
        .contains(&"policy.evaluate.knowledge.search".to_string()));
}

#[test]
fn chat_rpc_adapter_registers_as_typed_protocol_adapter_provider() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let manifest =
        AgentManifest::from_json(CHAT_RPC_AGENT_MANIFEST_JSON).expect("chat RPC manifest parses");
    let report = RuntimeBuilder::new("runtime.chat.rpc.local", manifest)
        .with_generated_at("2026-06-09T00:00:00Z")
        .register_model_provider(
            "provider.model.chat.primary",
            "0.1.0",
            RecordingModelProvider::new(
                "provider.model.chat.primary",
                "primary",
                captured_model_requests.clone(),
            ),
        )
        .register_policy_provider(
            "provider.policy.chat",
            "0.1.0",
            RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
        )
        .register_protocol_adapter(
            "adapter.rpc.agent-chat",
            "0.1.0",
            AgentChatRpcAdapter::new(),
        )
        .bootstrap()
        .expect("chat RPC runtime bootstraps");

    assert_eq!(
        report.runtime.state(),
        sdkwork_agent_kernel::RuntimeState::Ready
    );
    assert_eq!(
        report.runtime.protocol_adapter_ids(),
        ["adapter.rpc.agent-chat"]
    );
    assert!(report
        .runtime
        .capability_manifest()
        .protocol_adapters
        .contains(&"adapter.rpc.agent-chat".to_string()));

    let diagnostic = report
        .runtime
        .diagnostics()
        .provider("adapter.rpc.agent-chat")
        .expect("chat RPC adapter diagnostics are present")
        .clone();
    assert_eq!(diagnostic.provider_family, "protocol_adapter");
    assert!(diagnostic.typed_registered);
    assert!(diagnostic
        .capabilities
        .contains(&"protocol.map".to_string()));
    assert!(diagnostic
        .capabilities
        .contains(&"protocol.stream".to_string()));
    assert!(report
        .runtime
        .capability_manifest()
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "protocol.stream"
            && capability.provider_id == "adapter.rpc.agent-chat"));

    let task = report
        .runtime
        .protocol_adapter_by_id("adapter.rpc.agent-chat")
        .expect("chat RPC protocol adapter is registered by id")
        .map_request_to_task(
            ProtocolAdapterRequest::new(
                "chat-rpc.adapter",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello from RPC",
            )
            .with_metadata("sdkwork.agent.session_id", "session.rpc")
            .with_metadata("sdkwork.agent.task_id", "task.rpc"),
        )
        .expect("chat RPC adapter maps request to SDKWork task");
    assert_eq!(task.task_id, "task.rpc");
    assert_eq!(task.session_id, "session.rpc");
    assert_eq!(task.instruction, "hello from RPC");

    let envelope = report
        .runtime
        .protocol_adapter_by_id("adapter.rpc.agent-chat")
        .expect("chat RPC protocol adapter is registered by id")
        .handle_request(
            &report.runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.runtime-adapter",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello through runtime adapter",
            )
            .with_metadata("sdkwork.chat.provider_id", "provider.model.chat.primary")
            .with_trace_context(TraceContext::new(
                "trace.runtime-adapter",
                "span.runtime-adapter",
            )),
        )
        .expect("runtime-registered chat RPC adapter handles envelope requests");
    assert_eq!(envelope.object_kind, ProtocolObjectKind::ExtensionObject);
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.object_kind"),
        Some("agent_chat_response")
    );
}

#[test]
fn chat_rpc_adapter_handles_rpc_chat_requests_through_chat_service() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
    );
    let trace = TraceContext::new("trace.rpc.adapter", "span.rpc.adapter");

    let envelope = AgentChatRpcAdapter::new()
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.adapter.handle",
                ProtocolFamily::Rpc,
                "model.chat.invoke",
                "invoke through adapter",
            )
            .with_metadata("sdkwork.chat.provider_id", "provider.model.chat.primary")
            .with_metadata("sdkwork.chat.model_id", "model.chat.fast")
            .with_trace_context(trace),
        )
        .expect("chat RPC adapter handles RPC request through chat service");

    assert_eq!(envelope.object_kind, ProtocolObjectKind::ExtensionObject);
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.object_kind"),
        Some("agent_chat_response")
    );
    assert!(envelope.payload.contains("primary: model.chat.fast"));
    assert_eq!(captured_model_requests.lock().unwrap().len(), 1);
}

#[test]
fn chat_rpc_adapter_maps_stream_updates_to_sse_frames_for_streaming_chat_clients() {
    let trace = TraceContext::new("trace.sse.chat", "span.sse.chat");
    let event = sdkwork_agent_kernel::KernelEvent::new(
        "event.chat.stream.1",
        "agent.model.output.streamed",
        sdkwork_agent_kernel::KernelEventSeverity::Info,
        "model_request_id=chat-rpc.sse;sequence=1;chunk=hello\nworld",
    )
    .for_session("session.sse")
    .for_task("task.sse")
    .with_trace_context(trace.clone())
    .with_payload_schema("sdkwork.agent.model.stream_chunk.v1");

    let update = AgentChatRpcAdapter::new()
        .map_event_to_stream_update(event)
        .expect("chat RPC adapter maps kernel events to ordered stream updates");
    assert_eq!(update.event_id, "event.chat.stream.1");
    assert_eq!(update.event_type, "agent.model.output.streamed");
    assert_eq!(update.sequence, 1);
    assert_eq!(update.trace_context.as_ref(), Some(&trace));

    let sse = update.to_sse_event();
    assert_eq!(sse.event_id, "event.chat.stream.1");
    assert_eq!(sse.event_type, "agent.model.output.streamed");
    assert_eq!(
        sse.to_frame(),
        concat!(
            "id: event.chat.stream.1\n",
            "event: agent.model.output.streamed\n",
            "data: event_version=0.1.0\n",
            "data: sequence=1\n",
            "data: payload=model_request_id=chat-rpc.sse;sequence=1;chunk=hello\n",
            "data: world\n",
            "\n"
        )
    );
}

#[test]
fn chat_rpc_adapter_maps_event_stream_items_to_ordered_sse_frames() {
    let adapter = AgentChatRpcAdapter::new();
    let mut stream = EventStream::new("stream.chat.sse");
    stream.publish(
        sdkwork_agent_kernel::ModelStreamChunk::output("chat-rpc.sse", 1, "hello")
            .to_event("event.chat.sse.1")
            .for_session("session.sse"),
    );
    stream.publish(
        sdkwork_agent_kernel::ModelStreamChunk::output("chat-rpc.sse", 2, "world")
            .to_event("event.chat.sse.2")
            .for_session("session.sse"),
    );

    let batch = stream
        .subscribe(
            "subscription.chat.sse",
            EventStreamFilter::new().for_session("session.sse"),
            EventStreamCursor::from_start(),
            10,
        )
        .expect("chat SSE stream subscription returns ordered events");
    let frames: Vec<String> = batch
        .events
        .into_iter()
        .map(|item| {
            adapter
                .map_stream_item_to_stream_update(item)
                .expect("stream item maps with original sequence")
                .to_sse_event()
                .to_frame()
        })
        .collect();

    assert_eq!(frames.len(), 2);
    assert!(frames[0].contains("id: event.chat.sse.1\n"));
    assert!(frames[0].contains("data: sequence=1\n"));
    assert!(
        frames[0].contains("data: payload=model_request_id=chat-rpc.sse;sequence=1;chunk=hello\n")
    );
    assert!(frames[1].contains("id: event.chat.sse.2\n"));
    assert!(frames[1].contains("data: sequence=2\n"));
    assert!(
        frames[1].contains("data: payload=model_request_id=chat-rpc.sse;sequence=2;chunk=world\n")
    );
}

#[test]
fn chat_rpc_handler_maps_rpc_request_to_chat_service_and_protocol_envelope() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
    );
    let trace = TraceContext::new("trace.rpc.chat", "span.rpc.chat");

    let envelope = AgentChatRpcHandler::new()
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.1",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "Summarize the Rig plugin status",
            )
            .with_external_id("grpc.request.1")
            .with_metadata("sdkwork.chat.provider_id", "provider.model.chat.primary")
            .with_metadata("sdkwork.chat.model_id", "model.chat.fast")
            .with_metadata("sdkwork.agent.session_id", "session.1")
            .with_metadata("sdkwork.agent.task_id", "task.1")
            .with_metadata("sdkwork.agent.run_id", "run.1")
            .with_metadata("sdkwork.agent.step_id", "step.1")
            .with_metadata("sdkwork.chat.timeout_ms", "30000")
            .with_metadata("sdkwork.chat.source", "rpc-contract")
            .with_trace_context(trace.clone()),
        )
        .expect("chat RPC request maps to response envelope");

    envelope
        .validate()
        .expect("chat RPC response envelope metadata is namespaced");
    assert_eq!(envelope.protocol, ProtocolFamily::Rpc);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::ExtensionObject);
    assert_eq!(envelope.object_id, "chat-rpc.1");
    assert_eq!(envelope.external_id.as_deref(), Some("grpc.request.1"));
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.rpc.chat.response.v1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.object_kind"),
        Some("agent_chat_response")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.chat.provider_id"),
        Some("provider.model.chat.primary")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.chat.model_id"),
        Some("model.chat.fast")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.agent.session_id"),
        Some("session.1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.policy.decision"),
        Some("allow")
    );
    assert_eq!(envelope.trace_context.as_ref(), Some(&trace));
    assert_eq!(
        envelope.redaction_classification,
        KernelEventRedaction::Unknown
    );
    assert!(envelope.payload.contains("primary: model.chat.fast"));

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(model_requests[0].model_request_id, "chat-rpc.1");
    assert_eq!(
        model_requests[0].messages,
        ["Summarize the Rig plugin status"]
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.chat.source"),
        Some("rpc-contract")
    );
    assert_eq!(model_requests[0].timeout_ms, Some(30_000));
}

#[test]
fn chat_rpc_handler_maps_policy_subject_without_forwarding_policy_metadata_to_model() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests.clone()),
    );

    AgentChatRpcHandler::new()
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.subject",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello with policy subject",
            )
            .with_metadata("sdkwork.chat.provider_id", "provider.model.chat.primary")
            .with_metadata("sdkwork.policy.subject_id", "user.1")
            .with_metadata("sdkwork.policy.tenant_id", "tenant.1")
            .with_metadata("sdkwork.policy.roles", "admin, developer")
            .with_trace_context(TraceContext::new("trace.rpc.subject", "span.rpc.subject")),
        )
        .expect("policy subject metadata maps to policy context");

    let policy_requests = captured_policy_requests.lock().unwrap();
    let subject = policy_requests[0]
        .subject
        .as_ref()
        .expect("policy subject is mapped");
    assert_eq!(subject.subject_id, "user.1");
    assert_eq!(subject.tenant_id, "tenant.1");
    assert_eq!(subject.roles, ["admin", "developer"]);

    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.chat.provider_id"),
        Some("provider.model.chat.primary")
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.policy.subject_id"),
        None
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.policy.tenant_id"),
        None
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.policy.roles"),
        None
    );
}

#[test]
fn chat_rpc_handler_maps_tool_and_memory_metadata_to_model_request_context() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_tool_calls = Arc::new(Mutex::new(Vec::new()));
    let captured_memory_queries = Arc::new(Mutex::new(Vec::new()));
    let captured_knowledge_queries = Arc::new(Mutex::new(Vec::new()));
    let manifest =
        AgentManifest::from_json(CHAT_AGENT_MANIFEST_JSON).expect("chat manifest parses");
    let runtime = RuntimeBuilder::new("runtime.chat.rpc-enrichment", manifest)
        .with_generated_at("2026-06-09T00:00:00Z")
        .register_model_provider(
            "provider.model.chat.primary",
            "0.1.0",
            RecordingModelProvider::new(
                "provider.model.chat.primary",
                "primary",
                captured_model_requests.clone(),
            ),
        )
        .register_policy_provider(
            "provider.policy.chat",
            "0.1.0",
            RecordingPolicyProvider::allow(
                "provider.policy.chat",
                captured_policy_requests.clone(),
            ),
        )
        .register_tool_provider(
            "provider.tool.chat",
            "0.1.0",
            RecordingToolProvider::new("provider.tool.chat", captured_tool_calls.clone()),
        )
        .register_memory_provider(
            "provider.memory.chat",
            "0.1.0",
            RecordingMemoryProvider::new(captured_memory_queries.clone()).with_record(
                MemoryRecord::new(
                    "memory.rpc.1",
                    MemoryScope::Session,
                    "session.rpc.context",
                    "RPC memory context",
                    TrustLevel::AgentMessage,
                    RedactionClassification::Internal,
                ),
            ),
        )
        .register_knowledge_provider(
            "provider.knowledge.chat",
            "0.1.0",
            RecordingKnowledgeProvider::new(captured_knowledge_queries.clone()).with_result(
                KnowledgeSearchResult::new(
                    "knowledge.rpc.1",
                    KnowledgeDocumentKind::WikiSection,
                    "RPC Knowledge",
                    KnowledgeRetrievalMethod::Hybrid,
                )
                .with_snippet("RPC knowledge context")
                .with_source_uri("kb://space/7/document/22#chunk-1")
                .with_score(0.88),
            ),
        )
        .bootstrap()
        .expect("chat RPC enrichment runtime bootstraps")
        .runtime;

    let envelope = AgentChatRpcHandler::new()
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.enriched",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "chat with context",
            )
            .with_metadata("sdkwork.chat.include_tools", "true")
            .with_metadata("sdkwork.memory.scope", "session")
            .with_metadata("sdkwork.memory.owner_context", "session.rpc.context")
            .with_metadata("sdkwork.knowledge.query", "rpc knowledge query")
            .with_metadata("sdkwork.knowledge.provider_id", "provider.knowledge.chat")
            .with_metadata("sdkwork.knowledge.tenant_id", "tenant.rpc")
            .with_metadata("sdkwork.knowledge.namespace", "sdkwork.rpc")
            .with_metadata("sdkwork.knowledge.top_k", "2")
            .with_metadata("sdkwork.knowledge.methods", "hybrid,keyword")
            .with_metadata("sdkwork.knowledge.filter.space_id", "7")
            .with_metadata("sdkwork.agent.session_id", "session.rpc.context")
            .with_metadata("sdkwork.agent.task_id", "task.rpc.context"),
        )
        .expect("chat RPC metadata maps to tool memory and knowledge enrichment");

    assert_eq!(envelope.object_kind, ProtocolObjectKind::ExtensionObject);
    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(model_requests[0].tool_descriptors.len(), 1);
    assert_eq!(model_requests[0].context_frames.len(), 2);
    assert_eq!(
        model_requests[0].context_frames[0].content,
        "RPC memory context"
    );
    assert_eq!(
        model_requests[0].context_frames[1].content,
        "RPC knowledge context"
    );
    assert_eq!(
        model_requests[0].metadata_value("sdkwork.chat.include_tools"),
        Some("true")
    );
    assert_eq!(
        model_requests[0]
            .metadata
            .iter()
            .filter(|(key, _)| key == "sdkwork.chat.include_tools")
            .count(),
        1,
        "consumed RPC control metadata must be normalized once"
    );
    assert_eq!(
        model_requests[0]
            .metadata
            .iter()
            .filter(|(key, _)| key == "sdkwork.memory.scope")
            .count(),
        1,
        "consumed memory scope metadata must be normalized once"
    );
    assert_eq!(
        model_requests[0]
            .metadata
            .iter()
            .filter(|(key, _)| key == "sdkwork.knowledge.query")
            .count(),
        1,
        "consumed knowledge query metadata must be normalized once"
    );
    assert_eq!(
        captured_memory_queries.lock().unwrap().as_slice(),
        &[(MemoryScope::Session, "session.rpc.context".to_string())]
    );
    assert!(
        captured_tool_calls.lock().unwrap().is_empty(),
        "RPC chat enrichment must not auto-execute exposed tools"
    );
    let knowledge_queries = captured_knowledge_queries.lock().unwrap();
    assert_eq!(knowledge_queries.len(), 1);
    assert_eq!(knowledge_queries[0].query, "rpc knowledge query");
    assert_eq!(
        knowledge_queries[0].tenant_id.as_deref(),
        Some("tenant.rpc")
    );
    assert_eq!(
        knowledge_queries[0].namespace.as_deref(),
        Some("sdkwork.rpc")
    );
    assert_eq!(knowledge_queries[0].top_k, 2);
    assert!(knowledge_queries[0].supports_method(KnowledgeRetrievalMethod::Hybrid));
    assert!(knowledge_queries[0].supports_method(KnowledgeRetrievalMethod::Keyword));
    assert_eq!(knowledge_queries[0].metadata_value("space_id"), Some("7"));
    assert!(captured_policy_requests
        .lock()
        .unwrap()
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::MemoryRead)));
    assert!(captured_policy_requests
        .lock()
        .unwrap()
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::KnowledgeSearch)));
}

#[test]
fn chat_rpc_handler_maps_policy_denial_to_safe_protocol_error_envelope() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::deny(
            "provider.policy.chat",
            captured_policy_requests,
            "model.invoke.denied",
        ),
    );
    let trace = TraceContext::new("trace.rpc.denied", "span.rpc.denied");

    let envelope = AgentChatRpcHandler::new()
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.denied",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello",
            )
            .with_metadata("sdkwork.chat.model_id", "model.chat.fast")
            .with_trace_context(trace.clone()),
        )
        .expect("policy denial maps to protocol error envelope");

    envelope
        .validate()
        .expect("chat RPC error envelope metadata is namespaced");
    assert_eq!(envelope.protocol, ProtocolFamily::Rpc);
    assert_eq!(envelope.object_kind, ProtocolObjectKind::KernelError);
    assert_eq!(
        envelope.payload_schema.as_deref(),
        Some("sdkwork.agent.error.v1")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.error.code"),
        Some("permission_denied")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.error.kernel_code"),
        Some("policy_denied")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.error.kind"),
        Some("policy_denied")
    );
    assert_eq!(
        envelope.metadata_value("sdkwork.protocol.request_id"),
        Some("chat-rpc.denied")
    );
    assert_eq!(envelope.trace_context.as_ref(), Some(&trace));
    assert!(envelope
        .payload
        .contains("safe_message=request denied by policy"));
    assert!(!envelope.payload.contains("model.invoke.denied"));
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "policy-denied RPC requests must not invoke the model provider"
    );
}

#[test]
fn chat_rpc_handler_rejects_invalid_protocol_operations_and_payloads_before_invocation() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let runtime = chat_runtime(
        RecordingModelProvider::new(
            "provider.model.chat.primary",
            "primary",
            captured_model_requests.clone(),
        ),
        RecordingPolicyProvider::allow("provider.policy.chat", captured_policy_requests),
    );
    let handler = AgentChatRpcHandler::new();

    let non_rpc_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.invalid-protocol",
                ProtocolFamily::Http,
                "agent.chat.create",
                "hello",
            ),
        )
        .expect_err("chat RPC handler only accepts RPC protocol requests");
    assert_eq!(non_rpc_error.kind(), KernelErrorKind::ValidationError);

    let unsupported_operation_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.unsupported",
                ProtocolFamily::Rpc,
                "agent.chat.delete",
                "hello",
            ),
        )
        .expect_err("chat RPC handler rejects unsupported operations");
    assert_eq!(
        unsupported_operation_error.kind(),
        KernelErrorKind::ValidationError
    );

    let empty_payload_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.empty",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                " ",
            ),
        )
        .expect_err("chat RPC handler rejects empty chat payloads");
    assert_eq!(empty_payload_error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid RPC requests must be rejected before model invocation"
    );

    let incomplete_subject_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.incomplete-subject",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello",
            )
            .with_metadata("sdkwork.policy.subject_id", "user.1"),
        )
        .expect_err("chat RPC handler rejects incomplete policy subject metadata");
    assert_eq!(
        incomplete_subject_error.kind(),
        KernelErrorKind::ValidationError
    );
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid policy subject metadata must be rejected before model invocation"
    );

    let orphan_roles_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.orphan-roles",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello",
            )
            .with_metadata("sdkwork.policy.roles", "developer"),
        )
        .expect_err("chat RPC handler rejects policy roles without a policy subject");
    assert_eq!(orphan_roles_error.kind(), KernelErrorKind::ValidationError);

    let incomplete_memory_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.incomplete-memory",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello",
            )
            .with_metadata("sdkwork.memory.scope", "session"),
        )
        .expect_err("chat RPC handler rejects incomplete memory query metadata");
    assert_eq!(
        incomplete_memory_error.kind(),
        KernelErrorKind::ValidationError
    );

    let incomplete_knowledge_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.incomplete-knowledge",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello",
            )
            .with_metadata("sdkwork.knowledge.provider_id", "provider.knowledge.chat"),
        )
        .expect_err("chat RPC handler rejects incomplete knowledge query metadata");
    assert_eq!(
        incomplete_knowledge_error.kind(),
        KernelErrorKind::ValidationError
    );

    let zero_top_k_error = handler
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.zero-top-k",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "hello",
            )
            .with_metadata("sdkwork.knowledge.query", "hello")
            .with_metadata("sdkwork.knowledge.top_k", "0"),
        )
        .expect_err("chat RPC handler rejects zero knowledge top_k metadata");
    assert_eq!(zero_top_k_error.kind(), KernelErrorKind::ValidationError);
    assert!(
        captured_model_requests.lock().unwrap().is_empty(),
        "invalid knowledge metadata must be rejected before model invocation"
    );
}

fn chat_runtime(
    model_provider: RecordingModelProvider,
    policy_provider: RecordingPolicyProvider,
) -> sdkwork_agent_kernel::AgentRuntime {
    let manifest =
        AgentManifest::from_json(CHAT_AGENT_MANIFEST_JSON).expect("chat manifest parses");
    RuntimeBuilder::new("runtime.chat.local", manifest)
        .with_generated_at("2026-06-09T00:00:00Z")
        .register_model_provider(model_provider.provider_id.clone(), "0.1.0", model_provider)
        .register_policy_provider(
            policy_provider.provider_id.clone(),
            "0.1.0",
            policy_provider,
        )
        .bootstrap()
        .expect("chat runtime bootstraps")
        .runtime
}

fn chat_runtime_with_tool_provider<M, T>(
    model_provider: M,
    policy_provider: RecordingPolicyProvider,
    tool_provider: T,
) -> sdkwork_agent_kernel::AgentRuntime
where
    M: ModelProvider + Send + Sync + 'static,
    T: ToolProvider + Send + Sync + 'static,
{
    let manifest =
        AgentManifest::from_json(CHAT_AGENT_MANIFEST_JSON).expect("chat manifest parses");
    let model_provider_id = model_provider.provider_manifest().provider_id;
    let tool_provider_id = tool_provider.provider_manifest().provider_id;

    RuntimeBuilder::new("runtime.chat.local", manifest)
        .with_generated_at("2026-06-09T00:00:00Z")
        .register_model_provider(model_provider_id, "0.1.0", model_provider)
        .register_policy_provider(
            policy_provider.provider_id.clone(),
            "0.1.0",
            policy_provider,
        )
        .register_tool_provider(tool_provider_id, "0.1.0", tool_provider)
        .bootstrap()
        .expect("chat runtime with tool provider bootstraps")
        .runtime
}

fn chat_runtime_with_memory_provider<M, P>(
    model_provider: M,
    policy_provider: P,
    memory_provider: RecordingMemoryProvider,
) -> sdkwork_agent_kernel::AgentRuntime
where
    M: ModelProvider + Send + Sync + 'static,
    P: PolicyProvider + Send + Sync + 'static,
{
    let manifest =
        AgentManifest::from_json(CHAT_AGENT_MANIFEST_JSON).expect("chat manifest parses");
    let model_provider_id = model_provider.provider_manifest().provider_id;
    let policy_provider_id = policy_provider.provider_manifest().provider_id;
    let memory_provider_id = memory_provider.provider_manifest().provider_id;

    RuntimeBuilder::new("runtime.chat.local", manifest)
        .with_generated_at("2026-06-09T00:00:00Z")
        .register_model_provider(model_provider_id, "0.1.0", model_provider)
        .register_policy_provider(policy_provider_id, "0.1.0", policy_provider)
        .register_memory_provider(memory_provider_id, "0.1.0", memory_provider)
        .bootstrap()
        .expect("chat runtime with memory provider bootstraps")
        .runtime
}

fn chat_runtime_with_knowledge_provider<M, P>(
    model_provider: M,
    policy_provider: P,
    knowledge_provider: RecordingKnowledgeProvider,
) -> sdkwork_agent_kernel::AgentRuntime
where
    M: ModelProvider + Send + Sync + 'static,
    P: PolicyProvider + Send + Sync + 'static,
{
    let manifest =
        AgentManifest::from_json(CHAT_AGENT_MANIFEST_JSON).expect("chat manifest parses");
    let model_provider_id = model_provider.provider_manifest().provider_id;
    let policy_provider_id = policy_provider.provider_manifest().provider_id;
    let knowledge_provider_id = knowledge_provider.provider_manifest().provider_id;

    RuntimeBuilder::new("runtime.chat.local", manifest)
        .with_generated_at("2026-06-09T00:00:00Z")
        .register_model_provider(model_provider_id, "0.1.0", model_provider)
        .register_policy_provider(policy_provider_id, "0.1.0", policy_provider)
        .register_knowledge_provider(knowledge_provider_id, "0.1.0", knowledge_provider)
        .bootstrap()
        .expect("chat runtime with knowledge provider bootstraps")
        .runtime
}

#[derive(Clone)]
struct RecordingModelProvider {
    provider_id: String,
    response_prefix: String,
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl RecordingModelProvider {
    fn new(
        provider_id: impl Into<String>,
        response_prefix: impl Into<String>,
        captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            response_prefix: response_prefix.into(),
            captured_requests,
        }
    }
}

impl ModelProvider for RecordingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "recording-chat-model",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.captured_requests.lock().unwrap().push(request.clone());
        let model_id = request.model_id.as_deref().unwrap_or("default");
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            format!("{}: {model_id}", self.response_prefix),
        )
        .with_optional_trace_context(request.trace_context))
    }
}

trait ModelResponseTestExt {
    fn with_optional_trace_context(self, trace_context: Option<TraceContext>) -> Self;
}

impl ModelResponseTestExt for ModelResponse {
    fn with_optional_trace_context(self, trace_context: Option<TraceContext>) -> Self {
        if let Some(trace_context) = trace_context {
            self.with_trace_context(trace_context)
        } else {
            self
        }
    }
}

#[derive(Clone)]
struct RecordingPolicyProvider {
    provider_id: String,
    captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
    mode: RecordingPolicyMode,
}

impl RecordingPolicyProvider {
    fn allow(
        provider_id: impl Into<String>,
        captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            captured_requests,
            mode: RecordingPolicyMode::Allow,
        }
    }

    fn deny(
        provider_id: impl Into<String>,
        captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            captured_requests,
            mode: RecordingPolicyMode::Deny(reason_code.into()),
        }
    }

    fn needs_approval(
        provider_id: impl Into<String>,
        captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            captured_requests,
            mode: RecordingPolicyMode::NeedsApproval(reason_code.into()),
        }
    }

    fn deny_knowledge_search(
        provider_id: impl Into<String>,
        captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            captured_requests,
            mode: RecordingPolicyMode::DenyCategory {
                category: PolicyCategory::KnowledgeSearch,
                reason_code: reason_code.into(),
            },
        }
    }
}

impl PolicyProvider for RecordingPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "policy",
            "recording-chat-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(match &self.mode {
            RecordingPolicyMode::Allow => PolicyDecision::allow(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                self.provider_id.clone(),
            ),
            RecordingPolicyMode::Deny(reason_code) => PolicyDecision::deny(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                self.provider_id.clone(),
                reason_code.clone(),
            ),
            RecordingPolicyMode::NeedsApproval(reason_code) => PolicyDecision::needs_approval(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                self.provider_id.clone(),
                reason_code.clone(),
            ),
            RecordingPolicyMode::DenyCategory {
                category,
                reason_code,
            } if request.typed_category == Some(category.clone()) => PolicyDecision::deny(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                self.provider_id.clone(),
                reason_code.clone(),
            ),
            RecordingPolicyMode::DenyCategory { .. } => PolicyDecision::allow(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                self.provider_id.clone(),
            ),
        })
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
enum RecordingPolicyMode {
    Allow,
    Deny(String),
    NeedsApproval(String),
    DenyCategory {
        category: PolicyCategory,
        reason_code: String,
    },
}

#[derive(Clone)]
struct RecordingToolProvider {
    provider_id: String,
    captured_calls: Arc<Mutex<Vec<ToolCall>>>,
}

impl RecordingToolProvider {
    fn new(provider_id: impl Into<String>, captured_calls: Arc<Mutex<Vec<ToolCall>>>) -> Self {
        Self {
            provider_id: provider_id.into(),
            captured_calls,
        }
    }
}

impl ToolProvider for RecordingToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "tool",
            "recording-chat-tool",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "tool.chat.search",
            self.provider_id.clone(),
            "Chat Search",
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
        Ok(ToolResult::succeeded(call.tool_call_id, "tool output"))
    }
}

#[derive(Clone)]
struct RecordingMemoryProvider {
    records: Vec<MemoryRecord>,
    captured_queries: Arc<Mutex<Vec<(MemoryScope, String)>>>,
}

impl RecordingMemoryProvider {
    fn new(captured_queries: Arc<Mutex<Vec<(MemoryScope, String)>>>) -> Self {
        Self {
            records: Vec::new(),
            captured_queries,
        }
    }

    fn with_record(mut self, record: MemoryRecord) -> Self {
        self.records.push(record);
        self
    }
}

impl MemoryProvider for RecordingMemoryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.memory.chat",
            "memory",
            "recording-chat-memory",
            "0.1.0",
            vec!["memory.query".to_string(), "memory.write".to_string()],
        )
    }

    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        self.captured_queries
            .lock()
            .unwrap()
            .push((scope, owner_context.to_string()));
        Ok(self
            .records
            .iter()
            .filter(|record| record.scope == scope && record.owner_context == owner_context)
            .cloned()
            .collect())
    }

    fn write(&mut self, record: MemoryRecord) -> KernelResult<()> {
        self.records
            .retain(|existing| existing.memory_record_id != record.memory_record_id);
        self.records.push(record);
        Ok(())
    }

    fn delete(&mut self, memory_record_id: &str) -> KernelResult<()> {
        self.records
            .retain(|record| record.memory_record_id != memory_record_id);
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
struct RecordingKnowledgeProvider {
    results: Vec<KnowledgeSearchResult>,
    captured_queries: Arc<Mutex<Vec<KnowledgeSearchRequest>>>,
}

impl RecordingKnowledgeProvider {
    fn new(captured_queries: Arc<Mutex<Vec<KnowledgeSearchRequest>>>) -> Self {
        Self {
            results: Vec::new(),
            captured_queries,
        }
    }

    fn with_result(mut self, result: KnowledgeSearchResult) -> Self {
        self.results.push(result);
        self
    }
}

impl KnowledgeProvider for RecordingKnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.knowledge.chat",
            "knowledge",
            "recording-chat-knowledge",
            "0.1.0",
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        self.captured_queries.lock().unwrap().push(request);
        Ok(self.results.clone())
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::WikiSection,
            "Knowledge Document",
            "knowledge document content",
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
struct ToolCallingModelProvider {
    provider_id: String,
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl ToolCallingModelProvider {
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

impl ModelProvider for ToolCallingModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "tool-calling-chat-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.tool_call".to_string()],
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
            "tool call requested",
        )
        .with_tool_call(
            ToolCall::new(
                "tool-call.from-model.1",
                "tool.chat.search",
                r#"{"query":"sdkwork"}"#,
            )
            .for_session("session.tool-call")
            .for_task("task.tool-call")
            .with_provider("provider.tool.chat"),
        ))
    }
}
