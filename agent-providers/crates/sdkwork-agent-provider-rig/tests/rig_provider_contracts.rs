use sdkwork_agent_kernel::{
    AgentExecutionRequest, AgentExecutionService, AgentExecutionStatus, KernelErrorKind,
    KernelResult, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, McpProvider, McpTransportKind,
    MemoryProvider, MemoryRecord, MemoryScope, ModelExecutionRequest, ModelExecutionService,
    ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat, PlanningProvider,
    PolicyCategory, PolicyDecisionValue, PolicyProvider, PolicyRequest, ProtocolAdapterRequest,
    ProtocolFamily, ProtocolObjectKind, RedactionClassification, RuntimeBuilder, SideEffectLevel,
    TrustLevel,
};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_rig::{
    ids, RigAgentInstaller, RigBackend, RigBackendBootstrapState, RigBackendConfig,
    RigBackendExecutionState, RigBackendExecutor, RigBackendMode, RigConfigurationProvider,
    RigKernelPlugin, RigKnowledgeProvider, RigMcpProvider, RigMemoryProvider, RigModelProvider,
    RigPlanningProvider,
};
use std::sync::{Arc, Mutex};

struct TestRigExecutor;

impl RigBackendExecutor for TestRigExecutor {
    fn invoke_model(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            ids::MODEL_PROVIDER_ID,
            "official-adapter-result",
        ))
    }
}

#[test]
fn rig_model_provider_exposes_catalog_and_fails_closed_without_live_backend() {
    let provider = RigModelProvider::fail_closed();

    let manifest = provider.provider_manifest();
    assert_ne!(
        provider.health().status,
        "available",
        "fail-closed Rig model provider must not report executable chat as healthy"
    );
    assert_eq!(manifest.provider_id, ids::MODEL_PROVIDER_ID);
    assert!(manifest.capabilities.contains(&"model.catalog".to_string()));
    assert!(manifest.capabilities.contains(&"model.chat".to_string()));
    assert!(
        !manifest.capabilities.contains(&"model.tool_call".to_string()),
        "Rig fail-closed model provider must not claim tool-call output until the catalog and backend support it"
    );

    let models = provider.list_models();
    assert!(!models.is_empty());
    assert_eq!(models[0].provider_id, ids::MODEL_PROVIDER_ID);
    assert!(models[0].supports_capability("model.chat"));
    assert!(
        !models[0].supports_capability("model.tool_call"),
        "Rig default model descriptor must mirror implemented model capabilities"
    );
    assert!(models[0].supports_response_format(&ModelResponseFormat::Text));

    let error = provider
        .invoke(ModelRequest::new(
            "model.request.1",
            vec!["hello".to_string()],
        ))
        .expect_err("live invocation must fail closed without backend");
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some(ids::MODEL_PROVIDER_ID));
}

#[test]
fn rig_mcp_provider_exposes_only_implemented_resources_and_prompts() {
    let provider = RigMcpProvider::fail_closed();

    let manifest = provider.provider_manifest();
    assert_eq!(manifest.provider_id, ids::MCP_PROVIDER_ID);
    assert_eq!(manifest.provider_family, "mcp");
    assert_eq!(manifest.capabilities, ["mcp.resources", "mcp.prompts"]);
    assert_eq!(provider.health().status, "degraded");

    let server = provider.list_servers().expect("list_servers")[0].clone();
    assert_eq!(server.provider_id, ids::MCP_PROVIDER_ID);
    assert_eq!(server.transport, McpTransportKind::Sse);
    assert!(!server.capabilities.contains(&"mcp.tools".to_string()));
    assert!(server.capabilities.contains(&"mcp.resources".to_string()));
    assert!(server.capabilities.contains(&"mcp.prompts".to_string()));

    let tools = provider
        .list_tools(&server.server_id)
        .expect("Rig MCP tools are discoverable");
    assert!(tools.is_empty());

    let resource = provider
        .read_resource(&server.server_id, "rig://knowledge/adapter")
        .expect("Rig MCP resource is readable");
    assert_eq!(resource.trust_level, TrustLevel::TrustedHost);
    assert_eq!(resource.metadata_value("sdkwork.adapter"), Some("rig-core"));
    let resource_context = resource.to_context_frame("session.rig.mcp");
    assert_eq!(resource_context.source, "mcp.resource");
    assert_eq!(
        resource_context.metadata_value("sdkwork.adapter"),
        Some("rig-core")
    );

    let prompt = provider
        .get_prompt(&server.server_id, "rig.prompt.chat", Vec::new())
        .expect("Rig MCP prompt is loadable");
    let prompt_context = prompt.to_context_frames("session.rig.mcp");
    assert_eq!(prompt_context[0].source, "mcp.prompt");
    assert_eq!(
        prompt_context[0].metadata_value("sdkwork.adapter"),
        Some("rig-core")
    );
}

