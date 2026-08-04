use sdkwork_agent_kernel::{
    AgentChatRequest, AgentChatService, AgentManifest, KernelResult, KnowledgeDocument,
    KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider, KnowledgeRetrievalMethod,
    KnowledgeSearchRequest, ModelProvider, ModelRequest, ModelResponse, PolicyCategory,
    PolicyDecision, PolicyProvider, PolicyRequest, ProviderHealth, ProviderManifest,
    RedactionClassification, RuntimeBuilder, TrustLevel,
};
use sdkwork_agent_plugin_core::SdkworkKernelFoundationPlugin;
use sdkwork_kernel_plugin_knowledgebase::{
    sdkwork_knowledgebase_plugin_manifest, sdkwork_knowledgebase_provider_manifests,
    KnowledgebaseRetrievalClient, SdkworkKnowledgebasePlugin, SdkworkKnowledgebaseProvider,
    SDKWORK_KNOWLEDGEBASE_PLUGIN_ID, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID,
};
use sdkwork_knowledgebase_contract::{
    KnowledgeContextFragment, KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
    KnowledgeRetrievalTrace,
};
use std::sync::{Arc, Mutex};

#[test]
fn plugin_manifest_declares_optional_foundation_knowledge_provider() {
    let manifest = sdkwork_knowledgebase_plugin_manifest();

    assert_eq!(manifest.plugin_id, SDKWORK_KNOWLEDGEBASE_PLUGIN_ID);
    assert_eq!(manifest.implementation_kind, "official-foundation-plugin");
    assert_eq!(manifest.agent_id, None);
    assert_eq!(
        manifest.provider_ids,
        [SDKWORK_KNOWLEDGEBASE_PROVIDER_ID.to_string()]
    );
    assert!(manifest.supports_profile("provider-knowledge"));
}

#[test]
fn foundation_plugin_trait_exposes_provider_without_agent_manifest() {
    let plugin = SdkworkKnowledgebasePlugin::new();
    assert_foundation_plugin_trait(&plugin);

    assert_eq!(
        plugin.plugin_manifest().plugin_id,
        SDKWORK_KNOWLEDGEBASE_PLUGIN_ID
    );
    assert_eq!(
        plugin.provider_manifests()[0].provider_id,
        SDKWORK_KNOWLEDGEBASE_PROVIDER_ID
    );
    assert!(plugin.conformance_profile().requires("provider-knowledge"));
}

#[test]
fn provider_manifest_declares_standard_knowledge_capabilities() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_id, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID);
    assert_eq!(manifest.provider_family, "knowledge");
    assert!(manifest
        .capabilities
        .contains(&"knowledge.search".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"knowledge.read".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"knowledge.list".to_string()));
    assert_eq!(sdkwork_knowledgebase_provider_manifests(), vec![manifest]);
}

#[test]
fn search_maps_kernel_request_to_knowledgebase_retrieval_and_back() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let results = provider
        .search(
            KnowledgeSearchRequest::new("RAG boundary")
                .with_tenant_id("100001")
                .with_namespace("space:7")
                .with_top_k(3)
                .with_method(KnowledgeRetrievalMethod::Hybrid)
                .with_filter("document.kind", "spec")
                .with_metadata("sdkwork.knowledge.retrieval_profile_id", "31"),
        )
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].document_id, "301");
    assert_eq!(results[0].title, "RAG Boundary");
    assert_eq!(
        results[0].retrieval_method,
        KnowledgeRetrievalMethod::Hybrid
    );
    assert_eq!(results[0].score, Some(0.91));
    assert_eq!(results[0].trust_level, TrustLevel::TrustedHost);
    assert_eq!(
        results[0].redaction_classification,
        RedactionClassification::TenantSensitive
    );
    assert_eq!(
        results[0]
            .metadata
            .iter()
            .find(|(key, _)| key == "sdkwork.knowledge.space_id")
            .map(|(_, value)| value.as_str()),
        Some("7")
    );
    assert_eq!(
        results[0]
            .metadata
            .iter()
            .find(|(key, _)| key == "sdkwork.knowledge.chunk_id")
            .map(|(_, value)| value.as_str()),
        Some("201")
    );
}

