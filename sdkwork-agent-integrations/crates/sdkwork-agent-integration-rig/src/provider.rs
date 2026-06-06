use sdkwork_agent_kernel::{
    Action, ActionKind, KernelResult, KnowledgeDocument, KnowledgeDocumentFilter,
    KnowledgeDocumentKind, KnowledgeProvider, KnowledgeRetrievalMethod, KnowledgeSearchRequest,
    KnowledgeSearchResult, MemoryProvider, MemoryRecord, MemoryScope, ModelDescriptor,
    ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat, Plan, PlanningProvider,
    PolicyCategory, PolicyDecision, PolicyProvider, PolicyRequest, ProviderHealth,
    ProviderManifest, RedactionClassification, SideEffectLevel, ToolCall, ToolDescriptor,
    ToolProvider, ToolResult, TrustLevel,
};

use crate::{backend::RigBackend, ids};

#[derive(Debug, Clone)]
pub struct RigModelProvider {
    backend: RigBackend,
}

impl RigModelProvider {
    pub fn fail_closed() -> Self {
        Self {
            backend: RigBackend::fail_closed(),
        }
    }
}

impl ModelProvider for RigModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::MODEL_PROVIDER_ID,
            "model",
            "rig-rust",
            "0.1.0",
            vec![
                "model.catalog".to_string(),
                "model.chat".to_string(),
                "model.streaming".to_string(),
                "model.tool_call".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ids::DEFAULT_MODEL_ID,
            ids::MODEL_PROVIDER_ID,
            "Rig Default Chat",
            "rig",
        )
        .with_capability("model.chat")
        .with_capability("model.catalog")
        .with_response_format(ModelResponseFormat::Text)
        .with_input_mode("text")
        .with_output_mode("text")
        .with_policy_category(PolicyCategory::ModelInvoke.as_str())
        .with_metadata("sdkwork.backend.default_mode", "fail_closed")]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.backend.invoke_model(request)
    }
}

#[derive(Debug, Clone)]
pub struct RigToolProvider {
    backend: RigBackend,
}

impl RigToolProvider {
    pub fn fail_closed() -> Self {
        Self {
            backend: RigBackend::fail_closed(),
        }
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::TOOL_PROVIDER_ID,
            "tool",
            "rig-rust-tools",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }
}

impl ToolProvider for RigToolProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        RigToolProvider::provider_manifest(self)
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            ids::DEFAULT_TOOL_ID,
            ids::TOOL_PROVIDER_ID,
            "Rig Tool Bridge",
            SideEffectLevel::SideEffectful,
        )
        .with_policy_categories(vec![PolicyCategory::ToolInvoke.as_str().to_string()])
        .require_audit()]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(self.backend.invoke_tool(call))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RigMemoryProvider {
    records: Vec<MemoryRecord>,
}

impl RigMemoryProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::MEMORY_PROVIDER_ID,
            "memory",
            "rig-rust-memory",
            "0.1.0",
            vec![
                "memory.query".to_string(),
                "memory.write".to_string(),
                "memory.delete".to_string(),
                "memory.export".to_string(),
            ],
        )
    }
}

impl MemoryProvider for RigMemoryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        RigMemoryProvider::provider_manifest(self)
    }

    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
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

#[derive(Debug, Clone)]
pub struct RigKnowledgeProvider {
    documents: Vec<KnowledgeDocument>,
}

impl RigKnowledgeProvider {
    pub fn new() -> Self {
        Self {
            documents: vec![rig_knowledge_adapter_document()],
        }
    }

    pub fn with_document(mut self, document: KnowledgeDocument) -> Self {
        self.documents
            .retain(|existing| existing.document_id != document.document_id);
        self.documents.push(document);
        self
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::KNOWLEDGE_PROVIDER_ID,
            "knowledge",
            "rig-rust-knowledge",
            "0.1.0",
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )
    }
}