#[test]
fn rig_live_backend_configuration_remains_fail_closed_until_adapter_is_connected() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };

    let model_provider = RigModelProvider::with_backend_config(config.clone());
    assert_eq!(model_provider.health().status, "degraded");
    let error = model_provider
        .invoke(ModelRequest::new(
            "model.request.live-pending",
            vec!["hello".to_string()],
        ))
        .expect_err("configured live backend must fail closed until upstream adapter is connected");
    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some(ids::MODEL_PROVIDER_ID));
}

#[test]
fn rig_live_backend_executes_only_after_adapter_injection() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };
    let provider = RigModelProvider::with_executor(config, Arc::new(TestRigExecutor));

    assert_eq!(provider.health().status, "available");
    assert_eq!(
        provider.backend_execution_status().state,
        RigBackendExecutionState::Live
    );
    assert!(!provider.backend_execution_status().fail_closed);
    let response = provider
        .invoke(ModelRequest::new(
            "model.request.live",
            vec!["hello".to_string()],
        ))
        .expect("injected live adapter must execute");
    assert_eq!(response.messages, vec!["official-adapter-result"]);
}

#[test]
fn rig_backend_execution_status_distinguishes_live_pending_from_fail_closed() {
    let fail_closed_status = RigBackend::fail_closed().execution_status();
    assert_eq!(fail_closed_status.mode, RigBackendMode::FailClosed);
    assert_eq!(
        fail_closed_status.state,
        RigBackendExecutionState::FailClosed
    );
    assert_eq!(fail_closed_status.state.as_str(), "fail_closed");
    assert!(fail_closed_status.fail_closed);
    assert!(fail_closed_status.safe_reason.contains("fail-closed"));

    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };
    let live_pending_status = config.execution_status();
    assert_eq!(live_pending_status.mode, RigBackendMode::Live);
    assert_eq!(
        live_pending_status.state,
        RigBackendExecutionState::LivePending
    );
    assert_eq!(live_pending_status.state.as_str(), "live_pending");
    assert!(live_pending_status.fail_closed);
    assert!(live_pending_status.safe_reason.contains("not connected"));

    let model_provider = RigModelProvider::with_backend_config(config.clone());
    assert_eq!(
        model_provider.backend_execution_status(),
        live_pending_status
    );
    let models = model_provider.list_models();
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.mode"),
        Some("live")
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.execution_state"),
        Some("live_pending")
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.fail_closed"),
        Some("true")
    );
}

#[test]
fn rig_providers_expose_secret_safe_backend_bootstrap_plan() {
    let config = RigBackendConfig {
        mode: RigBackendMode::Live,
        provider_id: Some("openai".to_string()),
        api_key_secret_ref: Some("secret://rig/openai".to_string()),
    };

    let model_provider = RigModelProvider::with_backend_config(config.clone());
    let model_plan = model_provider.backend_bootstrap_plan();
    assert_eq!(model_plan.state, RigBackendBootstrapState::LivePending);
    assert_eq!(model_plan.provider_id.as_deref(), Some("openai"));
    assert_eq!(model_plan.required_secret_refs, vec!["llm.rig.api_key"]);
    assert_eq!(
        model_plan.secret_ref_value("llm.rig.api_key"),
        Some("secret://rig/openai")
    );
    assert!(
        !model_plan.safe_summary.contains("secret://rig/openai"),
        "provider safe bootstrap summary must not echo secret references"
    );

    let models = model_provider.list_models();
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.bootstrap_state"),
        Some("live_pending")
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.provider_id"),
        Some("openai")
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.required_secret_refs"),
        Some("llm.rig.api_key")
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.policy_categories"),
        Some("model.invoke,host.secrets.read")
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.api_key_secret_ref"),
        None,
        "model catalog metadata must not expose secret reference values"
    );
    assert_eq!(
        models[0].metadata_value("sdkwork.backend.safe_summary"),
        Some(model_plan.safe_summary.as_str())
    );
}

