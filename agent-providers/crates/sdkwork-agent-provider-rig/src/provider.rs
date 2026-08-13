use sdkwork_agent_kernel::{
    Action, ActionKind, KernelError, KernelResult, KnowledgeDocument, KnowledgeDocumentFilter,
    KnowledgeDocumentKind, KnowledgeProvider, KnowledgeRetrievalMethod, KnowledgeSearchRequest,
    KnowledgeSearchResult, McpPromptDescriptor, McpPromptMessage, McpProvider, McpResourceContent,
    McpResourceDescriptor, McpServerDescriptor, McpTransportKind, MemoryProvider, MemoryRecord,
    MemoryScope, ModelDescriptor, ModelProvider, ModelRequest, ModelResponse, ModelResponseFormat,
    Plan, PlanningProvider, PolicyCategory, PolicyDecision, PolicyProvider, PolicyRequest,
    ProviderHealth, ProviderManifest, RedactionClassification, SideEffectLevel, ToolCall,
    ToolDescriptor, ToolResult, TrustLevel,
};

#[cfg(feature = "rig-core-adapter")]
use crate::rig_core_adapter::RigCoreOpenAiExecutor;
use crate::{
    backend::{
        RigBackend, RigBackendBootstrapPlan, RigBackendConfig, RigBackendExecutionStatus,
        RigBackendExecutor,
    },
    ids,
};
#[cfg(feature = "rig-core-adapter")]
use sdkwork_agent_kernel::HostProvider;
use std::sync::Arc;

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

    pub fn with_backend_config(config: RigBackendConfig) -> Self {
        Self {
            backend: RigBackend::from_config(config),
        }
    }

    pub fn with_executor(config: RigBackendConfig, executor: Arc<dyn RigBackendExecutor>) -> Self {
        Self {
            backend: RigBackend::with_executor(config, executor),
        }
    }

    #[cfg(feature = "rig-core-adapter")]
    pub fn with_rig_core_openai(
        config: RigBackendConfig,
        host: Arc<dyn HostProvider + Send + Sync>,
        default_model_id: impl Into<String>,
    ) -> KernelResult<Self> {
        if config.mode != crate::backend::RigBackendMode::Live {
            return Err(KernelError::validation(
                "Rig official adapter requires live backend mode",
            ));
        }
        // Any non-cloudrouter vendor id (openai, deepseek, qwen, custom …) is
        // a direct OpenAI-compatible provider; only the explicit cloudrouter
        // provider id routes through the account-pool gateway instead.
        if config.provider_id.as_deref() == Some("cloudrouter") {
            return Err(KernelError::validation(
                "Rig OpenAI-compatible adapter cannot target the cloudrouter gateway; \
                 use the cloud router executor for provider_id=cloudrouter",
            ));
        }
        let secret_ref = config.api_key_secret_ref.clone().ok_or_else(|| {
            KernelError::validation(
                "Rig official adapter requires llm.rig.api_key secret reference",
            )
        })?;
        Ok(Self::with_executor(
            config.clone(),
            Arc::new(RigCoreOpenAiExecutor::new(
                host,
                secret_ref,
                default_model_id,
                config.base_url,
            )?),
        ))
    }

    pub fn backend_execution_status(&self) -> RigBackendExecutionStatus {
        self.backend.execution_status()
    }

    pub fn backend_bootstrap_plan(&self) -> RigBackendBootstrapPlan {
        self.backend.bootstrap_plan()
    }
}

fn fail_closed_health() -> ProviderHealth {
    ProviderHealth {
        status: "degraded".to_string(),
    }
}