#[test]
fn search_requires_namespace_space_id_to_preserve_scope() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let error = provider
        .search(KnowledgeSearchRequest::new("missing scope"))
        .unwrap_err();

    assert_eq!(error.code(), "validation_error");
}

#[test]
fn read_and_list_delegate_to_typed_client() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let document = provider.read("301").unwrap();
    let documents = provider
        .list(KnowledgeDocumentFilter::new().with_namespace("space:7"))
        .unwrap();

    assert_eq!(document.document_id, "301");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].namespace.as_deref(), Some("space:7"));
}

#[test]
fn official_knowledgebase_provider_enriches_agent_chat_context_when_selected() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let provider = SdkworkKnowledgebaseProvider::new(ChatKnowledgebaseClient, 100001);
    let manifest = AgentManifest::from_json(
        r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.knowledgebase.chat",
  "name": "knowledgebase-chat",
  "display_name": "Knowledgebase Chat",
  "description": "Agent used to verify optional knowledgebase provider chat enrichment.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    { "capability_id": "model.chat", "min_version": "0.1.0" },
    { "capability_id": "policy.evaluate", "min_version": "0.1.0" }
  ],
  "optional_capabilities": [
    { "capability_id": "knowledge.search", "min_version": "0.1.0" }
  ],
  "event_families": ["agent.model.*", "agent.knowledge.*"],
  "owner": { "name": "sdkwork-platform" },
  "status": "candidate"
}
"#,
    )
    .expect("knowledgebase chat manifest parses");

    let runtime = RuntimeBuilder::new("runtime.knowledgebase.chat", manifest)
        .register_model_provider(
            "provider.recording",
            "0.1.0",
            RecordingKnowledgeChatModelProvider::new(captured_model_requests.clone()),
        )
        .register_policy_provider(
            "provider.policy.allow",
            "0.1.0",
            AllowKnowledgeChatPolicyProvider,
        )
        .register_knowledge_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID, "0.1.0", provider)
        .bootstrap()
        .expect("knowledgebase chat runtime bootstraps")
        .runtime;

    let response = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat.knowledgebase.1",
                vec!["Summarize the RAG boundary.".to_string()],
            )
            .with_provider_id("provider.recording")
            .for_session("session.knowledgebase")
            .for_task("task.knowledgebase")
            .with_knowledge_query("RAG boundary")
            .with_knowledge_provider_id(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
            .with_knowledge_tenant_id("100001")
            .with_knowledge_namespace("space:7")
            .with_knowledge_top_k(2)
            .with_knowledge_method(KnowledgeRetrievalMethod::Hybrid),
        )
        .expect("knowledgebase-enriched chat succeeds");

    assert_eq!(response.provider_id, "provider.recording");
    let model_requests = captured_model_requests.lock().unwrap();
    assert_eq!(model_requests.len(), 1);
    assert_eq!(model_requests[0].context_frames.len(), 1);
    assert_eq!(
        model_requests[0].context_frames[0].content,
        "Knowledge retrieval is separate from model generation."
    );
    assert_eq!(model_requests[0].context_frames[0].source, "knowledge");
    assert_eq!(
        model_requests[0].context_frames[0].trust_level,
        TrustLevel::TrustedHost
    );
    assert_eq!(
        model_requests[0].context_frames[0].redaction_classification,
        RedactionClassification::TenantSensitive
    );
    assert_eq!(
        model_requests[0].context_frames[0].metadata_value("sdkwork.knowledge.document_id"),
        Some("301")
    );
    assert_eq!(
        model_requests[0].context_frames[0].metadata_value("sdkwork.knowledge.chunk_id"),
        Some("201")
    );
    assert_eq!(
        model_requests[0].context_frames[0].metadata_value("sdkwork.knowledge.retrieval_method"),
        Some("hybrid")
    );
}