#[test]
fn rig_memory_provider_maps_sdkwork_memory_records() {
    let mut provider = RigMemoryProvider::new();
    let manifest = provider.provider_manifest();
    assert_eq!(manifest.provider_id, ids::MEMORY_PROVIDER_ID);
    assert!(manifest.capabilities.contains(&"memory.query".to_string()));
    assert!(manifest.capabilities.contains(&"memory.write".to_string()));

    provider
        .write(MemoryRecord::new(
            "memory.rig.session.1",
            MemoryScope::Session,
            "session.1",
            "remember active session preference",
            TrustLevel::AgentMessage,
            RedactionClassification::Internal,
        ))
        .expect("Rig memory accepts SDKWork record writes");
    provider
        .write(MemoryRecord::new(
            "memory.rig.agent.1",
            MemoryScope::Agent,
            ids::AGENT_ID,
            "remember agent-level instruction",
            TrustLevel::TrustedHost,
            RedactionClassification::Internal,
        ))
        .expect("Rig memory supports agent scoped records");

    let session_records = provider
        .query(MemoryScope::Session, "session.1")
        .expect("Rig memory supports scoped queries");
    assert_eq!(session_records.len(), 1);
    assert_eq!(session_records[0].memory_record_id, "memory.rig.session.1");
    assert_eq!(
        session_records[0].content,
        "remember active session preference"
    );

    let exported = provider
        .export(MemoryScope::Session, "session.1")
        .expect("Rig memory export mirrors query scope");
    assert_eq!(exported, session_records);

    provider
        .delete("memory.rig.session.1")
        .expect("Rig memory supports deletes by record id");
    assert!(provider
        .query(MemoryScope::Session, "session.1")
        .expect("query after delete succeeds")
        .is_empty());
    assert_eq!(
        provider
            .query(MemoryScope::Agent, ids::AGENT_ID)
            .expect("other memory scopes are isolated")
            .len(),
        1
    );
}

#[test]
fn rig_knowledge_provider_exposes_provider_neutral_retrieval() {
    let provider = RigKnowledgeProvider::new();
    let manifest = provider.provider_manifest();
    assert_eq!(manifest.provider_id, ids::KNOWLEDGE_PROVIDER_ID);
    assert_eq!(manifest.provider_family, "knowledge");
    assert!(manifest
        .capabilities
        .contains(&"knowledge.search".to_string()));

    let results = provider
        .search(
            KnowledgeSearchRequest::new("rig")
                .with_namespace("sdkwork.rig")
                .with_method(KnowledgeRetrievalMethod::Keyword)
                .with_method(KnowledgeRetrievalMethod::Graph),
        )
        .expect("Rig knowledge searches through SDKWork SPI");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].retrieval_method,
        KnowledgeRetrievalMethod::Keyword
    );
    assert_eq!(results[0].source_uri.as_deref(), Some("external/rig"));
    assert!(results[0]
        .metadata
        .iter()
        .any(|(key, value)| key == "sdkwork.adapter" && value == "rig-core"));

    let filtered_results = provider
        .search(
            KnowledgeSearchRequest::new("rig")
                .with_namespace("sdkwork.rig")
                .with_filter("tag", "knowledge")
                .with_filter("retrieval_method", "keyword"),
        )
        .expect("Rig knowledge applies provider-neutral request filters");
    assert_eq!(filtered_results.len(), 1);

    let mismatched_results = provider
        .search(
            KnowledgeSearchRequest::new("rig")
                .with_namespace("sdkwork.rig")
                .with_filter("tag", "missing"),
        )
        .expect("Rig knowledge applies mismatched filters");
    assert!(mismatched_results.is_empty());

    let document = provider
        .read(&results[0].document_id)
        .expect("Rig knowledge reads by document id");
    assert_eq!(document.namespace.as_deref(), Some("sdkwork.rig"));
    assert!(document
        .retrieval_methods
        .contains(&KnowledgeRetrievalMethod::Vector));

    let listed = provider
        .list(
            KnowledgeDocumentFilter::new()
                .with_namespace("sdkwork.rig")
                .with_kind(KnowledgeDocumentKind::WikiSection)
                .with_tag("knowledge"),
        )
        .expect("Rig knowledge lists filtered documents");
    assert_eq!(listed.len(), 1);
}

