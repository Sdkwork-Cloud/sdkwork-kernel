use sdkwork_agent_integration_core::SdkworkAgentIntegrationPlugin;
use sdkwork_agent_integration_rig::{
    ids, RigIntegrationPlugin, RigKnowledgeProvider, RigMemoryProvider, RigModelProvider,
    RigPlanningProvider, RigToolProvider,
};
use sdkwork_agent_kernel::{
    KernelErrorKind, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, MemoryProvider, MemoryRecord, MemoryScope,
    ModelProvider, ModelRequest, ModelResponseFormat, PlanningProvider, RedactionClassification,
    ToolCall, ToolProvider, TrustLevel,
};

#[test]
fn rig_model_provider_exposes_catalog_and_fails_closed_without_live_backend() {
    let provider = RigModelProvider::fail_closed();

    let manifest = provider.provider_manifest();
    assert_eq!(manifest.provider_id, ids::MODEL_PROVIDER_ID);
    assert!(manifest.capabilities.contains(&"model.catalog".to_string()));

    let models = provider.list_models();
    assert!(!models.is_empty());
    assert_eq!(models[0].provider_id, ids::MODEL_PROVIDER_ID);
    assert!(models[0].supports_capability("model.chat"));
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
fn rig_tool_provider_describes_policy_aware_tools() {
    let provider = RigToolProvider::fail_closed();
    let tools = provider.list_tools();

    assert!(!tools.is_empty());
    assert_eq!(tools[0].provider_id, ids::TOOL_PROVIDER_ID);
    assert!(tools[0].requires_policy());

    let request = tools[0].policy_request(
        "policy.tool.1",
        &ToolCall::new("tool.call.1", tools[0].tool_id.clone(), "{}"),
    );
    assert_eq!(request.category, "tool.invoke");
}

#[test]
fn rig_tool_invocation_fails_closed_without_live_backend() {
    let provider = RigToolProvider::fail_closed();
    let tool = provider.list_tools()[0].clone();

    let result = provider
        .invoke_tool(ToolCall::new("tool.call.1", tool.tool_id, "{}"))
        .expect("fail-closed tool calls return normalized denied result");

    assert_eq!(result.status, "denied");
    assert!(result.error.unwrap().contains("fail-closed"));
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
    let plan = sdkwork_agent_integration_rig::RigCoreKnowledgeAdapter::vector_search_plan(
        &KnowledgeSearchRequest::new("rig adapter").with_top_k(3),
    );

    assert_eq!(plan.query, "rig adapter");
    assert_eq!(plan.samples, 3);
}

#[test]
fn rig_planning_provider_creates_valid_policy_aware_plan() {
    let provider = RigPlanningProvider::new();
    let plan = provider.create_plan("task.1", "run.1", "summarize repository");

    assert_eq!(plan.task_id, "task.1");
    assert!(!plan.actions.is_empty());
    plan.validate().expect("rig plan is valid");
}

#[test]
fn rig_plugin_model_provider_can_be_selected_by_provider_id() {
    let plugin = RigIntegrationPlugin::fail_closed();
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