impl ModelProvider for RigModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::MODEL_PROVIDER_ID,
            "model",
            "rig-rust",
            "0.1.0",
            vec!["model.catalog".to_string(), "model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        if self.backend.execution_status().fail_closed {
            fail_closed_health()
        } else {
            ProviderHealth::available()
        }
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        let backend_status = self.backend.execution_status();
        let bootstrap_plan = self.backend.bootstrap_plan();
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
        .with_metadata("sdkwork.backend.default_mode", backend_status.mode.as_str())
        .with_metadata("sdkwork.backend.mode", backend_status.mode.as_str())
        .with_metadata(
            "sdkwork.backend.execution_state",
            backend_status.state.as_str(),
        )
        .with_metadata(
            "sdkwork.backend.fail_closed",
            if backend_status.fail_closed {
                "true"
            } else {
                "false"
            },
        )
        .with_metadata("sdkwork.backend.safe_reason", backend_status.safe_reason)
        .with_metadata(
            "sdkwork.backend.bootstrap_state",
            bootstrap_plan.state.as_str(),
        )
        .with_metadata(
            "sdkwork.backend.required_secret_refs",
            bootstrap_plan.required_secret_refs.join(","),
        )
        .with_metadata(
            "sdkwork.backend.policy_categories",
            bootstrap_plan.policy_categories.join(","),
        )
        .with_metadata("sdkwork.backend.safe_summary", bootstrap_plan.safe_summary)
        .with_metadata(
            "sdkwork.backend.provider_id",
            bootstrap_plan.provider_id.unwrap_or_default(),
        )]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.backend.invoke_model(request)
    }
}

#[derive(Debug, Clone)]
pub struct RigMcpProvider {
    backend: RigBackend,
}

impl RigMcpProvider {
    pub fn fail_closed() -> Self {
        Self {
            backend: RigBackend::fail_closed(),
        }
    }

    pub fn with_backend_config(config: RigBackendConfig) -> Self {
        Self {
            backend: RigBackend::from_config(config),
        }
    }

    pub fn backend_execution_status(&self) -> RigBackendExecutionStatus {
        self.backend.execution_status()
    }

    pub fn backend_bootstrap_plan(&self) -> RigBackendBootstrapPlan {
        self.backend.bootstrap_plan()
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::MCP_PROVIDER_ID,
            "mcp",
            "rig-rust-mcp",
            "0.1.0",
            vec!["mcp.resources".to_string(), "mcp.prompts".to_string()],
        )
    }

    fn ensure_server(server_id: &str) -> KernelResult<()> {
        if server_id == ids::DEFAULT_MCP_SERVER_ID {
            Ok(())
        } else {
            Err(KernelError::CapabilityMissing {
                capability_id: format!("mcp.server.{server_id}"),
            })
        }
    }
}