#[cfg(feature = "rig-core-adapter")]
#[test]
fn rig_core_adapter_wraps_vector_search_without_leaking_rig_types() {
    let plan = sdkwork_agent_provider_rig::RigCoreKnowledgeAdapter::vector_search_plan(
        &KnowledgeSearchRequest::new("rig adapter").with_top_k(3),
    );

    assert_eq!(plan.query, "rig adapter");
    assert_eq!(plan.samples, 3);
}

#[test]
fn rig_planning_provider_creates_valid_policy_aware_plan() {
    let provider = RigPlanningProvider::new();
    let plan = provider
        .create_plan("task.1", "run.1", "summarize repository")
        .expect("rig plan should be created");

    assert_eq!(plan.task_id, "task.1");
    assert!(!plan.actions.is_empty());
    plan.validate().expect("rig plan is valid");
}

#[test]
fn rig_planning_and_policy_provider_trait_manifests_match_rig_provider_ids() {
    let planning_provider = RigPlanningProvider::new();
    let planning_manifest = PlanningProvider::provider_manifest(&planning_provider);
    assert_eq!(planning_manifest.provider_id, ids::PLANNING_PROVIDER_ID);
    assert_eq!(planning_manifest.provider_family, "planning");
    assert_eq!(planning_manifest.name, "rig-rust-planning");
    assert!(planning_manifest
        .capabilities
        .contains(&"planning.create".to_string()));

    let policy_provider = sdkwork_agent_provider_rig::RigPolicyProvider::new();
    let policy_manifest = PolicyProvider::provider_manifest(&policy_provider);
    assert_eq!(policy_manifest.provider_id, ids::POLICY_PROVIDER_ID);
    assert_eq!(policy_manifest.provider_family, "policy");
    assert_eq!(policy_manifest.name, "rig-local-conformance-policy");
    assert!(policy_manifest
        .capabilities
        .contains(&"policy.evaluate".to_string()));
}