impl Default for RigKnowledgeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeProvider for RigKnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        RigKnowledgeProvider::provider_manifest(self)
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        let query = request.query.to_ascii_lowercase();
        let methods = if request.methods.is_empty() {
            vec![KnowledgeRetrievalMethod::Hybrid]
        } else {
            request.methods.clone()
        };

        let mut results: Vec<_> = self
            .documents
            .iter()
            .filter(|document| {
                request
                    .namespace
                    .as_deref()
                    .is_none_or(|namespace| document.namespace.as_deref() == Some(namespace))
            })
            .filter(|document| {
                request.include_external
                    || document.kind != KnowledgeDocumentKind::ExternalReference
            })
            .filter(|document| {
                request
                    .filters
                    .iter()
                    .all(|(key, value)| knowledge_document_matches_filter(document, key, value))
            })
            .filter(|document| {
                query.is_empty()
                    || document.title.to_ascii_lowercase().contains(&query)
                    || document.content.to_ascii_lowercase().contains(&query)
                    || document
                        .tags
                        .iter()
                        .any(|tag| tag.to_ascii_lowercase().contains(&query))
            })
            .map(|document| {
                let method = document
                    .retrieval_methods
                    .iter()
                    .copied()
                    .find(|method| methods.contains(method))
                    .unwrap_or(methods[0]);
                KnowledgeSearchResult::new(
                    document.document_id.clone(),
                    document.kind,
                    document.title.clone(),
                    method,
                )
                .with_snippet(document.content.clone())
                .with_score(1.0)
                .with_optional_source_uri(document.source_uri.clone())
                .with_trust_level(document.trust_level)
                .with_redaction_classification(document.redaction_classification)
                .with_document_metadata(document.metadata.clone())
            })
            .collect();

        results.truncate(request.top_k);
        Ok(results)
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        self.documents
            .iter()
            .find(|document| document.document_id == document_id)
            .cloned()
            .ok_or_else(|| sdkwork_agent_kernel::KernelError::CapabilityMissing {
                capability_id: format!("knowledge.read.{document_id}"),
            })
    }

    fn list(&self, filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(self
            .documents
            .iter()
            .filter(|document| filter.matches(document))
            .cloned()
            .collect())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn knowledge_document_matches_filter(document: &KnowledgeDocument, key: &str, value: &str) -> bool {
    match key {
        "document_id" => document.document_id == value,
        "kind" | "doc.kind" | "document_kind" => document.kind.as_str() == value,
        "namespace" => document.namespace.as_deref() == Some(value),
        "tag" | "tags" => document.tags.iter().any(|tag| tag == value),
        "retrieval_method" | "method" => document
            .retrieval_methods
            .iter()
            .any(|method| method.as_str() == value),
        "source_uri" => document.source_uri.as_deref() == Some(value),
        _ => document
            .metadata
            .iter()
            .any(|(metadata_key, metadata_value)| metadata_key == key && metadata_value == value),
    }
}

trait KnowledgeSearchResultExt {
    fn with_optional_source_uri(self, source_uri: Option<String>) -> Self;
    fn with_document_metadata(self, metadata: Vec<(String, String)>) -> Self;
}

impl KnowledgeSearchResultExt for KnowledgeSearchResult {
    fn with_optional_source_uri(self, source_uri: Option<String>) -> Self {
        if let Some(source_uri) = source_uri {
            self.with_source_uri(source_uri)
        } else {
            self
        }
    }

    fn with_document_metadata(mut self, metadata: Vec<(String, String)>) -> Self {
        self.metadata.extend(metadata);
        self
    }
}

fn rig_knowledge_adapter_document() -> KnowledgeDocument {
    KnowledgeDocument::new(
        "knowledge.rig.adapter",
        KnowledgeDocumentKind::WikiSection,
        "Rig Knowledge Adapter",
        "Rig retrieval is exposed through SDKWork KnowledgeProvider; vector, keyword, graph, and wiki-style retrieval remain adapter details.",
    )
    .with_namespace("sdkwork.rig")
    .with_source_uri("external/rig")
    .with_tag("rig")
    .with_tag("knowledge")
    .with_retrieval_method(KnowledgeRetrievalMethod::Keyword)
    .with_retrieval_method(KnowledgeRetrievalMethod::Hybrid)
    .with_retrieval_method(KnowledgeRetrievalMethod::Vector)
    .with_trust_level(TrustLevel::TrustedHost)
    .with_redaction_classification(RedactionClassification::Internal)
    .with_metadata("sdkwork.adapter", "rig-core")
}

#[derive(Debug, Clone, Default)]
pub struct RigPlanningProvider;

impl RigPlanningProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::PLANNING_PROVIDER_ID,
            "planning",
            "rig-rust-planning",
            "0.1.0",
            vec!["planning.create".to_string()],
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct RigPolicyProvider;

impl RigPolicyProvider {
    pub fn new() -> Self {
        Self
    }
}

impl PolicyProvider for RigPolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            ids::POLICY_PROVIDER_ID,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

impl PlanningProvider for RigPlanningProvider {
    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> Plan {
        Plan::new("plan.rig.runtime", task_id, run_id, summary).add_action(
            Action::new(
                "action.model.invoke",
                ActionKind::ModelCall,
                "invoke Rig model provider",
            )
            .with_required_capabilities(vec!["model.chat".to_string()])
            .with_side_effect_level(SideEffectLevel::ExternalSend)
            .with_policy_categories(vec![PolicyCategory::ModelInvoke.as_str().to_string()]),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