#[test]
fn official_knowledgebase_chat_fails_closed_when_sensitive_context_policy_denies_model_send() {
    let captured_model_requests = Arc::new(Mutex::new(Vec::new()));
    let captured_policy_requests = Arc::new(Mutex::new(Vec::new()));
    let provider = SdkworkKnowledgebaseProvider::new(ChatKnowledgebaseClient, 100001);
    let manifest = AgentManifest::from_json(
        r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.knowledgebase.chat.policy",
  "name": "knowledgebase-chat-policy",
  "display_name": "Knowledgebase Chat Policy",
  "description": "Agent used to verify sensitive knowledge context is policy-gated before model send.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    { "capability_id": "model.chat", "min_version": "0.1.0" },
    { "capability_id": "policy.evaluate", "min_version": "0.1.0" }
  ],
  "optional_capabilities": [
    { "capability_id": "knowledge.search", "min_version": "0.1.0" }
  ],
  "event_families": ["agent.model.*", "agent.knowledge.*", "agent.policy.*"],
  "owner": { "name": "sdkwork-platform" },
  "status": "candidate"
}
"#,
    )
    .expect("knowledgebase chat policy manifest parses");

    let runtime = RuntimeBuilder::new("runtime.knowledgebase.chat.policy", manifest)
        .register_model_provider(
            "provider.recording",
            "0.1.0",
            RecordingKnowledgeChatModelProvider::new(captured_model_requests.clone()),
        )
        .register_policy_provider(
            "provider.policy.deny-sensitive",
            "0.1.0",
            DenySensitiveKnowledgeChatPolicyProvider::new(captured_policy_requests.clone()),
        )
        .register_knowledge_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID, "0.1.0", provider)
        .bootstrap()
        .expect("knowledgebase sensitive policy runtime bootstraps")
        .runtime;

    let error = AgentChatService::new()
        .invoke(
            &runtime,
            AgentChatRequest::new(
                "chat.knowledgebase.policy",
                vec!["Summarize the RAG boundary.".to_string()],
            )
            .with_provider_id("provider.recording")
            .for_session("session.knowledgebase.policy")
            .for_task("task.knowledgebase.policy")
            .with_knowledge_query("RAG boundary")
            .with_knowledge_provider_id(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
            .with_knowledge_tenant_id("100001")
            .with_knowledge_namespace("space:7")
            .with_knowledge_top_k(2)
            .with_knowledge_method(KnowledgeRetrievalMethod::Hybrid),
        )
        .expect_err("sensitive knowledge context policy blocks model send");

    assert_eq!(
        error.kind(),
        sdkwork_agent_kernel::KernelErrorKind::PolicyDenied
    );
    assert!(captured_model_requests.lock().unwrap().is_empty());
    let policy_requests = captured_policy_requests.lock().unwrap();
    assert!(policy_requests
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::KnowledgeSearch)));
    assert!(policy_requests
        .iter()
        .any(|request| request.typed_category == Some(PolicyCategory::ModelInvoke)));
    assert!(policy_requests.iter().any(|request| {
        request.typed_category == Some(PolicyCategory::ModelSendSensitiveContext)
            && request.resource == "model.default"
    }));
}

fn assert_foundation_plugin_trait<T: SdkworkKernelFoundationPlugin>(_plugin: &T) {}

struct FakeKnowledgebaseClient;