impl McpProvider for RigMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        RigMcpProvider::provider_manifest(self)
    }

    fn health(&self) -> ProviderHealth {
        fail_closed_health()
    }

    fn list_servers(&self) -> KernelResult<Vec<McpServerDescriptor>> {
        Ok(vec![McpServerDescriptor::new(
            ids::DEFAULT_MCP_SERVER_ID,
            ids::MCP_PROVIDER_ID,
            McpTransportKind::Sse,
        )
        .with_capability("mcp.resources")
        .with_capability("mcp.prompts")])
    }

    fn list_tools(&self, server_id: &str) -> KernelResult<Vec<ToolDescriptor>> {
        Self::ensure_server(server_id)?;
        Ok(Vec::new())
    }

    fn invoke_tool(&self, server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        Self::ensure_server(server_id)?;
        Err(KernelError::CapabilityMissing {
            capability_id: call.tool_id,
        })
    }

    fn list_resources(&self, server_id: &str) -> KernelResult<Vec<McpResourceDescriptor>> {
        Self::ensure_server(server_id)?;
        Ok(vec![McpResourceDescriptor::new(
            ids::DEFAULT_MCP_RESOURCE_URI,
            "Rig Knowledge Adapter",
            "text/plain",
        )
        .with_description(
            "Provider-neutral Rig knowledge adapter reference.",
        )])
    }

    fn read_resource(&self, server_id: &str, uri: &str) -> KernelResult<McpResourceContent> {
        Self::ensure_server(server_id)?;
        if uri != ids::DEFAULT_MCP_RESOURCE_URI {
            return Err(KernelError::CapabilityMissing {
                capability_id: uri.to_string(),
            });
        }

        let document = rig_knowledge_adapter_document();
        let mut content = McpResourceContent::new(
            ids::DEFAULT_MCP_RESOURCE_URI,
            document.content_type.clone(),
            document.content.clone(),
        )
        .with_trust_level(document.trust_level)
        .with_redaction_classification(document.redaction_classification)
        .with_metadata("sdkwork.knowledge.document_id", document.document_id)
        .with_metadata("sdkwork.knowledge.kind", document.kind.as_str())
        .with_metadata("sdkwork.knowledge.title", document.title);

        if let Some(namespace) = document.namespace {
            content = content.with_metadata("sdkwork.knowledge.namespace", namespace);
        }

        if let Some(source_uri) = document.source_uri {
            content = content.with_metadata("sdkwork.knowledge.source_uri", source_uri);
        }

        for (key, value) in document.metadata {
            content = content.with_metadata(key, value);
        }

        Ok(content)
    }

    fn list_prompts(&self, server_id: &str) -> KernelResult<Vec<McpPromptDescriptor>> {
        Self::ensure_server(server_id)?;
        Ok(vec![McpPromptDescriptor::new(
            ids::DEFAULT_MCP_PROMPT_ID,
            "Rig Chat Prompt",
        )
        .with_description("Prompt scaffold for Rig-backed SDKWork chat execution.")
        .with_argument("input")])
    }

    fn get_prompt(
        &self,
        server_id: &str,
        prompt_id: &str,
        _arguments: Vec<(String, String)>,
    ) -> KernelResult<McpPromptMessage> {
        Self::ensure_server(server_id)?;
        if prompt_id != ids::DEFAULT_MCP_PROMPT_ID {
            return Err(KernelError::CapabilityMissing {
                capability_id: prompt_id.to_string(),
            });
        }

        Ok(McpPromptMessage::new(
            ids::DEFAULT_MCP_PROMPT_ID,
            vec![
                "Use SDKWork policy, knowledge, memory, and tool provider boundaries when planning Rig chat execution."
                    .to_string(),
            ],
        )
        .with_trust_level(TrustLevel::TrustedHost)
        .with_redaction_classification(RedactionClassification::Internal)
        .with_metadata("sdkwork.adapter", "rig-core")
        .with_metadata("sdkwork.prompt.kind", "chat"))
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

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::POLICY_PROVIDER_ID,
            "policy",
            "rig-local-conformance-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }
}

impl PolicyProvider for RigPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        RigPolicyProvider::provider_manifest(self)
    }

    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        if requires_local_approval(&request) {
            return Ok(PolicyDecision::needs_approval(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                ids::POLICY_PROVIDER_ID,
                "rig.local_conformance.approval_required",
            )
            .with_safe_reason(
                "Rig local conformance policy requires approval for side-effectful actions",
            )
            .require_audit());
        }

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

fn requires_local_approval(request: &PolicyRequest) -> bool {
    if matches!(
        request.side_effect_level,
        Some(
            SideEffectLevel::SideEffectful
                | SideEffectLevel::Destructive
                | SideEffectLevel::Privileged
        )
    ) {
        return true;
    }

    matches!(
        request.category.as_str(),
        "model.send_sensitive_context"
            | "tool.invoke"
            | "tool.external_send"
            | "host.secrets.read"
            | "host.filesystem.write"
            | "host.process.execute"
            | "host.network.connect"
            | "memory.write"
            | "memory.delete"
            | "artifact.write"
            | "protocol.send"
            | "agent.install"
            | "agent.uninstall"
            | "agent.upgrade"
            | "agent.configure"
            | "provider.register"
            | "provider.configure"
    )
}

impl PlanningProvider for RigPlanningProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        RigPlanningProvider::provider_manifest(self)
    }

    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> KernelResult<Plan> {
        Ok(
            Plan::new("plan.rig.runtime", task_id, run_id, summary).add_action(
                Action::new(
                    "action.model.invoke",
                    ActionKind::ModelCall,
                    "invoke Rig model provider",
                )
                .with_required_capabilities(vec!["model.chat".to_string()])
                .with_side_effect_level(SideEffectLevel::ExternalSend)
                .with_policy_categories(vec![PolicyCategory::ModelInvoke.as_str().to_string()]),
            ),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