#[test]
fn rig_policy_provider_requires_approval_for_side_effectful_requests() {
    let policy_provider = sdkwork_agent_provider_rig::RigPolicyProvider::new();

    let read_decision = policy_provider
        .evaluate(
            PolicyRequest::new(
                "policy.rig.knowledge.read",
                "knowledge.search",
                "knowledge.rig.adapter",
            )
            .with_category(PolicyCategory::KnowledgeSearch)
            .with_side_effect_level(SideEffectLevel::ReadOnly),
        )
        .expect("read-only Rig policy request is evaluated");
    assert_eq!(read_decision.decision, PolicyDecisionValue::Allow);

    let model_decision = policy_provider
        .evaluate(
            PolicyRequest::new(
                "policy.rig.model.invoke",
                "model.invoke",
                ids::DEFAULT_MODEL_ID,
            )
            .with_category(PolicyCategory::ModelInvoke)
            .with_side_effect_level(SideEffectLevel::ExternalSend),
        )
        .expect("model invoke policy request is evaluated");
    assert_eq!(
        model_decision.decision,
        PolicyDecisionValue::Allow,
        "Rig local policy must allow model.invoke to reach the fail-closed backend"
    );

    let privileged_model_decision = policy_provider
        .evaluate(
            PolicyRequest::new(
                "policy.rig.model.privileged",
                "model.invoke",
                ids::DEFAULT_MODEL_ID,
            )
            .with_category(PolicyCategory::ModelInvoke)
            .with_side_effect_level(SideEffectLevel::Privileged),
        )
        .expect("privileged Rig policy request is evaluated");
    assert_eq!(
        privileged_model_decision.decision,
        PolicyDecisionValue::NeedsApproval
    );
    assert!(privileged_model_decision.audit_required);

    let sensitive_model_decision = policy_provider
        .evaluate(
            PolicyRequest::new(
                "policy.rig.model.sensitive",
                "model.send_sensitive_context",
                ids::DEFAULT_MODEL_ID,
            )
            .with_category(PolicyCategory::ModelSendSensitiveContext)
            .with_side_effect_level(SideEffectLevel::ExternalSend),
        )
        .expect("sensitive model policy request is evaluated");
    assert_eq!(
        sensitive_model_decision.decision,
        PolicyDecisionValue::NeedsApproval
    );

    let tool_decision = policy_provider
        .evaluate(
            PolicyRequest::new(
                "policy.rig.tool.invoke",
                "tool.invoke",
                "tool.rig.policy-test",
            )
            .with_category(PolicyCategory::ToolInvoke)
            .with_side_effect_level(SideEffectLevel::SideEffectful),
        )
        .expect("side-effectful Rig policy request is evaluated");
    assert_eq!(tool_decision.decision, PolicyDecisionValue::NeedsApproval);
    assert_eq!(
        tool_decision.safe_reason.as_deref(),
        Some("Rig local conformance policy requires approval for side-effectful actions")
    );
    assert!(tool_decision.audit_required);

    let secrets_decision = policy_provider
        .evaluate(
            PolicyRequest::new(
                "policy.rig.secrets.read",
                "host.secrets.read",
                "secret.rig.api-key",
            )
            .with_category(PolicyCategory::HostSecretsRead)
            .with_side_effect_level(SideEffectLevel::ReadOnly),
        )
        .expect("secret-read Rig policy request is evaluated");
    assert_eq!(
        secrets_decision.decision,
        PolicyDecisionValue::NeedsApproval
    );
    assert!(secrets_decision.audit_required);
}