impl KnowledgebaseRetrievalClient for FakeKnowledgebaseClient {
    fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, String> {
        assert_eq!(request.tenant_id, 100001);
        assert_eq!(request.bindings[0].space_id, 7);
        assert_eq!(request.bindings[0].top_k, Some(3));
        assert_eq!(request.retrieval_profile_id, Some(31));
        assert!(request
            .metadata
            .iter()
            .any(|filter| filter.key == "document.kind" && filter.value == "spec"));

        Ok(KnowledgeRetrievalResult {
            retrieval_id: 101,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 103,
                status: "succeeded".to_string(),
                latency_ms: Some(20),
                result_count: 1,
            }),
            hits: vec![KnowledgeContextFragment {
                chunk_id: 201,
                document_id: 301,
                document_version_id: Some(401),
                space_id: 7,
                collection_id: None,
                title: "RAG Boundary".to_string(),
                content: "Knowledge retrieval is separate from model generation.".to_string(),
                score: Some(0.91),
                rank: 1,
                token_count: Some(8),
                retrieval_method: sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid,
                citation: None,
            }],
        })
    }

    fn read_document(&self, document_id: &str) -> Result<KnowledgeDocument, String> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            "Knowledge SPI",
            "KnowledgeProvider search/read/list.",
        )
        .with_namespace("space:7"))
    }

    fn list_documents(
        &self,
        _filter: KnowledgeDocumentFilter,
    ) -> Result<Vec<KnowledgeDocument>, String> {
        Ok(vec![KnowledgeDocument::new(
            "301",
            KnowledgeDocumentKind::Spec,
            "Knowledge SPI",
            "KnowledgeProvider search/read/list.",
        )
        .with_namespace("space:7")])
    }
}

struct ChatKnowledgebaseClient;

impl KnowledgebaseRetrievalClient for ChatKnowledgebaseClient {
    fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, String> {
        assert_eq!(request.tenant_id, 100001);
        assert_eq!(request.query, "RAG boundary");
        assert_eq!(request.bindings[0].space_id, 7);
        assert_eq!(request.bindings[0].top_k, Some(2));
        assert_eq!(
            request.methods,
            [sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid]
        );

        Ok(KnowledgeRetrievalResult {
            retrieval_id: 20101,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 20103,
                status: "succeeded".to_string(),
                latency_ms: Some(12),
                result_count: 1,
            }),
            hits: vec![KnowledgeContextFragment {
                chunk_id: 201,
                document_id: 301,
                document_version_id: Some(401),
                space_id: 7,
                collection_id: None,
                title: "RAG Boundary".to_string(),
                content: "Knowledge retrieval is separate from model generation.".to_string(),
                score: Some(0.91),
                rank: 1,
                token_count: Some(8),
                retrieval_method: sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid,
                citation: None,
            }],
        })
    }

    fn read_document(&self, document_id: &str) -> Result<KnowledgeDocument, String> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            "Knowledge SPI",
            "KnowledgeProvider search/read/list.",
        )
        .with_namespace("space:7"))
    }

    fn list_documents(
        &self,
        _filter: KnowledgeDocumentFilter,
    ) -> Result<Vec<KnowledgeDocument>, String> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct RecordingKnowledgeChatModelProvider {
    captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl RecordingKnowledgeChatModelProvider {
    fn new(captured_requests: Arc<Mutex<Vec<ModelRequest>>>) -> Self {
        Self { captured_requests }
    }
}

impl ModelProvider for RecordingKnowledgeChatModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.recording",
            "model",
            "recording-model",
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
            "provider.recording",
            "recorded",
        ))
    }
}

#[derive(Clone)]
struct AllowKnowledgeChatPolicyProvider;

impl PolicyProvider for AllowKnowledgeChatPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.allow",
            "policy",
            "allow-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            "provider.policy.allow",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Clone)]
struct DenySensitiveKnowledgeChatPolicyProvider {
    captured_requests: Arc<Mutex<Vec<PolicyRequest>>>,
}

impl DenySensitiveKnowledgeChatPolicyProvider {
    fn new(captured_requests: Arc<Mutex<Vec<PolicyRequest>>>) -> Self {
        Self { captured_requests }
    }
}

impl PolicyProvider for DenySensitiveKnowledgeChatPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.deny-sensitive",
            "policy",
            "deny-sensitive-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        self.captured_requests.lock().unwrap().push(request.clone());
        if request.typed_category == Some(PolicyCategory::ModelSendSensitiveContext) {
            Ok(PolicyDecision::deny(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                "provider.policy.deny-sensitive",
                "knowledge.sensitive_context.denied",
            ))
        } else {
            Ok(PolicyDecision::allow(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                "provider.policy.deny-sensitive",
            ))
        }
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