#[test]
fn rig_plugin_model_provider_can_be_selected_by_provider_id() {
    let plugin = RigKernelPlugin::fail_closed();
    let report = plugin
        .configure_runtime(sdkwork_agent_kernel::RuntimeBuilder::new(
            "runtime.rig.local",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("rig runtime bootstraps");

    let provider = report
        .runtime
        .model_provider_by_id(ids::MODEL_PROVIDER_ID)
        .expect("rig model provider is registered by id");
    assert_eq!(
        provider.provider_manifest().provider_id,
        ids::MODEL_PROVIDER_ID
    );

    let memory_provider = report
        .runtime
        .memory_provider_by_id(ids::MEMORY_PROVIDER_ID)
        .expect("rig memory provider is registered by id");
    let mut memory_provider = memory_provider
        .lock()
        .expect("memory provider lock is available");
    memory_provider
        .write(MemoryRecord::new(
            "memory.runtime.1",
            MemoryScope::Session,
            "session.runtime",
            "runtime memory",
            TrustLevel::AgentMessage,
            RedactionClassification::Internal,
        ))
        .expect("registered Rig memory provider is writable");
    assert_eq!(
        memory_provider
            .query(MemoryScope::Session, "session.runtime")
            .expect("registered Rig memory provider is queryable")
            .len(),
        1
    );

    let knowledge_provider = report
        .runtime
        .knowledge_provider_by_id(ids::KNOWLEDGE_PROVIDER_ID)
        .expect("rig knowledge provider is registered by id");
    let results = knowledge_provider
        .search(KnowledgeSearchRequest::new("adapter").with_namespace("sdkwork.rig"))
        .expect("registered Rig knowledge provider is searchable");
    assert_eq!(results.len(), 1);
}

#[test]
fn rig_runtime_model_execution_service_reaches_fail_closed_model_backend() {
    let plugin = RigKernelPlugin::fail_closed();
    let runtime = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.rig.model.execution",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("rig model execution runtime bootstraps")
        .runtime;

    let error = ModelExecutionService::new()
        .invoke(
            &runtime,
            ModelExecutionRequest::new(
                "model.execution.rig",
                ModelRequest::new("model.request.rig", vec!["hello Rig".to_string()])
                    .with_model_id(ids::DEFAULT_MODEL_ID),
            )
            .with_provider_id(ids::MODEL_PROVIDER_ID),
        )
        .expect_err("Rig model service reaches fail-closed backend");

    assert_eq!(error.kind(), KernelErrorKind::ProviderUnavailable);
    assert_eq!(error.provider_id(), Some(ids::MODEL_PROVIDER_ID));
}

#[test]
fn rig_runtime_agent_execution_service_reports_fail_closed_model_backend() {
    let plugin = RigKernelPlugin::fail_closed();
    let runtime = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.rig.agent.execution",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("rig agent execution runtime bootstraps")
        .runtime;

    let report = AgentExecutionService::new()
        .execute(
            &runtime,
            AgentExecutionRequest::new("execution.rig.fail-closed", vec!["hello Rig".to_string()])
                .with_provider_id(ids::MODEL_PROVIDER_ID)
                .with_model_id(ids::DEFAULT_MODEL_ID)
                .for_session("session.rig.execution")
                .for_task("task.rig.execution")
                .for_run("run.rig.execution"),
        )
        .expect("Rig fail-closed backend is represented as an execution report");

    assert_eq!(report.status, AgentExecutionStatus::Failed);
    assert!(report.plan.is_some());
    assert!(report.model_response.is_none());
    assert!(report.tool_executions.is_empty());
    assert!(report.mcp_tool_executions.is_empty());
    assert_eq!(
        report.error.as_ref().unwrap().kind(),
        KernelErrorKind::ProviderUnavailable
    );
    assert_eq!(
        report.error.as_ref().unwrap().provider_id(),
        Some(ids::MODEL_PROVIDER_ID)
    );
    assert_eq!(report.observations.len(), 1);
    assert_eq!(report.observations[0].source_family, "model");
    assert_eq!(report.observations[0].status, "provider_unavailable");
}

#[test]
fn rig_runtime_chat_rpc_enriches_model_requests_with_memory_and_knowledge() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let mut memory_provider = RigMemoryProvider::new();
    memory_provider
        .write(MemoryRecord::new(
            "memory.rig.chat.1",
            MemoryScope::Session,
            "session.rig.chat",
            "Rig chat memory context",
            TrustLevel::AgentMessage,
            RedactionClassification::Internal,
        ))
        .expect("Rig memory can be seeded before runtime registration");

    let runtime = RuntimeBuilder::new(
        "runtime.rig.chat.enrichment",
        RigKernelPlugin::fail_closed().agent_manifest(),
    )
    .with_generated_at("2026-06-09T00:00:00Z")
    .register_model_provider(
        ids::MODEL_PROVIDER_ID,
        "0.1.0",
        RecordingRigChatModelProvider::new(captured_model_requests.clone()),
    )
    .register_memory_provider(ids::MEMORY_PROVIDER_ID, "0.1.0", memory_provider)
    .register_knowledge_provider(
        ids::KNOWLEDGE_PROVIDER_ID,
        "0.1.0",
        RigKnowledgeProvider::new(),
    )
    .register_policy_provider(
        ids::POLICY_PROVIDER_ID,
        "0.1.0",
        sdkwork_agent_provider_rig::RigPolicyProvider::new(),
    )
    .register_protocol_adapter(
        ids::CHAT_RPC_ADAPTER_ID,
        "0.1.0",
        sdkwork_agent_kernel::AgentChatRpcAdapter::new(),
    )
    .register_agent_installer(
        ids::INSTALLER_PROVIDER_ID,
        "0.1.0",
        RigAgentInstaller::new(),
    )
    .register_agent_configuration(
        ids::CONFIGURATION_PROVIDER_ID,
        "0.1.0",
        RigConfigurationProvider::new(),
    )
    .bootstrap()
    .expect("Rig chat enrichment runtime bootstraps")
    .runtime;

    let envelope = runtime
        .protocol_adapter_by_id(ids::CHAT_RPC_ADAPTER_ID)
        .expect("Rig chat RPC adapter is registered")
        .handle_request(
            &runtime,
            ProtocolAdapterRequest::new(
                "chat-rpc.rig.enriched",
                ProtocolFamily::Rpc,
                "agent.chat.create",
                "use Rig memory and knowledge",
            )
            .with_metadata("sdkwork.chat.provider_id", ids::MODEL_PROVIDER_ID)
            .with_metadata("sdkwork.memory.scope", "session")
            .with_metadata("sdkwork.memory.owner_context", "session.rig.chat")
            .with_metadata("sdkwork.memory.provider_id", ids::MEMORY_PROVIDER_ID)
            .with_metadata("sdkwork.knowledge.query", "Rig retrieval")
            .with_metadata("sdkwork.knowledge.provider_id", ids::KNOWLEDGE_PROVIDER_ID)
            .with_metadata("sdkwork.knowledge.namespace", "sdkwork.rig")
            .with_metadata("sdkwork.knowledge.methods", "hybrid")
            .with_metadata("sdkwork.knowledge.top_k", "1")
            .with_metadata("sdkwork.agent.session_id", "session.rig.chat")
            .with_metadata("sdkwork.agent.task_id", "task.rig.chat"),
        )
        .expect("Rig chat RPC adapter handles enriched chat request");

    assert_eq!(envelope.object_kind, ProtocolObjectKind::ExtensionObject);
    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert!(model_requests[0].tool_descriptors.is_empty());
    assert_eq!(model_requests[0].context_frames.len(), 2);
    assert_eq!(
        model_requests[0].context_frames[0].content,
        "Rig chat memory context"
    );
    assert_eq!(
        model_requests[0].context_frames[0].metadata_value("sdkwork.memory.record_id"),
        Some("memory.rig.chat.1")
    );
    assert_eq!(
        model_requests[0].context_frames[1].content,
        "Rig retrieval is exposed through SDKWork KnowledgeProvider; vector, keyword, graph, and wiki-style retrieval remain adapter details."
    );
    assert_eq!(
        model_requests[0].context_frames[1].metadata_value("sdkwork.knowledge.document_id"),
        Some("knowledge.rig.adapter")
    );
    assert_eq!(
        model_requests[0].context_frames[1].metadata_value("sdkwork.knowledge.retrieval_method"),
        Some("hybrid")
    );
    assert_eq!(
        model_requests[0].context_frames[1].metadata_value("sdkwork.adapter"),
        Some("rig-core")
    );
}

#[derive(Clone)]
struct RecordingRigChatModelProvider {
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl RecordingRigChatModelProvider {
    fn new(captured_requests: Arc<Mutex<Vec<ModelRequest>>>) -> Self {
        Self { captured_requests }
    }
}

impl ModelProvider for RecordingRigChatModelProvider {
    fn provider_manifest(&self) -> sdkwork_agent_kernel::ProviderManifest {
        sdkwork_agent_kernel::ProviderManifest::new(
            ids::MODEL_PROVIDER_ID,
            "model",
            "recording-rig-chat-model",
            "0.1.0",
            vec!["model.chat".to_string(), "model.catalog".to_string()],
        )
    }

    fn health(&self) -> sdkwork_agent_kernel::ProviderHealth {
        sdkwork_agent_kernel::ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> sdkwork_agent_kernel::KernelResult<ModelResponse> {
        self.captured_requests.lock().unwrap().push(request.clone());
        Ok(ModelResponse::text(
            request.model_request_id,
            ids::MODEL_PROVIDER_ID,
            "recorded Rig chat request",
        ))
    }
}
