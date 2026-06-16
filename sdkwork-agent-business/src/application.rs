use crate::domain::{
    AgentAuditAction, AgentBusinessRecord, AgentBusinessStatus, AgentDeploymentRecord,
    AgentDeploymentStatus, AgentImplementationKind, AgentImplementationType,
    AgentKnowledgeBaseKind, AgentKnowledgeBaseRecord, AgentKnowledgeBindingRecord,
    AgentKnowledgeBindingScopeKind, AgentKnowledgeChunkRecord, AgentKnowledgeDocumentKind,
    AgentKnowledgeDocumentRecord, AgentKnowledgeIndexKind, AgentKnowledgeIndexRecord,
    AgentKnowledgeSearchResult, AgentKnowledgeSourceKind, AgentKnowledgeSourceRecord,
    AgentKnowledgeSyncJobKind, AgentKnowledgeSyncJobRecord, AgentKnowledgeSyncJobStatus,
    AgentMcpAuthKind, AgentMcpServerRecord, AgentMcpTransportKind, AgentMemoryBindingRecord,
    AgentMemoryBindingScopeKind, AgentMemoryIndexKind, AgentMemoryNamespaceKind,
    AgentMemoryNamespaceRecord, AgentMemoryProfileRecord, AgentMemoryRecord, AgentMemoryRecordKind,
    AgentMemoryRelationKind, AgentMemoryRelationRecord, AgentMemoryRetrievalIndexRecord,
    AgentMemorySourceKind, AgentMemorySourceRecord, AgentMemoryStoreKind, AgentMemoryStoreRecord,
    AgentPromptTemplateFormat, AgentPromptTemplateKind, AgentPromptTemplateRecord,
    AgentProviderBindingRecord, AgentRuntimeExecutionOperation, AgentRuntimeExecutionRecord,
    AgentRuntimeExecutionStatus, AgentSkillInvocationKind, AgentSkillPackageRecord,
    AgentVisibility, DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY,
};
use crate::ports::{AgentAuditSink, AgentListQuery, AgentMarketplaceListQuery, AgentRepository};
use crate::validation::{validate_capabilities, validate_standard_id};
use sdkwork_agent_kernel::{
    AgentManifest, KernelError, KernelEvent, KernelEventRedaction, KernelEventSeverity,
    KernelEventSource, KernelResult, PolicyCategory, PolicyDecisionValue, PolicyProvider,
    PolicyRequest, PolicySubject,
};
use sdkwork_code_kernel::CodeTaskIntent;
use std::{cmp::Ordering, collections::HashSet};

const MAX_KNOWLEDGE_SEARCH_QUERY_CHARS: usize = 4096;
const MAX_KNOWLEDGE_SEARCH_TOP_K: usize = 100;
const MAX_KNOWLEDGE_REFERENCE_CHARS: usize = 1024;
const MAX_KNOWLEDGE_HASH_CHARS: usize = 128;
const MAX_KNOWLEDGE_HEADING_CHARS: usize = 512;
const MAX_KNOWLEDGE_SCOPE_REF_CHARS: usize = 128;
const MAX_KNOWLEDGE_REDACTION_CLASSIFICATION_CHARS: usize = 64;
const MAX_MEMORY_SCOPE_REF_CHARS: usize = 128;

struct AgentBusinessAuditEventInput<'a> {
    action: AgentAuditAction,
    item_kind: &'a str,
    tenant_id: u64,
    organization_id: u64,
    item_id: &'a str,
    status: AgentBusinessStatus,
    visibility: AgentVisibility,
    version: u64,
    subject: PolicySubject,
    occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAgentCommand {
    pub agent_id: String,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub manifest: AgentManifest,
    pub visibility: AgentVisibility,
    pub tags: Vec<String>,
    pub default_code_task_intent: Option<CodeTaskIntent>,
    pub implementation_provider_id: Option<String>,
    pub implementation_kind: Option<AgentImplementationKind>,
    pub implementation_type: Option<AgentImplementationType>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAgentCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub expected_version: Option<u64>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub manifest: Option<AgentManifest>,
    pub visibility: Option<AgentVisibility>,
    pub tags: Option<Vec<String>>,
    pub default_code_task_intent: Option<CodeTaskIntent>,
    pub implementation_provider_id: Option<Option<String>>,
    pub implementation_kind: Option<Option<AgentImplementationKind>>,
    pub implementation_type: Option<AgentImplementationType>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeAgentStatusCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub expected_version: Option<u64>,
    pub target_status: AgentBusinessStatus,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteAgentCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub expected_version: Option<u64>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreAgentCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub expected_version: Option<u64>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAgentCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub requested_by: PolicySubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAgentsCommand {
    pub query: AgentListQuery,
    pub requested_by: PolicySubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub binding_id: String,
    pub provider_id: String,
    pub implementation_kind: AgentImplementationKind,
    pub configuration_profile_id: String,
    pub capabilities: Vec<String>,
    pub make_default: bool,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivateAgentProviderBindingCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub binding_id: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderDeploymentCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub deployment_id: String,
    pub binding_id: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentPreviewResponseCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub execution_id: String,
    pub content: String,
    pub debug_mode: bool,
    pub memory_enabled: bool,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub input_payload_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPromptOptimizationCommand {
    pub tenant_id: u64,
    pub agent_id: String,
    pub execution_id: String,
    pub prompt: String,
    pub input_payload_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillPackageCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub skill_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub invocation_kind: AgentSkillInvocationKind,
    pub package_ref: String,
    pub entrypoint: String,
    pub input_schema_json: String,
    pub output_schema_json: String,
    pub capability_ids: Vec<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub security_profile_id: Option<String>,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSkillPackageUpdateCommand {
    pub tenant_id: u64,
    pub skill_id: String,
    pub expected_version: Option<u64>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub invocation_kind: Option<AgentSkillInvocationKind>,
    pub package_ref: Option<String>,
    pub entrypoint: Option<String>,
    pub input_schema_json: Option<String>,
    pub output_schema_json: Option<String>,
    pub capability_ids: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub security_profile_id: Option<String>,
    pub visibility: Option<AgentVisibility>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMcpServerCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub mcp_server_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub protocol_version: String,
    pub transport_kind: AgentMcpTransportKind,
    pub endpoint_ref: Option<String>,
    pub command_ref: Option<String>,
    pub auth_kind: AgentMcpAuthKind,
    pub auth_profile_id: Option<String>,
    pub capability_ids: Vec<String>,
    pub tool_count: u32,
    pub resource_count: u32,
    pub prompt_count: u32,
    pub capabilities_json: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub security_profile_id: Option<String>,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMcpServerUpdateCommand {
    pub tenant_id: u64,
    pub mcp_server_id: String,
    pub expected_version: Option<u64>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub protocol_version: Option<String>,
    pub transport_kind: Option<AgentMcpTransportKind>,
    pub endpoint_ref: Option<Option<String>>,
    pub command_ref: Option<Option<String>>,
    pub auth_kind: Option<AgentMcpAuthKind>,
    pub auth_profile_id: Option<Option<String>>,
    pub capability_ids: Option<Vec<String>>,
    pub tool_count: Option<u32>,
    pub resource_count: Option<u32>,
    pub prompt_count: Option<u32>,
    pub capabilities_json: Option<String>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub security_profile_id: Option<Option<String>>,
    pub visibility: Option<AgentVisibility>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPromptTemplateCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub prompt_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub prompt_kind: AgentPromptTemplateKind,
    pub template_format: AgentPromptTemplateFormat,
    pub template_body: String,
    pub variables_schema_json: String,
    pub model_constraints_json: String,
    pub capability_ids: Vec<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub safety_profile_id: Option<String>,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPromptTemplateUpdateCommand {
    pub tenant_id: u64,
    pub prompt_id: String,
    pub expected_version: Option<u64>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub prompt_kind: Option<AgentPromptTemplateKind>,
    pub template_format: Option<AgentPromptTemplateFormat>,
    pub template_body: Option<String>,
    pub variables_schema_json: Option<String>,
    pub model_constraints_json: Option<String>,
    pub capability_ids: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub safety_profile_id: Option<String>,
    pub visibility: Option<AgentVisibility>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAgentMarketplaceItemCommand {
    pub tenant_id: u64,
    pub item_id: String,
    pub requested_by: PolicySubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteAgentMarketplaceItemCommand {
    pub tenant_id: u64,
    pub item_id: String,
    pub expected_version: Option<u64>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreAgentMarketplaceItemCommand {
    pub tenant_id: u64,
    pub item_id: String,
    pub expected_version: Option<u64>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBaseCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub knowledge_base_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub provider_id: String,
    pub base_kind: AgentKnowledgeBaseKind,
    pub retrieval_modes: Vec<AgentKnowledgeIndexKind>,
    pub capability_ids: Vec<String>,
    pub configuration_profile_id: String,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBaseUpdateCommand {
    pub tenant_id: u64,
    pub knowledge_base_id: String,
    pub expected_version: Option<u64>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub provider_id: Option<String>,
    pub base_kind: Option<AgentKnowledgeBaseKind>,
    pub retrieval_modes: Option<Vec<AgentKnowledgeIndexKind>>,
    pub capability_ids: Option<Vec<String>>,
    pub configuration_profile_id: Option<String>,
    pub visibility: Option<AgentVisibility>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSourceCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub knowledge_source_id: String,
    pub knowledge_base_id: String,
    pub source_kind: AgentKnowledgeSourceKind,
    pub source_ref: String,
    pub source_hash: String,
    pub sync_policy_json: String,
    pub metadata_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSourceUpdateCommand {
    pub tenant_id: u64,
    pub knowledge_source_id: String,
    pub expected_version: Option<u64>,
    pub source_kind: Option<AgentKnowledgeSourceKind>,
    pub source_ref: Option<String>,
    pub source_hash: Option<String>,
    pub sync_policy_json: Option<String>,
    pub metadata_json: Option<String>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeDocumentCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub knowledge_document_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub document_kind: AgentKnowledgeDocumentKind,
    pub title: String,
    pub content_ref: String,
    pub content_hash: String,
    pub summary: Option<String>,
    pub metadata_json: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub trust_level: i16,
    pub redaction_classification: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeDocumentUpdateCommand {
    pub tenant_id: u64,
    pub knowledge_document_id: String,
    pub expected_version: Option<u64>,
    pub knowledge_source_id: Option<String>,
    pub document_kind: Option<AgentKnowledgeDocumentKind>,
    pub title: Option<String>,
    pub content_ref: Option<String>,
    pub content_hash: Option<String>,
    pub summary: Option<String>,
    pub metadata_json: Option<String>,
    pub tags: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub trust_level: Option<i16>,
    pub redaction_classification: Option<String>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeChunkCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub knowledge_chunk_id: String,
    pub knowledge_document_id: String,
    pub parent_chunk_id: Option<String>,
    pub chunk_ordinal: u32,
    pub heading: Option<String>,
    pub content_ref: String,
    pub content_hash: String,
    pub token_estimate: u32,
    pub summary: Option<String>,
    pub metadata_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeIndexUpsertCommand {
    pub tenant_id: u64,
    pub knowledge_index_id: String,
    pub knowledge_base_id: String,
    pub knowledge_document_id: Option<String>,
    pub knowledge_chunk_id: Option<String>,
    pub index_kind: AgentKnowledgeIndexKind,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeListCommand {
    pub tenant_id: u64,
    pub knowledge_base_id: String,
    pub requested_by: PolicySubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeReadCommand {
    pub tenant_id: u64,
    pub knowledge_document_id: String,
    pub requested_by: PolicySubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBindingCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub knowledge_binding_id: String,
    pub knowledge_base_id: String,
    pub agent_id: Option<String>,
    pub deployment_id: Option<String>,
    pub scope_kind: AgentKnowledgeBindingScopeKind,
    pub scope_ref: String,
    pub active: bool,
    pub default_binding: bool,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub sync_job_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub job_kind: AgentKnowledgeSyncJobKind,
    pub input_ref: String,
    pub input_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobStartCommand {
    pub tenant_id: u64,
    pub sync_job_id: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobCompleteCommand {
    pub tenant_id: u64,
    pub sync_job_id: String,
    pub output_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobFailCommand {
    pub tenant_id: u64,
    pub sync_job_id: String,
    pub error_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobCancelCommand {
    pub tenant_id: u64,
    pub sync_job_id: String,
    pub cancellation_json: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSearchCommand {
    pub tenant_id: u64,
    pub knowledge_base_id: String,
    pub query: String,
    pub top_k: usize,
    pub retrieval_modes: Vec<AgentKnowledgeIndexKind>,
    pub include_external: bool,
    pub requested_by: PolicySubject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryStoreCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub memory_store_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub provider_id: String,
    pub store_kind: AgentMemoryStoreKind,
    pub retrieval_modes: Vec<AgentMemoryIndexKind>,
    pub capability_ids: Vec<String>,
    pub configuration_profile_id: String,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryStoreUpdateCommand {
    pub tenant_id: u64,
    pub memory_store_id: String,
    pub expected_version: Option<u64>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub provider_id: Option<String>,
    pub store_kind: Option<AgentMemoryStoreKind>,
    pub retrieval_modes: Option<Vec<AgentMemoryIndexKind>>,
    pub capability_ids: Option<Vec<String>>,
    pub configuration_profile_id: Option<String>,
    pub visibility: Option<AgentVisibility>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryProfileCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub memory_profile_id: String,
    pub memory_store_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub write_policy_json: String,
    pub retrieval_policy_json: String,
    pub compaction_policy_json: String,
    pub retention_policy_json: String,
    pub privacy_policy_json: String,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryBindingCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub memory_binding_id: String,
    pub memory_profile_id: String,
    pub agent_id: Option<String>,
    pub deployment_id: Option<String>,
    pub scope_kind: AgentMemoryBindingScopeKind,
    pub scope_ref: String,
    pub active: bool,
    pub default_binding: bool,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryNamespaceCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub user_ref: Option<String>,
    pub session_ref: Option<String>,
    pub thread_ref: Option<String>,
    pub namespace_kind: AgentMemoryNamespaceKind,
    pub visibility: AgentVisibility,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRecordCreateCommand {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub memory_id: String,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub memory_kind: AgentMemoryRecordKind,
    pub content_format: String,
    pub content_json: String,
    pub summary: Option<String>,
    pub salience_score: f32,
    pub confidence_score: f32,
    pub freshness_score: f32,
    pub sensitivity_level: i16,
    pub effective_at: Option<String>,
    pub expires_at: Option<String>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemorySourceCreateCommand {
    pub tenant_id: u64,
    pub memory_source_id: String,
    pub memory_id: String,
    pub source_kind: AgentMemorySourceKind,
    pub source_ref: String,
    pub source_hash: String,
    pub evidence_json: String,
    pub captured_at: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRelationCreateCommand {
    pub tenant_id: u64,
    pub memory_relation_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation_kind: AgentMemoryRelationKind,
    pub weight: f32,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryRetrievalIndexUpsertCommand {
    pub tenant_id: u64,
    pub memory_index_id: String,
    pub memory_id: String,
    pub index_kind: AgentMemoryIndexKind,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub requested_by: PolicySubject,
    pub requested_at: String,
}

pub struct AgentBusinessService<R, A, P>
where
    R: AgentRepository,
    A: AgentAuditSink,
    P: PolicyProvider,
{
    repository: R,
    audit_sink: A,
    policy_provider: P,
}

impl<R, A, P> AgentBusinessService<R, A, P>
where
    R: AgentRepository,
    A: AgentAuditSink,
    P: PolicyProvider,
{
    pub fn new(repository: R, audit_sink: A, policy_provider: P) -> Self {
        Self {
            repository,
            audit_sink,
            policy_provider,
        }
    }

    pub fn create_agent(
        &mut self,
        command: CreateAgentCommand,
    ) -> KernelResult<AgentBusinessRecord> {
        validate_agent_id(command.agent_id.as_str())?;

        let policy_resource = format!("agent.business.{}", command.agent_id);
        self.authorize(
            "agent.business.create",
            command.requested_by.clone(),
            policy_resource,
            "create",
        )?;

        if self
            .repository
            .get(command.tenant_id, command.agent_id.as_str())
            .is_some()
        {
            return Err(KernelError::conflict("agent already exists"));
        }

        if command.code.trim().is_empty() {
            return Err(KernelError::validation("agent code is required"));
        }
        if command.display_name.trim().is_empty() {
            return Err(KernelError::validation("agent display_name is required"));
        }
        if let Some(provider_id) = command.implementation_provider_id.as_deref() {
            validate_standard_id(provider_id, "implementationProviderId", Some("provider."))?;
        }

        let mut record = AgentBusinessRecord {
            id: self.repository.next_id()?,
            agent_id: command.agent_id,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            manifest: command.manifest,
            default_code_task_intent: command.default_code_task_intent,
            implementation_provider_id: command.implementation_provider_id,
            implementation_kind: command.implementation_kind,
            implementation_type: command.implementation_type.unwrap_or_default(),
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            tags: command.tags,
            version: 0,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        record.mark_updated(command.requested_at.clone());

        self.repository.insert(record.clone())?;
        self.emit_audit_event(
            AgentAuditAction::Create,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn add_provider_binding(
        &mut self,
        command: AgentProviderBindingCommand,
    ) -> KernelResult<AgentProviderBindingRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        self.authorize(
            "agent.business.provider_binding.add",
            command.requested_by.clone(),
            format!("agent.business.{}", command.agent_id),
            "provider_binding.add",
        )?;

        self.repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;

        validate_standard_id(command.binding_id.as_str(), "bindingId", Some("binding."))?;
        validate_standard_id(
            command.provider_id.as_str(),
            "providerId",
            Some("provider."),
        )?;
        validate_standard_id(
            command.configuration_profile_id.as_str(),
            "configurationProfileId",
            Some("profile."),
        )?;
        validate_capabilities(command.capabilities.as_slice(), "capabilities")?;

        if self
            .repository
            .get_provider_binding(
                command.tenant_id,
                command.agent_id.as_str(),
                command.binding_id.as_str(),
            )
            .is_some()
        {
            return Err(KernelError::conflict(
                "agent provider binding already exists",
            ));
        }

        if command.make_default {
            self.deactivate_provider_bindings(
                command.tenant_id,
                command.agent_id.as_str(),
                command.requested_at.clone(),
            )?;
        }

        let record = AgentProviderBindingRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            agent_id: command.agent_id.clone(),
            binding_id: command.binding_id,
            provider_id: command.provider_id,
            implementation_kind: command.implementation_kind,
            configuration_profile_id: command.configuration_profile_id,
            capabilities: command.capabilities,
            active: command.make_default,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
        };

        self.repository.insert_provider_binding(record.clone())?;
        self.emit_binding_audit_event(
            AgentAuditAction::ProviderBindingChanged,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn activate_provider_binding(
        &mut self,
        command: ActivateAgentProviderBindingCommand,
    ) -> KernelResult<AgentProviderBindingRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        self.authorize(
            "agent.business.provider_binding.activate",
            command.requested_by.clone(),
            format!("agent.business.{}", command.agent_id),
            "provider_binding.activate",
        )?;

        self.repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;
        validate_standard_id(command.binding_id.as_str(), "bindingId", Some("binding."))?;

        let mut record = self
            .repository
            .get_provider_binding(
                command.tenant_id,
                command.agent_id.as_str(),
                command.binding_id.as_str(),
            )
            .ok_or_else(|| KernelError::validation("agent provider binding not found"))?;

        if record.active {
            return Ok(record);
        }

        self.deactivate_provider_bindings(
            command.tenant_id,
            command.agent_id.as_str(),
            command.requested_at.clone(),
        )?;
        record.active = true;
        record.mark_updated(command.requested_at.clone());
        self.repository.update_provider_binding(record.clone())?;
        self.emit_binding_audit_event(
            AgentAuditAction::ProviderBindingChanged,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn list_provider_bindings(
        &mut self,
        tenant_id: u64,
        agent_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentProviderBindingRecord>> {
        validate_agent_id(agent_id)?;
        self.authorize(
            "agent.business.provider_binding.list",
            requested_by,
            format!("agent.business.{}", agent_id),
            "provider_binding.list",
        )?;
        self.repository
            .get(tenant_id, agent_id)
            .ok_or_else(|| KernelError::validation("agent not found"))?;
        Ok(self.repository.list_provider_bindings(tenant_id, agent_id))
    }

    pub fn create_deployment(
        &mut self,
        command: AgentProviderDeploymentCommand,
    ) -> KernelResult<AgentDeploymentRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        self.authorize(
            "agent.business.deployment.create",
            command.requested_by.clone(),
            format!("agent.business.{}", command.agent_id),
            "deployment.create",
        )?;

        self.repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;
        validate_standard_id(
            command.deployment_id.as_str(),
            "deploymentId",
            Some("deployment."),
        )?;
        validate_standard_id(command.binding_id.as_str(), "bindingId", Some("binding."))?;

        let binding = self
            .repository
            .get_provider_binding(
                command.tenant_id,
                command.agent_id.as_str(),
                command.binding_id.as_str(),
            )
            .ok_or_else(|| KernelError::validation("agent provider binding not found"))?;

        let record = AgentDeploymentRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            agent_id: command.agent_id,
            deployment_id: command.deployment_id,
            binding_id: binding.binding_id,
            provider_id_snapshot: binding.provider_id,
            implementation_kind_snapshot: binding.implementation_kind,
            configuration_profile_id_snapshot: binding.configuration_profile_id,
            capabilities_snapshot: binding.capabilities,
            status: AgentDeploymentStatus::Created,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
        };

        self.repository.insert_deployment(record.clone())?;
        self.emit_deployment_audit_event(
            AgentAuditAction::DeploymentCreated,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn list_deployments(
        &mut self,
        tenant_id: u64,
        agent_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentDeploymentRecord>> {
        validate_agent_id(agent_id)?;
        self.authorize(
            "agent.business.deployment.list",
            requested_by,
            format!("agent.business.{}", agent_id),
            "deployment.list",
        )?;
        self.repository
            .get(tenant_id, agent_id)
            .ok_or_else(|| KernelError::validation("agent not found"))?;
        Ok(self.repository.list_deployments(tenant_id, agent_id))
    }

    pub fn create_preview_response(
        &mut self,
        command: AgentPreviewResponseCommand,
    ) -> KernelResult<AgentRuntimeExecutionRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        self.authorize(
            "agent.business.runtime.preview_response",
            command.requested_by.clone(),
            format!("agent.business.{}", command.agent_id),
            "runtime.preview_response",
        )?;

        self.repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;
        validate_standard_id(
            command.execution_id.as_str(),
            "executionId",
            Some("execution."),
        )?;
        validate_non_empty(command.content.as_str(), "content")?;
        validate_json_payload(command.input_payload_json.as_str(), "inputPayload")?;
        if let Some(model) = command.model.as_deref() {
            validate_optional_plain_ref(Some(model), "model")?;
        }
        if let Some(temperature) = command.temperature {
            if !(0.0..=2.0).contains(&temperature) || !temperature.is_finite() {
                return Err(KernelError::validation(
                    "temperature must be between 0 and 2",
                ));
            }
        }

        let output_payload_json = serde_json::json!({
            "content": command.content,
            "debugMode": command.debug_mode,
            "memoryEnabled": command.memory_enabled,
            "model": command.model,
            "temperature": command.temperature,
            "runtimeMode": "deterministic-local-contract"
        })
        .to_string();

        let record = AgentRuntimeExecutionRecord {
            tenant_id: command.tenant_id,
            agent_id: command.agent_id,
            execution_id: command.execution_id,
            operation: AgentRuntimeExecutionOperation::PreviewResponse,
            status: AgentRuntimeExecutionStatus::Completed,
            input_payload_json: command.input_payload_json,
            output_payload_json,
            requested_at: command.requested_at.clone(),
            completed_at: command.requested_at.clone(),
        };

        self.emit_runtime_execution_audit_event(
            AgentAuditAction::RuntimeExecutionCompleted,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn create_prompt_optimization(
        &mut self,
        command: AgentPromptOptimizationCommand,
    ) -> KernelResult<AgentRuntimeExecutionRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        self.authorize(
            "agent.business.runtime.prompt_optimization",
            command.requested_by.clone(),
            format!("agent.business.{}", command.agent_id),
            "runtime.prompt_optimization",
        )?;

        self.repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;
        validate_standard_id(
            command.execution_id.as_str(),
            "executionId",
            Some("execution."),
        )?;
        validate_non_empty(command.prompt.as_str(), "prompt")?;
        validate_json_payload(command.input_payload_json.as_str(), "inputPayload")?;

        let optimized_prompt = normalize_prompt_text(command.prompt.as_str());
        let output_payload_json = serde_json::json!({
            "optimizedPrompt": optimized_prompt,
            "runtimeMode": "deterministic-local-contract"
        })
        .to_string();

        let record = AgentRuntimeExecutionRecord {
            tenant_id: command.tenant_id,
            agent_id: command.agent_id,
            execution_id: command.execution_id,
            operation: AgentRuntimeExecutionOperation::PromptOptimization,
            status: AgentRuntimeExecutionStatus::Completed,
            input_payload_json: command.input_payload_json,
            output_payload_json,
            requested_at: command.requested_at.clone(),
            completed_at: command.requested_at.clone(),
        };

        self.emit_runtime_execution_audit_event(
            AgentAuditAction::RuntimeExecutionCompleted,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn create_skill_package(
        &mut self,
        command: AgentSkillPackageCreateCommand,
    ) -> KernelResult<AgentSkillPackageRecord> {
        self.authorize(
            "agent.business.skill.create",
            command.requested_by.clone(),
            format!("agent.business.skill.{}", command.skill_id),
            "skill.create",
        )?;

        validate_marketplace_identity(
            command.skill_id.as_str(),
            "skillId",
            Some("skill."),
            command.code.as_str(),
            command.display_name.as_str(),
        )?;
        validate_marketplace_json(command.input_schema_json.as_str(), "inputSchemaJson")?;
        validate_marketplace_json(command.output_schema_json.as_str(), "outputSchemaJson")?;
        validate_capabilities(command.capability_ids.as_slice(), "capabilityIds")?;
        validate_marketplace_labels(command.categories.as_slice(), "categories")?;
        validate_marketplace_labels(command.tags.as_slice(), "tags")?;
        validate_optional_standard_ref(
            command.security_profile_id.as_deref(),
            "securityProfileId",
            "profile.",
        )?;
        validate_non_empty(command.package_ref.as_str(), "packageRef")?;
        validate_non_empty(command.entrypoint.as_str(), "entrypoint")?;
        reject_secret_material(command.package_ref.as_str(), "packageRef")?;
        reject_secret_material(command.entrypoint.as_str(), "entrypoint")?;

        if self
            .repository
            .get_skill_package(command.tenant_id, command.skill_id.as_str())
            .is_some()
        {
            return Err(KernelError::conflict("agent skill package already exists"));
        }

        let record = AgentSkillPackageRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            skill_id: command.skill_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            invocation_kind: command.invocation_kind,
            package_ref: command.package_ref,
            entrypoint: command.entrypoint,
            input_schema_json: command.input_schema_json,
            output_schema_json: command.output_schema_json,
            capability_ids: command.capability_ids,
            categories: command.categories,
            tags: command.tags,
            security_profile_id: command.security_profile_id,
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };

        self.repository.insert_skill_package(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::SkillPackageCreated,
            item_kind: "skill",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.skill_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn update_skill_package(
        &mut self,
        command: AgentSkillPackageUpdateCommand,
    ) -> KernelResult<AgentSkillPackageRecord> {
        self.authorize(
            "agent.business.skill.update",
            command.requested_by.clone(),
            format!("agent.business.skill.{}", command.skill_id),
            "skill.update",
        )?;
        validate_standard_id(command.skill_id.as_str(), "skillId", Some("skill."))?;
        let mut record = self
            .repository
            .get_skill_package(command.tenant_id, command.skill_id.as_str())
            .ok_or_else(|| KernelError::validation("agent skill package not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent skill package",
        )?;

        if let Some(display_name) = command.display_name {
            validate_non_empty(display_name.as_str(), "displayName")?;
            record.display_name = display_name;
        }
        if let Some(description) = command.description {
            record.description = Some(description);
        }
        if let Some(invocation_kind) = command.invocation_kind {
            record.invocation_kind = invocation_kind;
        }
        if let Some(package_ref) = command.package_ref {
            validate_non_empty(package_ref.as_str(), "packageRef")?;
            reject_secret_material(package_ref.as_str(), "packageRef")?;
            record.package_ref = package_ref;
        }
        if let Some(entrypoint) = command.entrypoint {
            validate_non_empty(entrypoint.as_str(), "entrypoint")?;
            reject_secret_material(entrypoint.as_str(), "entrypoint")?;
            record.entrypoint = entrypoint;
        }
        if let Some(input_schema_json) = command.input_schema_json {
            validate_marketplace_json(input_schema_json.as_str(), "inputSchemaJson")?;
            record.input_schema_json = input_schema_json;
        }
        if let Some(output_schema_json) = command.output_schema_json {
            validate_marketplace_json(output_schema_json.as_str(), "outputSchemaJson")?;
            record.output_schema_json = output_schema_json;
        }
        if let Some(capability_ids) = command.capability_ids {
            validate_capabilities(capability_ids.as_slice(), "capabilityIds")?;
            record.capability_ids = capability_ids;
        }
        if let Some(categories) = command.categories {
            validate_marketplace_labels(categories.as_slice(), "categories")?;
            record.categories = categories;
        }
        if let Some(tags) = command.tags {
            validate_marketplace_labels(tags.as_slice(), "tags")?;
            record.tags = tags;
        }
        if let Some(security_profile_id) = command.security_profile_id {
            validate_standard_id(
                security_profile_id.as_str(),
                "securityProfileId",
                Some("profile."),
            )?;
            record.security_profile_id = Some(security_profile_id);
        }
        if let Some(visibility) = command.visibility {
            record.visibility = visibility;
        }
        record.mark_updated(command.requested_at.clone());

        self.repository.update_skill_package(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::SkillPackageUpdated,
            item_kind: "skill",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.skill_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_skill_package(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentSkillPackageRecord> {
        self.authorize(
            "agent.business.skill.retrieve",
            command.requested_by,
            format!("agent.business.skill.{}", command.item_id),
            "skill.retrieve",
        )?;
        validate_standard_id(command.item_id.as_str(), "skillId", Some("skill."))?;
        self.repository
            .get_skill_package(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent skill package not found"))
    }

    pub fn list_skill_packages(
        &mut self,
        query: AgentMarketplaceListQuery,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentSkillPackageRecord>> {
        self.authorize(
            "agent.business.skill.list",
            requested_by,
            format!("agent.business.skill.tenant.{}", query.tenant_id),
            "skill.list",
        )?;
        Ok(self.repository.list_skill_packages(&query))
    }

    pub fn delete_skill_package(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentSkillPackageRecord> {
        self.authorize(
            "agent.business.skill.delete",
            command.requested_by.clone(),
            format!("agent.business.skill.{}", command.item_id),
            "skill.delete",
        )?;
        validate_standard_id(command.item_id.as_str(), "skillId", Some("skill."))?;
        let mut record = self
            .repository
            .get_skill_package(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent skill package not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent skill package",
        )?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_skill_package(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::SkillPackageDeleted,
            item_kind: "skill",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.skill_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_skill_package(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentSkillPackageRecord> {
        self.authorize(
            "agent.business.skill.restore",
            command.requested_by.clone(),
            format!("agent.business.skill.{}", command.item_id),
            "skill.restore",
        )?;
        validate_standard_id(command.item_id.as_str(), "skillId", Some("skill."))?;
        let mut record = self
            .repository
            .get_skill_package(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent skill package not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent skill package",
        )?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_skill_package(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::SkillPackageRestored,
            item_kind: "skill",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.skill_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_mcp_server(
        &mut self,
        command: AgentMcpServerCreateCommand,
    ) -> KernelResult<AgentMcpServerRecord> {
        self.authorize(
            "agent.business.mcp.create",
            command.requested_by.clone(),
            format!("agent.business.mcp.{}", command.mcp_server_id),
            "mcp.create",
        )?;

        validate_marketplace_identity(
            command.mcp_server_id.as_str(),
            "mcpServerId",
            Some("mcp.server."),
            command.code.as_str(),
            command.display_name.as_str(),
        )?;
        validate_non_empty(command.protocol_version.as_str(), "protocolVersion")?;
        validate_optional_standard_ref(
            command.endpoint_ref.as_deref(),
            "endpointRef",
            "endpoint.",
        )?;
        validate_optional_standard_ref(command.command_ref.as_deref(), "commandRef", "command.")?;
        validate_optional_standard_ref(
            command.auth_profile_id.as_deref(),
            "authProfileId",
            "profile.",
        )?;
        validate_optional_standard_ref(
            command.security_profile_id.as_deref(),
            "securityProfileId",
            "profile.",
        )?;
        validate_capabilities(command.capability_ids.as_slice(), "capabilityIds")?;
        validate_marketplace_json(command.capabilities_json.as_str(), "capabilitiesJson")?;
        validate_marketplace_labels(command.categories.as_slice(), "categories")?;
        validate_marketplace_labels(command.tags.as_slice(), "tags")?;
        validate_mcp_transport_reference_pair(
            command.transport_kind,
            command.endpoint_ref.as_deref(),
            command.command_ref.as_deref(),
        )?;

        if self
            .repository
            .get_mcp_server(command.tenant_id, command.mcp_server_id.as_str())
            .is_some()
        {
            return Err(KernelError::conflict("agent mcp server already exists"));
        }

        let record = AgentMcpServerRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            mcp_server_id: command.mcp_server_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            protocol_version: command.protocol_version,
            transport_kind: command.transport_kind,
            endpoint_ref: command.endpoint_ref,
            command_ref: command.command_ref,
            auth_kind: command.auth_kind,
            auth_profile_id: command.auth_profile_id,
            capability_ids: command.capability_ids,
            tool_count: command.tool_count,
            resource_count: command.resource_count,
            prompt_count: command.prompt_count,
            capabilities_json: command.capabilities_json,
            categories: command.categories,
            tags: command.tags,
            security_profile_id: command.security_profile_id,
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };

        self.repository.insert_mcp_server(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::McpServerCreated,
            item_kind: "mcp",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.mcp_server_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn update_mcp_server(
        &mut self,
        command: AgentMcpServerUpdateCommand,
    ) -> KernelResult<AgentMcpServerRecord> {
        self.authorize(
            "agent.business.mcp.update",
            command.requested_by.clone(),
            format!("agent.business.mcp.{}", command.mcp_server_id),
            "mcp.update",
        )?;
        validate_standard_id(
            command.mcp_server_id.as_str(),
            "mcpServerId",
            Some("mcp.server."),
        )?;
        let mut record = self
            .repository
            .get_mcp_server(command.tenant_id, command.mcp_server_id.as_str())
            .ok_or_else(|| KernelError::validation("agent mcp server not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent mcp server",
        )?;

        if let Some(display_name) = command.display_name {
            validate_non_empty(display_name.as_str(), "displayName")?;
            record.display_name = display_name;
        }
        if let Some(description) = command.description {
            record.description = Some(description);
        }
        if let Some(protocol_version) = command.protocol_version {
            validate_non_empty(protocol_version.as_str(), "protocolVersion")?;
            record.protocol_version = protocol_version;
        }
        if let Some(transport_kind) = command.transport_kind {
            record.transport_kind = transport_kind;
        }
        if let Some(endpoint_ref) = command.endpoint_ref {
            validate_optional_standard_ref(endpoint_ref.as_deref(), "endpointRef", "endpoint.")?;
            record.endpoint_ref = endpoint_ref;
        }
        if let Some(command_ref) = command.command_ref {
            validate_optional_standard_ref(command_ref.as_deref(), "commandRef", "command.")?;
            record.command_ref = command_ref;
        }
        if let Some(auth_kind) = command.auth_kind {
            record.auth_kind = auth_kind;
        }
        if let Some(auth_profile_id) = command.auth_profile_id {
            validate_optional_standard_ref(
                auth_profile_id.as_deref(),
                "authProfileId",
                "profile.",
            )?;
            record.auth_profile_id = auth_profile_id;
        }
        if let Some(capability_ids) = command.capability_ids {
            validate_capabilities(capability_ids.as_slice(), "capabilityIds")?;
            record.capability_ids = capability_ids;
        }
        if let Some(tool_count) = command.tool_count {
            record.tool_count = tool_count;
        }
        if let Some(resource_count) = command.resource_count {
            record.resource_count = resource_count;
        }
        if let Some(prompt_count) = command.prompt_count {
            record.prompt_count = prompt_count;
        }
        if let Some(capabilities_json) = command.capabilities_json {
            validate_marketplace_json(capabilities_json.as_str(), "capabilitiesJson")?;
            record.capabilities_json = capabilities_json;
        }
        if let Some(categories) = command.categories {
            validate_marketplace_labels(categories.as_slice(), "categories")?;
            record.categories = categories;
        }
        if let Some(tags) = command.tags {
            validate_marketplace_labels(tags.as_slice(), "tags")?;
            record.tags = tags;
        }
        if let Some(security_profile_id) = command.security_profile_id {
            validate_optional_standard_ref(
                security_profile_id.as_deref(),
                "securityProfileId",
                "profile.",
            )?;
            record.security_profile_id = security_profile_id;
        }
        if let Some(visibility) = command.visibility {
            record.visibility = visibility;
        }
        validate_mcp_transport_reference_pair(
            record.transport_kind,
            record.endpoint_ref.as_deref(),
            record.command_ref.as_deref(),
        )?;
        record.mark_updated(command.requested_at.clone());

        self.repository.update_mcp_server(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::McpServerUpdated,
            item_kind: "mcp",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.mcp_server_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_mcp_server(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMcpServerRecord> {
        self.authorize(
            "agent.business.mcp.retrieve",
            command.requested_by,
            format!("agent.business.mcp.{}", command.item_id),
            "mcp.retrieve",
        )?;
        validate_standard_id(command.item_id.as_str(), "mcpServerId", Some("mcp.server."))?;
        self.repository
            .get_mcp_server(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent mcp server not found"))
    }

    pub fn list_mcp_servers(
        &mut self,
        query: AgentMarketplaceListQuery,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentMcpServerRecord>> {
        self.authorize(
            "agent.business.mcp.list",
            requested_by,
            format!("agent.business.mcp.tenant.{}", query.tenant_id),
            "mcp.list",
        )?;
        Ok(self.repository.list_mcp_servers(&query))
    }

    pub fn delete_mcp_server(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMcpServerRecord> {
        self.authorize(
            "agent.business.mcp.delete",
            command.requested_by.clone(),
            format!("agent.business.mcp.{}", command.item_id),
            "mcp.delete",
        )?;
        validate_standard_id(command.item_id.as_str(), "mcpServerId", Some("mcp.server."))?;
        let mut record = self
            .repository
            .get_mcp_server(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent mcp server not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent mcp server",
        )?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_mcp_server(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::McpServerDeleted,
            item_kind: "mcp",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.mcp_server_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_mcp_server(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMcpServerRecord> {
        self.authorize(
            "agent.business.mcp.restore",
            command.requested_by.clone(),
            format!("agent.business.mcp.{}", command.item_id),
            "mcp.restore",
        )?;
        validate_standard_id(command.item_id.as_str(), "mcpServerId", Some("mcp.server."))?;
        let mut record = self
            .repository
            .get_mcp_server(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent mcp server not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent mcp server",
        )?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_mcp_server(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::McpServerRestored,
            item_kind: "mcp",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.mcp_server_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_prompt_template(
        &mut self,
        command: AgentPromptTemplateCreateCommand,
    ) -> KernelResult<AgentPromptTemplateRecord> {
        self.authorize(
            "agent.business.prompt.create",
            command.requested_by.clone(),
            format!("agent.business.prompt.{}", command.prompt_id),
            "prompt.create",
        )?;

        validate_marketplace_identity(
            command.prompt_id.as_str(),
            "promptId",
            Some("prompt."),
            command.code.as_str(),
            command.display_name.as_str(),
        )?;
        validate_non_empty(command.template_body.as_str(), "templateBody")?;
        reject_secret_material(command.template_body.as_str(), "templateBody")?;
        validate_marketplace_json(
            command.variables_schema_json.as_str(),
            "variablesSchemaJson",
        )?;
        validate_marketplace_json(
            command.model_constraints_json.as_str(),
            "modelConstraintsJson",
        )?;
        validate_capabilities(command.capability_ids.as_slice(), "capabilityIds")?;
        validate_marketplace_labels(command.categories.as_slice(), "categories")?;
        validate_marketplace_labels(command.tags.as_slice(), "tags")?;
        validate_optional_standard_ref(
            command.safety_profile_id.as_deref(),
            "safetyProfileId",
            "profile.",
        )?;

        if self
            .repository
            .get_prompt_template(command.tenant_id, command.prompt_id.as_str())
            .is_some()
        {
            return Err(KernelError::conflict(
                "agent prompt template already exists",
            ));
        }

        let record = AgentPromptTemplateRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            prompt_id: command.prompt_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            prompt_kind: command.prompt_kind,
            template_format: command.template_format,
            template_body: command.template_body,
            variables_schema_json: command.variables_schema_json,
            model_constraints_json: command.model_constraints_json,
            capability_ids: command.capability_ids,
            categories: command.categories,
            tags: command.tags,
            safety_profile_id: command.safety_profile_id,
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };

        self.repository.insert_prompt_template(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::PromptTemplateCreated,
            item_kind: "prompt",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.prompt_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn update_prompt_template(
        &mut self,
        command: AgentPromptTemplateUpdateCommand,
    ) -> KernelResult<AgentPromptTemplateRecord> {
        self.authorize(
            "agent.business.prompt.update",
            command.requested_by.clone(),
            format!("agent.business.prompt.{}", command.prompt_id),
            "prompt.update",
        )?;
        validate_standard_id(command.prompt_id.as_str(), "promptId", Some("prompt."))?;
        let mut record = self
            .repository
            .get_prompt_template(command.tenant_id, command.prompt_id.as_str())
            .ok_or_else(|| KernelError::validation("agent prompt template not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent prompt template",
        )?;

        if let Some(display_name) = command.display_name {
            validate_non_empty(display_name.as_str(), "displayName")?;
            record.display_name = display_name;
        }
        if let Some(description) = command.description {
            record.description = Some(description);
        }
        if let Some(prompt_kind) = command.prompt_kind {
            record.prompt_kind = prompt_kind;
        }
        if let Some(template_format) = command.template_format {
            record.template_format = template_format;
        }
        if let Some(template_body) = command.template_body {
            validate_non_empty(template_body.as_str(), "templateBody")?;
            reject_secret_material(template_body.as_str(), "templateBody")?;
            record.template_body = template_body;
        }
        if let Some(variables_schema_json) = command.variables_schema_json {
            validate_marketplace_json(variables_schema_json.as_str(), "variablesSchemaJson")?;
            record.variables_schema_json = variables_schema_json;
        }
        if let Some(model_constraints_json) = command.model_constraints_json {
            validate_marketplace_json(model_constraints_json.as_str(), "modelConstraintsJson")?;
            record.model_constraints_json = model_constraints_json;
        }
        if let Some(capability_ids) = command.capability_ids {
            validate_capabilities(capability_ids.as_slice(), "capabilityIds")?;
            record.capability_ids = capability_ids;
        }
        if let Some(categories) = command.categories {
            validate_marketplace_labels(categories.as_slice(), "categories")?;
            record.categories = categories;
        }
        if let Some(tags) = command.tags {
            validate_marketplace_labels(tags.as_slice(), "tags")?;
            record.tags = tags;
        }
        if let Some(safety_profile_id) = command.safety_profile_id {
            validate_standard_id(
                safety_profile_id.as_str(),
                "safetyProfileId",
                Some("profile."),
            )?;
            record.safety_profile_id = Some(safety_profile_id);
        }
        if let Some(visibility) = command.visibility {
            record.visibility = visibility;
        }
        record.mark_updated(command.requested_at.clone());

        self.repository.update_prompt_template(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::PromptTemplateUpdated,
            item_kind: "prompt",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.prompt_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_prompt_template(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentPromptTemplateRecord> {
        self.authorize(
            "agent.business.prompt.retrieve",
            command.requested_by,
            format!("agent.business.prompt.{}", command.item_id),
            "prompt.retrieve",
        )?;
        validate_standard_id(command.item_id.as_str(), "promptId", Some("prompt."))?;
        self.repository
            .get_prompt_template(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent prompt template not found"))
    }

    pub fn list_prompt_templates(
        &mut self,
        query: AgentMarketplaceListQuery,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentPromptTemplateRecord>> {
        self.authorize(
            "agent.business.prompt.list",
            requested_by,
            format!("agent.business.prompt.tenant.{}", query.tenant_id),
            "prompt.list",
        )?;
        Ok(self.repository.list_prompt_templates(&query))
    }

    pub fn delete_prompt_template(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentPromptTemplateRecord> {
        self.authorize(
            "agent.business.prompt.delete",
            command.requested_by.clone(),
            format!("agent.business.prompt.{}", command.item_id),
            "prompt.delete",
        )?;
        validate_standard_id(command.item_id.as_str(), "promptId", Some("prompt."))?;
        let mut record = self
            .repository
            .get_prompt_template(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent prompt template not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent prompt template",
        )?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_prompt_template(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::PromptTemplateDeleted,
            item_kind: "prompt",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.prompt_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_prompt_template(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentPromptTemplateRecord> {
        self.authorize(
            "agent.business.prompt.restore",
            command.requested_by.clone(),
            format!("agent.business.prompt.{}", command.item_id),
            "prompt.restore",
        )?;
        validate_standard_id(command.item_id.as_str(), "promptId", Some("prompt."))?;
        let mut record = self
            .repository
            .get_prompt_template(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent prompt template not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent prompt template",
        )?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_prompt_template(record.clone())?;
        self.emit_marketplace_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::PromptTemplateRestored,
            item_kind: "prompt",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.prompt_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_knowledge_base(
        &mut self,
        command: AgentKnowledgeBaseCreateCommand,
    ) -> KernelResult<AgentKnowledgeBaseRecord> {
        self.authorize(
            "agent.business.knowledge.base.create",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.base.{}",
                command.knowledge_base_id
            ),
            "knowledge.base.create",
        )?;
        validate_marketplace_identity(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
            command.code.as_str(),
            command.display_name.as_str(),
        )?;
        validate_standard_id(
            command.provider_id.as_str(),
            "providerId",
            Some("provider."),
        )?;
        validate_standard_id(
            command.configuration_profile_id.as_str(),
            "configurationProfileId",
            Some("profile."),
        )?;
        validate_capabilities(command.capability_ids.as_slice(), "capabilityIds")?;
        validate_non_empty_knowledge_modes(command.retrieval_modes.as_slice())?;

        if self
            .repository
            .get_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())
            .is_some()
        {
            return Err(KernelError::conflict("agent knowledge base already exists"));
        }

        let record = AgentKnowledgeBaseRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            knowledge_base_id: command.knowledge_base_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            provider_id: command.provider_id,
            base_kind: command.base_kind,
            retrieval_modes: command.retrieval_modes,
            capability_ids: command.capability_ids,
            configuration_profile_id: command.configuration_profile_id,
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        self.repository.insert_knowledge_base(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeBaseCreated,
            item_kind: "knowledge_base",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_base_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_bases(
        &mut self,
        query: AgentMarketplaceListQuery,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeBaseRecord>> {
        self.authorize(
            "agent.business.knowledge.base.list",
            requested_by,
            format!("agent.business.knowledge.base.tenant.{}", query.tenant_id),
            "knowledge.base.list",
        )?;
        Ok(self.repository.list_knowledge_bases(&query))
    }

    pub fn get_knowledge_base(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeBaseRecord> {
        self.authorize(
            "agent.business.knowledge.base.retrieve",
            command.requested_by,
            format!("agent.business.knowledge.base.{}", command.item_id),
            "knowledge.base.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        self.get_active_knowledge_base(command.tenant_id, command.item_id.as_str())
    }

    pub fn update_knowledge_base(
        &mut self,
        command: AgentKnowledgeBaseUpdateCommand,
    ) -> KernelResult<AgentKnowledgeBaseRecord> {
        self.authorize(
            "agent.business.knowledge.base.update",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.base.{}",
                command.knowledge_base_id
            ),
            "knowledge.base.update",
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge base not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge base",
        )?;

        if let Some(display_name) = command.display_name {
            validate_non_empty(display_name.as_str(), "displayName")?;
            record.display_name = display_name;
        }
        if let Some(description) = command.description {
            record.description = Some(description);
        }
        if let Some(provider_id) = command.provider_id {
            validate_standard_id(provider_id.as_str(), "providerId", Some("provider."))?;
            record.provider_id = provider_id;
        }
        if let Some(base_kind) = command.base_kind {
            record.base_kind = base_kind;
        }
        if let Some(retrieval_modes) = command.retrieval_modes {
            validate_non_empty_knowledge_modes(retrieval_modes.as_slice())?;
            record.retrieval_modes = retrieval_modes;
        }
        if let Some(capability_ids) = command.capability_ids {
            validate_capabilities(capability_ids.as_slice(), "capabilityIds")?;
            record.capability_ids = capability_ids;
        }
        if let Some(configuration_profile_id) = command.configuration_profile_id {
            validate_standard_id(
                configuration_profile_id.as_str(),
                "configurationProfileId",
                Some("profile."),
            )?;
            record.configuration_profile_id = configuration_profile_id;
        }
        if let Some(visibility) = command.visibility {
            record.visibility = visibility;
        }
        record.mark_updated(command.requested_at.clone());

        self.repository.update_knowledge_base(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeBaseUpdated,
            item_kind: "knowledge_base",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_base_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn delete_knowledge_base(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeBaseRecord> {
        self.authorize(
            "agent.business.knowledge.base.delete",
            command.requested_by.clone(),
            format!("agent.business.knowledge.base.{}", command.item_id),
            "knowledge.base.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_base(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge base not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge base",
        )?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_knowledge_base(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeBaseDeleted,
            item_kind: "knowledge_base",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_base_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_knowledge_base(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeBaseRecord> {
        self.authorize(
            "agent.business.knowledge.base.restore",
            command.requested_by.clone(),
            format!("agent.business.knowledge.base.{}", command.item_id),
            "knowledge.base.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_base(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge base not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge base",
        )?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_knowledge_base(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeBaseRestored,
            item_kind: "knowledge_base",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_base_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_knowledge_source(
        &mut self,
        command: AgentKnowledgeSourceCreateCommand,
    ) -> KernelResult<AgentKnowledgeSourceRecord> {
        self.authorize(
            "agent.business.knowledge.source.create",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.source.{}",
                command.knowledge_source_id
            ),
            "knowledge.source.create",
        )?;
        validate_standard_id(
            command.knowledge_source_id.as_str(),
            "knowledgeSourceId",
            Some("knowledge.source."),
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        validate_safe_text_field(
            command.source_ref.as_str(),
            "sourceRef",
            MAX_KNOWLEDGE_REFERENCE_CHARS,
        )?;
        validate_safe_text_field(
            command.source_hash.as_str(),
            "sourceHash",
            MAX_KNOWLEDGE_HASH_CHARS,
        )?;
        validate_marketplace_json(command.sync_policy_json.as_str(), "syncPolicyJson")?;
        validate_marketplace_json(command.metadata_json.as_str(), "metadataJson")?;
        self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;

        let record = AgentKnowledgeSourceRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            knowledge_source_id: command.knowledge_source_id,
            knowledge_base_id: command.knowledge_base_id,
            source_kind: command.source_kind,
            source_ref: command.source_ref,
            source_hash: command.source_hash,
            sync_policy_json: command.sync_policy_json,
            metadata_json: command.metadata_json,
            status: AgentBusinessStatus::Active,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        self.repository.insert_knowledge_source(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSourceCreated,
            item_kind: "knowledge_source",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_source_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_sources(
        &mut self,
        tenant_id: u64,
        knowledge_base_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeSourceRecord>> {
        self.authorize(
            "agent.business.knowledge.source.list",
            requested_by,
            format!("agent.business.knowledge.base.{knowledge_base_id}"),
            "knowledge.source.list",
        )?;
        validate_standard_id(
            knowledge_base_id,
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        self.get_active_knowledge_base(tenant_id, knowledge_base_id)?;
        Ok(self
            .repository
            .list_knowledge_sources(tenant_id, knowledge_base_id))
    }

    pub fn get_knowledge_source(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeSourceRecord> {
        self.authorize(
            "agent.business.knowledge.source.retrieve",
            command.requested_by,
            format!("agent.business.knowledge.source.{}", command.item_id),
            "knowledge.source.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeSourceId",
            Some("knowledge.source."),
        )?;
        self.get_active_knowledge_source(command.tenant_id, command.item_id.as_str())
    }

    pub fn update_knowledge_source(
        &mut self,
        command: AgentKnowledgeSourceUpdateCommand,
    ) -> KernelResult<AgentKnowledgeSourceRecord> {
        self.authorize(
            "agent.business.knowledge.source.update",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.source.{}",
                command.knowledge_source_id
            ),
            "knowledge.source.update",
        )?;
        validate_standard_id(
            command.knowledge_source_id.as_str(),
            "knowledgeSourceId",
            Some("knowledge.source."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_source(command.tenant_id, command.knowledge_source_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge source not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge source",
        )?;
        self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;

        if let Some(source_kind) = command.source_kind {
            record.source_kind = source_kind;
        }
        if let Some(source_ref) = command.source_ref {
            validate_safe_text_field(
                source_ref.as_str(),
                "sourceRef",
                MAX_KNOWLEDGE_REFERENCE_CHARS,
            )?;
            record.source_ref = source_ref;
        }
        if let Some(source_hash) = command.source_hash {
            validate_safe_text_field(source_hash.as_str(), "sourceHash", MAX_KNOWLEDGE_HASH_CHARS)?;
            record.source_hash = source_hash;
        }
        if let Some(sync_policy_json) = command.sync_policy_json {
            validate_marketplace_json(sync_policy_json.as_str(), "syncPolicyJson")?;
            record.sync_policy_json = sync_policy_json;
        }
        if let Some(metadata_json) = command.metadata_json {
            validate_marketplace_json(metadata_json.as_str(), "metadataJson")?;
            record.metadata_json = metadata_json;
        }
        record.mark_updated(command.requested_at.clone());

        self.repository.update_knowledge_source(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSourceUpdated,
            item_kind: "knowledge_source",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_source_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn delete_knowledge_source(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeSourceRecord> {
        self.authorize(
            "agent.business.knowledge.source.delete",
            command.requested_by.clone(),
            format!("agent.business.knowledge.source.{}", command.item_id),
            "knowledge.source.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeSourceId",
            Some("knowledge.source."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_source(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge source not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge source",
        )?;
        self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_knowledge_source(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSourceDeleted,
            item_kind: "knowledge_source",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_source_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_knowledge_source(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeSourceRecord> {
        self.authorize(
            "agent.business.knowledge.source.restore",
            command.requested_by.clone(),
            format!("agent.business.knowledge.source.{}", command.item_id),
            "knowledge.source.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeSourceId",
            Some("knowledge.source."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_source(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge source not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge source",
        )?;
        self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_knowledge_source(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSourceRestored,
            item_kind: "knowledge_source",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_source_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_knowledge_document(
        &mut self,
        command: AgentKnowledgeDocumentCreateCommand,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        self.authorize(
            "agent.business.knowledge.document.create",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.document.{}",
                command.knowledge_document_id
            ),
            "knowledge.document.create",
        )?;
        validate_standard_id(
            command.knowledge_document_id.as_str(),
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        if let Some(source_id) = command.knowledge_source_id.as_deref() {
            validate_standard_id(source_id, "knowledgeSourceId", Some("knowledge.source."))?;
        }
        validate_non_empty(command.title.as_str(), "title")?;
        validate_safe_text_field(
            command.content_ref.as_str(),
            "contentRef",
            MAX_KNOWLEDGE_REFERENCE_CHARS,
        )?;
        validate_safe_text_field(
            command.content_hash.as_str(),
            "contentHash",
            MAX_KNOWLEDGE_HASH_CHARS,
        )?;
        validate_marketplace_json(command.metadata_json.as_str(), "metadataJson")?;
        validate_marketplace_labels(command.tags.as_slice(), "tags")?;
        validate_marketplace_labels(command.categories.as_slice(), "categories")?;
        validate_trust_level(command.trust_level)?;
        validate_safe_text_field(
            command.redaction_classification.as_str(),
            "redactionClassification",
            MAX_KNOWLEDGE_REDACTION_CLASSIFICATION_CHARS,
        )?;
        let base =
            self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;
        if let Some(source_id) = command.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(command.tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }

        let record = AgentKnowledgeDocumentRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            knowledge_document_id: command.knowledge_document_id,
            knowledge_base_id: command.knowledge_base_id,
            knowledge_source_id: command.knowledge_source_id,
            document_kind: command.document_kind,
            title: command.title,
            content_ref: command.content_ref,
            content_hash: command.content_hash,
            summary: command.summary,
            metadata_json: command.metadata_json,
            tags: command.tags,
            categories: command.categories,
            trust_level: command.trust_level,
            redaction_classification: command.redaction_classification,
            chunk_count: 0,
            status: AgentBusinessStatus::Active,
            visibility: base.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        self.repository.insert_knowledge_document(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeDocumentCreated,
            item_kind: "knowledge_document",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_document_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_documents(
        &mut self,
        tenant_id: u64,
        knowledge_base_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeDocumentRecord>> {
        self.list_knowledge(AgentKnowledgeListCommand {
            tenant_id,
            knowledge_base_id: knowledge_base_id.to_string(),
            requested_by,
        })
    }

    pub fn list_knowledge(
        &mut self,
        command: AgentKnowledgeListCommand,
    ) -> KernelResult<Vec<AgentKnowledgeDocumentRecord>> {
        self.authorize(
            "agent.business.knowledge.list",
            command.requested_by,
            format!(
                "agent.business.knowledge.base.{}",
                command.knowledge_base_id
            ),
            "knowledge.list",
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;
        let mut active_documents = Vec::new();
        for document in self
            .repository
            .list_knowledge_documents(command.tenant_id, command.knowledge_base_id.as_str())
        {
            if self
                .get_active_knowledge_document(
                    command.tenant_id,
                    document.knowledge_document_id.as_str(),
                )
                .is_ok()
            {
                active_documents.push(document);
            }
        }
        Ok(active_documents)
    }

    pub fn get_knowledge_document(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        self.read_knowledge(AgentKnowledgeReadCommand {
            tenant_id: command.tenant_id,
            knowledge_document_id: command.item_id,
            requested_by: command.requested_by,
        })
    }

    pub fn update_knowledge_document(
        &mut self,
        command: AgentKnowledgeDocumentUpdateCommand,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        self.authorize(
            "agent.business.knowledge.document.update",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.document.{}",
                command.knowledge_document_id
            ),
            "knowledge.document.update",
        )?;
        validate_standard_id(
            command.knowledge_document_id.as_str(),
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_document(command.tenant_id, command.knowledge_document_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge document not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge document",
        )?;
        let base =
            self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        if let Some(source_id) = record.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(command.tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }

        if let Some(source_id) = command.knowledge_source_id {
            validate_standard_id(
                source_id.as_str(),
                "knowledgeSourceId",
                Some("knowledge.source."),
            )?;
            let source = self.get_active_knowledge_source(command.tenant_id, source_id.as_str())?;
            if source.knowledge_base_id != record.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
            record.knowledge_source_id = Some(source_id);
        }
        if let Some(document_kind) = command.document_kind {
            record.document_kind = document_kind;
        }
        if let Some(title) = command.title {
            validate_non_empty(title.as_str(), "title")?;
            record.title = title;
        }
        if let Some(content_ref) = command.content_ref {
            validate_safe_text_field(
                content_ref.as_str(),
                "contentRef",
                MAX_KNOWLEDGE_REFERENCE_CHARS,
            )?;
            record.content_ref = content_ref;
        }
        if let Some(content_hash) = command.content_hash {
            validate_safe_text_field(
                content_hash.as_str(),
                "contentHash",
                MAX_KNOWLEDGE_HASH_CHARS,
            )?;
            record.content_hash = content_hash;
        }
        if let Some(summary) = command.summary {
            record.summary = Some(summary);
        }
        if let Some(metadata_json) = command.metadata_json {
            validate_marketplace_json(metadata_json.as_str(), "metadataJson")?;
            record.metadata_json = metadata_json;
        }
        if let Some(tags) = command.tags {
            validate_marketplace_labels(tags.as_slice(), "tags")?;
            record.tags = tags;
        }
        if let Some(categories) = command.categories {
            validate_marketplace_labels(categories.as_slice(), "categories")?;
            record.categories = categories;
        }
        if let Some(trust_level) = command.trust_level {
            validate_trust_level(trust_level)?;
            record.trust_level = trust_level;
        }
        if let Some(redaction_classification) = command.redaction_classification {
            validate_safe_text_field(
                redaction_classification.as_str(),
                "redactionClassification",
                MAX_KNOWLEDGE_REDACTION_CLASSIFICATION_CHARS,
            )?;
            record.redaction_classification = redaction_classification;
        }
        record.mark_updated(command.requested_at.clone());

        self.repository.update_knowledge_document(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeDocumentUpdated,
            item_kind: "knowledge_document",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_document_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn read_knowledge(
        &mut self,
        command: AgentKnowledgeReadCommand,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        self.authorize(
            "agent.business.knowledge.read",
            command.requested_by,
            format!(
                "agent.business.knowledge.document.{}",
                command.knowledge_document_id
            ),
            "knowledge.read",
        )?;
        validate_standard_id(
            command.knowledge_document_id.as_str(),
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        self.get_active_knowledge_document(
            command.tenant_id,
            command.knowledge_document_id.as_str(),
        )
    }

    pub fn delete_knowledge_document(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        self.authorize(
            "agent.business.knowledge.document.delete",
            command.requested_by.clone(),
            format!("agent.business.knowledge.document.{}", command.item_id),
            "knowledge.document.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_document(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge document not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge document",
        )?;
        self.get_active_knowledge_document(
            command.tenant_id,
            record.knowledge_document_id.as_str(),
        )?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_knowledge_document(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeDocumentDeleted,
            item_kind: "knowledge_document",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_document_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_knowledge_document(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        self.authorize(
            "agent.business.knowledge.document.restore",
            command.requested_by.clone(),
            format!("agent.business.knowledge.document.{}", command.item_id),
            "knowledge.document.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        let mut record = self
            .repository
            .get_knowledge_document(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge document not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent knowledge document",
        )?;
        let base =
            self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        if let Some(source_id) = record.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(command.tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }
        record.mark_restored(command.requested_at.clone());
        self.repository.update_knowledge_document(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeDocumentRestored,
            item_kind: "knowledge_document",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_document_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_knowledge_chunk(
        &mut self,
        command: AgentKnowledgeChunkCreateCommand,
    ) -> KernelResult<AgentKnowledgeChunkRecord> {
        self.authorize(
            "agent.business.knowledge.chunk.create",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.chunk.{}",
                command.knowledge_chunk_id
            ),
            "knowledge.chunk.create",
        )?;
        validate_standard_id(
            command.knowledge_chunk_id.as_str(),
            "knowledgeChunkId",
            Some("knowledge.chunk."),
        )?;
        validate_standard_id(
            command.knowledge_document_id.as_str(),
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        if let Some(parent_chunk_id) = command.parent_chunk_id.as_deref() {
            validate_standard_id(parent_chunk_id, "parentChunkId", Some("knowledge.chunk."))?;
        }
        if command.chunk_ordinal == 0 {
            return Err(KernelError::validation(
                "chunkOrdinal must be greater than 0",
            ));
        }
        if command.token_estimate == 0 {
            return Err(KernelError::validation(
                "tokenEstimate must be greater than 0",
            ));
        }
        if let Some(heading) = command.heading.as_deref() {
            validate_safe_text_field(heading, "heading", MAX_KNOWLEDGE_HEADING_CHARS)?;
        }
        validate_safe_text_field(
            command.content_ref.as_str(),
            "contentRef",
            MAX_KNOWLEDGE_REFERENCE_CHARS,
        )?;
        validate_safe_text_field(
            command.content_hash.as_str(),
            "contentHash",
            MAX_KNOWLEDGE_HASH_CHARS,
        )?;
        validate_marketplace_json(command.metadata_json.as_str(), "metadataJson")?;
        let document = self.get_active_knowledge_document(
            command.tenant_id,
            command.knowledge_document_id.as_str(),
        )?;
        self.get_active_knowledge_base(command.tenant_id, document.knowledge_base_id.as_str())?;
        if let Some(parent_chunk_id) = command.parent_chunk_id.as_deref() {
            let parent = self
                .get_active_knowledge_chunk(command.tenant_id, parent_chunk_id)
                .map_err(|_| KernelError::validation("parent knowledge chunk not found"))?;
            if parent.knowledge_document_id != document.knowledge_document_id {
                return Err(KernelError::validation(
                    "parent knowledge chunk must belong to knowledge document",
                ));
            }
        }

        let record = AgentKnowledgeChunkRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            knowledge_chunk_id: command.knowledge_chunk_id,
            knowledge_document_id: command.knowledge_document_id,
            parent_chunk_id: command.parent_chunk_id,
            chunk_ordinal: command.chunk_ordinal,
            heading: command.heading,
            content_ref: command.content_ref,
            content_hash: command.content_hash,
            token_estimate: command.token_estimate,
            summary: command.summary,
            metadata_json: command.metadata_json,
            status: AgentBusinessStatus::Active,
            created_at: command.requested_at.clone(),
        };
        self.repository.insert_knowledge_chunk(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeChunkCreated,
            item_kind: "knowledge_chunk",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_chunk_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_chunks(
        &mut self,
        tenant_id: u64,
        knowledge_document_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeChunkRecord>> {
        self.authorize(
            "agent.business.knowledge.chunk.list",
            requested_by,
            format!("agent.business.knowledge.document.{knowledge_document_id}"),
            "knowledge.chunk.list",
        )?;
        validate_standard_id(
            knowledge_document_id,
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        let document = self.get_active_knowledge_document(tenant_id, knowledge_document_id)?;
        self.get_active_knowledge_base(tenant_id, document.knowledge_base_id.as_str())?;
        Ok(self
            .repository
            .list_knowledge_chunks(tenant_id, knowledge_document_id))
    }

    pub fn get_knowledge_chunk(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeChunkRecord> {
        self.authorize(
            "agent.business.knowledge.chunk.retrieve",
            command.requested_by,
            format!("agent.business.knowledge.chunk.{}", command.item_id),
            "knowledge.chunk.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeChunkId",
            Some("knowledge.chunk."),
        )?;
        let record =
            self.get_active_knowledge_chunk(command.tenant_id, command.item_id.as_str())?;
        let document = self.get_active_knowledge_document(
            command.tenant_id,
            record.knowledge_document_id.as_str(),
        )?;
        self.get_active_knowledge_base(command.tenant_id, document.knowledge_base_id.as_str())?;
        Ok(record)
    }

    pub fn upsert_knowledge_index(
        &mut self,
        command: AgentKnowledgeIndexUpsertCommand,
    ) -> KernelResult<AgentKnowledgeIndexRecord> {
        self.authorize(
            "agent.business.knowledge.index.upsert",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.index.{}",
                command.knowledge_index_id
            ),
            "knowledge.index.upsert",
        )?;
        validate_standard_id(
            command.knowledge_index_id.as_str(),
            "knowledgeIndexId",
            Some("knowledge.index."),
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        if let Some(document_id) = command.knowledge_document_id.as_deref() {
            validate_standard_id(
                document_id,
                "knowledgeDocumentId",
                Some("knowledge.document."),
            )?;
        }
        if let Some(chunk_id) = command.knowledge_chunk_id.as_deref() {
            validate_standard_id(chunk_id, "knowledgeChunkId", Some("knowledge.chunk."))?;
        }
        validate_standard_id(
            command.index_provider_id.as_str(),
            "indexProviderId",
            Some("provider."),
        )?;
        validate_safe_text_field(
            command.external_ref.as_str(),
            "externalRef",
            MAX_KNOWLEDGE_REFERENCE_CHARS,
        )?;
        validate_safe_text_field(
            command.content_hash.as_str(),
            "contentHash",
            MAX_KNOWLEDGE_HASH_CHARS,
        )?;
        if command.index_kind == AgentKnowledgeIndexKind::Vector {
            if command.embedding_model_id.is_none() || command.vector_dimension.is_none() {
                return Err(KernelError::validation(
                    "vector knowledge index requires embeddingModelId and vectorDimension",
                ));
            }
            if command.vector_dimension == Some(0) {
                return Err(KernelError::validation(
                    "vectorDimension must be greater than 0",
                ));
            }
        }
        if let Some(embedding_model_id) = command.embedding_model_id.as_deref() {
            validate_standard_id(embedding_model_id, "embeddingModelId", Some("model."))?;
        }
        let base =
            self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;
        let document = if let Some(document_id) = command.knowledge_document_id.as_deref() {
            let document = self.get_active_knowledge_document(command.tenant_id, document_id)?;
            if document.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge document does not belong to knowledge base",
                ));
            }
            Some(document)
        } else {
            None
        };
        if let Some(chunk_id) = command.knowledge_chunk_id.as_deref() {
            let Some(document) = document.as_ref() else {
                return Err(KernelError::validation(
                    "knowledgeDocumentId is required when knowledgeChunkId is provided",
                ));
            };
            let chunk = self.get_active_knowledge_chunk(command.tenant_id, chunk_id)?;
            if chunk.knowledge_document_id != document.knowledge_document_id {
                return Err(KernelError::validation(
                    "agent knowledge chunk does not belong to knowledge document",
                ));
            }
        }

        let record = AgentKnowledgeIndexRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            knowledge_index_id: command.knowledge_index_id,
            knowledge_base_id: command.knowledge_base_id,
            knowledge_document_id: command.knowledge_document_id,
            knowledge_chunk_id: command.knowledge_chunk_id,
            index_kind: command.index_kind,
            index_provider_id: command.index_provider_id,
            external_ref: command.external_ref,
            embedding_model_id: command.embedding_model_id,
            vector_dimension: command.vector_dimension,
            content_hash: command.content_hash,
            indexed_at: command.requested_at.clone(),
            status: AgentBusinessStatus::Active,
        };
        self.repository.upsert_knowledge_index(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeIndexUpserted,
            item_kind: "knowledge_index",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.knowledge_index_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_indexes(
        &mut self,
        tenant_id: u64,
        knowledge_document_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeIndexRecord>> {
        self.authorize(
            "agent.business.knowledge.index.list",
            requested_by,
            format!("agent.business.knowledge.document.{knowledge_document_id}"),
            "knowledge.index.list",
        )?;
        validate_standard_id(
            knowledge_document_id,
            "knowledgeDocumentId",
            Some("knowledge.document."),
        )?;
        let document = self.get_active_knowledge_document(tenant_id, knowledge_document_id)?;
        self.get_active_knowledge_base(tenant_id, document.knowledge_base_id.as_str())?;
        Ok(self
            .repository
            .list_knowledge_indexes(tenant_id, knowledge_document_id)
            .into_iter()
            .filter(|record| record.status != AgentBusinessStatus::Deleted)
            .collect())
    }

    pub fn get_knowledge_index(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeIndexRecord> {
        self.authorize(
            "agent.business.knowledge.index.retrieve",
            command.requested_by,
            format!("agent.business.knowledge.index.{}", command.item_id),
            "knowledge.index.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeIndexId",
            Some("knowledge.index."),
        )?;
        let record = self
            .repository
            .get_knowledge_index(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge index not found"))?;
        if record.status == AgentBusinessStatus::Deleted {
            return Err(KernelError::validation("agent knowledge index not found"));
        }

        let base =
            self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        let document = if let Some(document_id) = record.knowledge_document_id.as_deref() {
            let document = self.get_active_knowledge_document(command.tenant_id, document_id)?;
            if document.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge document does not belong to knowledge base",
                ));
            }
            Some(document)
        } else {
            None
        };
        if let Some(chunk_id) = record.knowledge_chunk_id.as_deref() {
            let Some(document) = document.as_ref() else {
                return Err(KernelError::validation(
                    "knowledge document is required when knowledge chunk is indexed",
                ));
            };
            let chunk = self.get_active_knowledge_chunk(command.tenant_id, chunk_id)?;
            if chunk.knowledge_document_id != document.knowledge_document_id {
                return Err(KernelError::validation(
                    "agent knowledge chunk does not belong to knowledge document",
                ));
            }
        }

        Ok(record)
    }

    pub fn search_knowledge(
        &mut self,
        command: AgentKnowledgeSearchCommand,
    ) -> KernelResult<Vec<AgentKnowledgeSearchResult>> {
        self.authorize(
            "agent.business.knowledge.search",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.base.{}",
                command.knowledge_base_id
            ),
            "knowledge.search",
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        if command.top_k == 0 {
            return Err(KernelError::validation("topK must be greater than 0"));
        }
        if command.top_k > MAX_KNOWLEDGE_SEARCH_TOP_K {
            return Err(KernelError::validation(format!(
                "topK must be at most {MAX_KNOWLEDGE_SEARCH_TOP_K}"
            )));
        }
        let query_terms = knowledge_search_terms(command.query.as_str())?;
        let base =
            self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;

        let retrieval_modes = if command.retrieval_modes.is_empty() {
            base.retrieval_modes.clone()
        } else {
            command.retrieval_modes.clone()
        };
        validate_non_empty_knowledge_modes(retrieval_modes.as_slice())?;
        for mode in &retrieval_modes {
            if !base.retrieval_modes.contains(mode) {
                return Err(KernelError::validation(format!(
                    "retrievalModes must be supported by knowledge base: {}",
                    mode.as_str()
                )));
            }
        }

        let mut candidates: Vec<(String, AgentKnowledgeSearchResult)> = Vec::new();
        for index in self
            .repository
            .list_knowledge_indexes_by_base(command.tenant_id, base.knowledge_base_id.as_str())
        {
            if index.status == AgentBusinessStatus::Deleted {
                continue;
            }
            if !retrieval_modes.contains(&index.index_kind) {
                continue;
            }
            if !command.include_external && index.index_kind == AgentKnowledgeIndexKind::External {
                continue;
            }

            let document = match index.knowledge_document_id.as_deref() {
                Some(document_id) => {
                    match self.get_active_knowledge_document(command.tenant_id, document_id) {
                        Ok(record) if record.knowledge_base_id == base.knowledge_base_id => {
                            Some(record)
                        }
                        _ => continue,
                    }
                }
                None => None,
            };
            if !command.include_external
                && document
                    .as_ref()
                    .map(|record| {
                        record.document_kind == AgentKnowledgeDocumentKind::ExternalReference
                    })
                    .unwrap_or(false)
            {
                continue;
            }

            let chunk = match index.knowledge_chunk_id.as_deref() {
                Some(chunk_id) => {
                    match self.get_active_knowledge_chunk(command.tenant_id, chunk_id) {
                        Ok(record)
                            if document
                                .as_ref()
                                .map(|doc| {
                                    record.knowledge_document_id == doc.knowledge_document_id
                                })
                                .unwrap_or(false) =>
                        {
                            Some(record)
                        }
                        _ => continue,
                    }
                }
                None => None,
            };

            let source = document
                .as_ref()
                .and_then(|record| record.knowledge_source_id.as_deref())
                .and_then(|source_id| {
                    self.get_active_knowledge_source(command.tenant_id, source_id)
                        .ok()
                });
            let score = knowledge_search_score(
                &index,
                &base,
                document.as_ref(),
                chunk.as_ref(),
                source.as_ref(),
                query_terms.as_slice(),
            );
            if score <= 0.0 {
                continue;
            }

            let title = document
                .as_ref()
                .map(|record| record.title.clone())
                .unwrap_or_else(|| base.display_name.clone());
            let snippet = chunk
                .as_ref()
                .and_then(|record| record.summary.clone())
                .or_else(|| chunk.as_ref().and_then(|record| record.heading.clone()))
                .or_else(|| document.as_ref().and_then(|record| record.summary.clone()));
            let content_ref = chunk
                .as_ref()
                .map(|record| record.content_ref.clone())
                .or_else(|| document.as_ref().map(|record| record.content_ref.clone()));
            let metadata_json = chunk
                .as_ref()
                .map(|record| record.metadata_json.clone())
                .or_else(|| document.as_ref().map(|record| record.metadata_json.clone()))
                .unwrap_or_else(|| "{}".to_string());

            let result = AgentKnowledgeSearchResult {
                tenant_id: command.tenant_id,
                knowledge_base_id: base.knowledge_base_id.clone(),
                provider_id: base.provider_id.clone(),
                knowledge_index_id: index.knowledge_index_id.clone(),
                index_provider_id: index.index_provider_id.clone(),
                retrieval_method: index.index_kind,
                knowledge_document_id: document
                    .as_ref()
                    .map(|record| record.knowledge_document_id.clone()),
                document_kind: document.as_ref().map(|record| record.document_kind),
                knowledge_chunk_id: chunk
                    .as_ref()
                    .map(|record| record.knowledge_chunk_id.clone()),
                title,
                snippet,
                score: Some(score),
                source_ref: source.as_ref().map(|record| record.source_ref.clone()),
                content_ref,
                external_ref: Some(index.external_ref.clone()),
                trust_level: document
                    .as_ref()
                    .map(|record| record.trust_level)
                    .unwrap_or(3),
                redaction_classification: document
                    .as_ref()
                    .map(|record| record.redaction_classification.clone())
                    .unwrap_or_else(|| "internal".to_string()),
                metadata_json,
            };
            candidates.push((index.indexed_at, result));
        }

        candidates.sort_by(|left, right| {
            right
                .1
                .score
                .unwrap_or(0.0)
                .partial_cmp(&left.1.score.unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.0.cmp(&left.0))
                .then_with(|| left.1.knowledge_index_id.cmp(&right.1.knowledge_index_id))
        });

        Ok(candidates
            .into_iter()
            .take(command.top_k)
            .map(|(_, result)| result)
            .collect())
    }

    pub fn create_knowledge_binding(
        &mut self,
        command: AgentKnowledgeBindingCreateCommand,
    ) -> KernelResult<AgentKnowledgeBindingRecord> {
        self.authorize(
            "agent.business.knowledge.binding.create",
            command.requested_by.clone(),
            format!(
                "agent.business.knowledge.binding.{}",
                command.knowledge_binding_id
            ),
            "knowledge.binding.create",
        )?;
        validate_standard_id(
            command.knowledge_binding_id.as_str(),
            "knowledgeBindingId",
            Some("knowledge.binding."),
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        validate_optional_agent_id(command.agent_id.as_deref())?;
        validate_optional_standard_ref(
            command.deployment_id.as_deref(),
            "deploymentId",
            "deployment.",
        )?;
        validate_safe_text_field(
            command.scope_ref.as_str(),
            "scopeRef",
            MAX_KNOWLEDGE_SCOPE_REF_CHARS,
        )?;
        validate_knowledge_binding_scope(
            command.scope_kind,
            command.scope_ref.as_str(),
            command.agent_id.as_deref(),
            command.deployment_id.as_deref(),
        )?;
        self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;

        let record = AgentKnowledgeBindingRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            knowledge_binding_id: command.knowledge_binding_id,
            knowledge_base_id: command.knowledge_base_id,
            agent_id: command.agent_id,
            deployment_id: command.deployment_id,
            scope_kind: command.scope_kind,
            scope_ref: command.scope_ref,
            active: command.active,
            default_binding: command.default_binding,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
        };
        self.repository.insert_knowledge_binding(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeBindingCreated,
            item_kind: "knowledge_binding",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.knowledge_binding_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_bindings(
        &mut self,
        tenant_id: u64,
        knowledge_base_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeBindingRecord>> {
        self.authorize(
            "agent.business.knowledge.binding.list",
            requested_by,
            format!("agent.business.knowledge.base.{knowledge_base_id}"),
            "knowledge.binding.list",
        )?;
        validate_standard_id(
            knowledge_base_id,
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        self.get_active_knowledge_base(tenant_id, knowledge_base_id)?;
        Ok(self
            .repository
            .list_knowledge_bindings(tenant_id, knowledge_base_id))
    }

    pub fn get_knowledge_binding(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeBindingRecord> {
        self.authorize(
            "agent.business.knowledge.binding.retrieve",
            command.requested_by,
            format!("agent.business.knowledge.binding.{}", command.item_id),
            "knowledge.binding.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "knowledgeBindingId",
            Some("knowledge.binding."),
        )?;
        let record = self
            .repository
            .get_knowledge_binding(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge binding not found"))?;
        self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        Ok(record)
    }

    pub fn create_knowledge_sync_job(
        &mut self,
        command: AgentKnowledgeSyncJobCreateCommand,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        self.authorize(
            "agent.business.knowledge.sync_job.create",
            command.requested_by.clone(),
            format!("agent.business.knowledge.sync_job.{}", command.sync_job_id),
            "knowledge.sync_job.create",
        )?;
        validate_standard_id(
            command.sync_job_id.as_str(),
            "syncJobId",
            Some("knowledge.sync."),
        )?;
        validate_standard_id(
            command.knowledge_base_id.as_str(),
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        if let Some(source_id) = command.knowledge_source_id.as_deref() {
            validate_standard_id(source_id, "knowledgeSourceId", Some("knowledge.source."))?;
        }
        validate_safe_text_field(
            command.input_ref.as_str(),
            "inputRef",
            MAX_KNOWLEDGE_REFERENCE_CHARS,
        )?;
        validate_marketplace_json(command.input_json.as_str(), "inputJson")?;
        let base =
            self.get_active_knowledge_base(command.tenant_id, command.knowledge_base_id.as_str())?;
        if let Some(source_id) = command.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(command.tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }

        let record = AgentKnowledgeSyncJobRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            sync_job_id: command.sync_job_id,
            knowledge_base_id: command.knowledge_base_id,
            knowledge_source_id: command.knowledge_source_id,
            job_kind: command.job_kind,
            status: AgentKnowledgeSyncJobStatus::Queued,
            input_ref: command.input_ref,
            input_json: command.input_json,
            output_json: None,
            error_json: None,
            requested_at: command.requested_at.clone(),
            started_at: None,
            completed_at: None,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
        };
        self.repository.insert_knowledge_sync_job(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSyncJobCreated,
            item_kind: "knowledge_sync_job",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.sync_job_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_knowledge_sync_jobs(
        &mut self,
        tenant_id: u64,
        knowledge_base_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentKnowledgeSyncJobRecord>> {
        self.authorize(
            "agent.business.knowledge.sync_job.list",
            requested_by,
            format!("agent.business.knowledge.base.{knowledge_base_id}"),
            "knowledge.sync_job.list",
        )?;
        validate_standard_id(
            knowledge_base_id,
            "knowledgeBaseId",
            Some("knowledge.base."),
        )?;
        self.get_active_knowledge_base(tenant_id, knowledge_base_id)?;
        Ok(self
            .repository
            .list_knowledge_sync_jobs(tenant_id, knowledge_base_id))
    }

    pub fn get_knowledge_sync_job(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        self.authorize(
            "agent.business.knowledge.sync_job.retrieve",
            command.requested_by,
            format!("agent.business.knowledge.sync_job.{}", command.item_id),
            "knowledge.sync_job.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "syncJobId",
            Some("knowledge.sync."),
        )?;
        let record = self
            .repository
            .get_knowledge_sync_job(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent knowledge sync job not found"))?;
        let base =
            self.get_active_knowledge_base(command.tenant_id, record.knowledge_base_id.as_str())?;
        if let Some(source_id) = record.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(command.tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }
        Ok(record)
    }

    pub fn start_knowledge_sync_job(
        &mut self,
        command: AgentKnowledgeSyncJobStartCommand,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        self.authorize(
            "agent.business.knowledge.sync_job.start",
            command.requested_by.clone(),
            format!("agent.business.knowledge.sync_job.{}", command.sync_job_id),
            "knowledge.sync_job.start",
        )?;
        let mut record =
            self.get_active_knowledge_sync_job(command.tenant_id, command.sync_job_id.as_str())?;
        if record.status != AgentKnowledgeSyncJobStatus::Queued {
            return Err(KernelError::validation(
                "queued knowledge sync job is required to start",
            ));
        }

        record.status = AgentKnowledgeSyncJobStatus::Running;
        record.output_json = None;
        record.error_json = None;
        record.started_at = Some(command.requested_at.clone());
        record.completed_at = None;
        record.updated_at = command.requested_at.clone();
        self.repository.update_knowledge_sync_job(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSyncJobStarted,
            item_kind: "knowledge_sync_job",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.sync_job_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: knowledge_sync_job_audit_sequence(record.status),
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn complete_knowledge_sync_job(
        &mut self,
        command: AgentKnowledgeSyncJobCompleteCommand,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        self.authorize(
            "agent.business.knowledge.sync_job.complete",
            command.requested_by.clone(),
            format!("agent.business.knowledge.sync_job.{}", command.sync_job_id),
            "knowledge.sync_job.complete",
        )?;
        validate_marketplace_json(command.output_json.as_str(), "outputJson")?;
        let mut record =
            self.get_active_knowledge_sync_job(command.tenant_id, command.sync_job_id.as_str())?;
        if record.status != AgentKnowledgeSyncJobStatus::Running {
            return Err(KernelError::validation(
                "running knowledge sync job is required to complete",
            ));
        }

        record.status = AgentKnowledgeSyncJobStatus::Succeeded;
        record.output_json = Some(command.output_json);
        record.error_json = None;
        if record.started_at.is_none() {
            record.started_at = Some(command.requested_at.clone());
        }
        record.completed_at = Some(command.requested_at.clone());
        record.updated_at = command.requested_at.clone();
        self.repository.update_knowledge_sync_job(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSyncJobCompleted,
            item_kind: "knowledge_sync_job",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.sync_job_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: knowledge_sync_job_audit_sequence(record.status),
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn fail_knowledge_sync_job(
        &mut self,
        command: AgentKnowledgeSyncJobFailCommand,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        self.authorize(
            "agent.business.knowledge.sync_job.fail",
            command.requested_by.clone(),
            format!("agent.business.knowledge.sync_job.{}", command.sync_job_id),
            "knowledge.sync_job.fail",
        )?;
        validate_marketplace_json(command.error_json.as_str(), "errorJson")?;
        let mut record =
            self.get_active_knowledge_sync_job(command.tenant_id, command.sync_job_id.as_str())?;
        if record.status != AgentKnowledgeSyncJobStatus::Running {
            return Err(KernelError::validation(
                "running knowledge sync job is required to fail",
            ));
        }

        record.status = AgentKnowledgeSyncJobStatus::Failed;
        record.output_json = None;
        record.error_json = Some(command.error_json);
        if record.started_at.is_none() {
            record.started_at = Some(command.requested_at.clone());
        }
        record.completed_at = Some(command.requested_at.clone());
        record.updated_at = command.requested_at.clone();
        self.repository.update_knowledge_sync_job(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSyncJobFailed,
            item_kind: "knowledge_sync_job",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.sync_job_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: knowledge_sync_job_audit_sequence(record.status),
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn cancel_knowledge_sync_job(
        &mut self,
        command: AgentKnowledgeSyncJobCancelCommand,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        self.authorize(
            "agent.business.knowledge.sync_job.cancel",
            command.requested_by.clone(),
            format!("agent.business.knowledge.sync_job.{}", command.sync_job_id),
            "knowledge.sync_job.cancel",
        )?;
        validate_marketplace_json(command.cancellation_json.as_str(), "cancellationJson")?;
        let mut record =
            self.get_active_knowledge_sync_job(command.tenant_id, command.sync_job_id.as_str())?;
        if !matches!(
            record.status,
            AgentKnowledgeSyncJobStatus::Queued | AgentKnowledgeSyncJobStatus::Running
        ) {
            return Err(KernelError::validation(
                "queued or running knowledge sync job is required to cancel",
            ));
        }

        record.status = AgentKnowledgeSyncJobStatus::Cancelled;
        record.output_json = None;
        record.error_json = Some(command.cancellation_json);
        record.completed_at = Some(command.requested_at.clone());
        record.updated_at = command.requested_at.clone();
        self.repository.update_knowledge_sync_job(record.clone())?;
        self.emit_knowledge_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::KnowledgeSyncJobCancelled,
            item_kind: "knowledge_sync_job",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.sync_job_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: knowledge_sync_job_audit_sequence(record.status),
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_memory_store(
        &mut self,
        command: AgentMemoryStoreCreateCommand,
    ) -> KernelResult<AgentMemoryStoreRecord> {
        self.authorize(
            "agent.business.memory.store.create",
            command.requested_by.clone(),
            format!("agent.business.memory.store.{}", command.memory_store_id),
            "memory.store.create",
        )?;
        validate_marketplace_identity(
            command.memory_store_id.as_str(),
            "memoryStoreId",
            Some("memory.store."),
            command.code.as_str(),
            command.display_name.as_str(),
        )?;
        validate_standard_id(
            command.provider_id.as_str(),
            "providerId",
            Some("provider."),
        )?;
        validate_standard_id(
            command.configuration_profile_id.as_str(),
            "configurationProfileId",
            Some("profile."),
        )?;
        validate_capabilities(command.capability_ids.as_slice(), "capabilityIds")?;
        validate_non_empty_memory_modes(command.retrieval_modes.as_slice())?;

        if self
            .repository
            .get_memory_store(command.tenant_id, command.memory_store_id.as_str())
            .is_some()
        {
            return Err(KernelError::conflict("agent memory store already exists"));
        }

        let record = AgentMemoryStoreRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            memory_store_id: command.memory_store_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            provider_id: command.provider_id,
            store_kind: command.store_kind,
            retrieval_modes: command.retrieval_modes,
            capability_ids: command.capability_ids,
            configuration_profile_id: command.configuration_profile_id,
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        self.repository.insert_memory_store(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryStoreCreated,
            item_kind: "memory_store",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_store_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn update_memory_store(
        &mut self,
        command: AgentMemoryStoreUpdateCommand,
    ) -> KernelResult<AgentMemoryStoreRecord> {
        self.authorize(
            "agent.business.memory.store.update",
            command.requested_by.clone(),
            format!("agent.business.memory.store.{}", command.memory_store_id),
            "memory.store.update",
        )?;
        validate_standard_id(
            command.memory_store_id.as_str(),
            "memoryStoreId",
            Some("memory.store."),
        )?;
        let mut record = self
            .repository
            .get_memory_store(command.tenant_id, command.memory_store_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory store not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory store",
        )?;

        if let Some(display_name) = command.display_name {
            validate_non_empty(display_name.as_str(), "displayName")?;
            record.display_name = display_name;
        }
        if let Some(description) = command.description {
            record.description = Some(description);
        }
        if let Some(provider_id) = command.provider_id {
            validate_standard_id(provider_id.as_str(), "providerId", Some("provider."))?;
            record.provider_id = provider_id;
        }
        if let Some(store_kind) = command.store_kind {
            record.store_kind = store_kind;
        }
        if let Some(retrieval_modes) = command.retrieval_modes {
            validate_non_empty_memory_modes(retrieval_modes.as_slice())?;
            record.retrieval_modes = retrieval_modes;
        }
        if let Some(capability_ids) = command.capability_ids {
            validate_capabilities(capability_ids.as_slice(), "capabilityIds")?;
            record.capability_ids = capability_ids;
        }
        if let Some(configuration_profile_id) = command.configuration_profile_id {
            validate_standard_id(
                configuration_profile_id.as_str(),
                "configurationProfileId",
                Some("profile."),
            )?;
            record.configuration_profile_id = configuration_profile_id;
        }
        if let Some(visibility) = command.visibility {
            record.visibility = visibility;
        }
        record.mark_updated(command.requested_at.clone());
        self.repository.update_memory_store(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryStoreUpdated,
            item_kind: "memory_store",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_store_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_store(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryStoreRecord> {
        self.authorize(
            "agent.business.memory.store.retrieve",
            command.requested_by,
            format!("agent.business.memory.store.{}", command.item_id),
            "memory.store.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryStoreId",
            Some("memory.store."),
        )?;
        self.get_active_memory_store(command.tenant_id, command.item_id.as_str())
    }

    pub fn create_memory_profile(
        &mut self,
        command: AgentMemoryProfileCreateCommand,
    ) -> KernelResult<AgentMemoryProfileRecord> {
        self.authorize(
            "agent.business.memory.profile.create",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.profile.{}",
                command.memory_profile_id
            ),
            "memory.profile.create",
        )?;
        validate_marketplace_identity(
            command.memory_profile_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
            command.code.as_str(),
            command.display_name.as_str(),
        )?;
        validate_standard_id(
            command.memory_store_id.as_str(),
            "memoryStoreId",
            Some("memory.store."),
        )?;
        for (value, field) in [
            (command.write_policy_json.as_str(), "writePolicyJson"),
            (
                command.retrieval_policy_json.as_str(),
                "retrievalPolicyJson",
            ),
            (
                command.compaction_policy_json.as_str(),
                "compactionPolicyJson",
            ),
            (
                command.retention_policy_json.as_str(),
                "retentionPolicyJson",
            ),
            (command.privacy_policy_json.as_str(), "privacyPolicyJson"),
        ] {
            validate_marketplace_json(value, field)?;
        }
        self.get_active_memory_store(command.tenant_id, command.memory_store_id.as_str())?;

        let record = AgentMemoryProfileRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            owner_user_id: command.owner_user_id,
            memory_profile_id: command.memory_profile_id,
            memory_store_id: command.memory_store_id,
            code: command.code,
            display_name: command.display_name,
            description: command.description,
            write_policy_json: command.write_policy_json,
            retrieval_policy_json: command.retrieval_policy_json,
            compaction_policy_json: command.compaction_policy_json,
            retention_policy_json: command.retention_policy_json,
            privacy_policy_json: command.privacy_policy_json,
            status: AgentBusinessStatus::Draft,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        self.repository.insert_memory_profile(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryProfileCreated,
            item_kind: "memory_profile",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_profile_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_profile(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryProfileRecord> {
        self.authorize(
            "agent.business.memory.profile.retrieve",
            command.requested_by,
            format!("agent.business.memory.profile.{}", command.item_id),
            "memory.profile.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
        )?;
        self.get_active_memory_profile(command.tenant_id, command.item_id.as_str())
    }

    pub fn create_memory_binding(
        &mut self,
        command: AgentMemoryBindingCreateCommand,
    ) -> KernelResult<AgentMemoryBindingRecord> {
        self.authorize(
            "agent.business.memory.binding.create",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.binding.{}",
                command.memory_binding_id
            ),
            "memory.binding.create",
        )?;
        validate_standard_id(
            command.memory_binding_id.as_str(),
            "memoryBindingId",
            Some("memory.binding."),
        )?;
        validate_standard_id(
            command.memory_profile_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
        )?;
        validate_optional_agent_id(command.agent_id.as_deref())?;
        validate_optional_standard_ref(
            command.deployment_id.as_deref(),
            "deploymentId",
            "deployment.",
        )?;
        validate_safe_text_field(
            command.scope_ref.as_str(),
            "scopeRef",
            MAX_MEMORY_SCOPE_REF_CHARS,
        )?;
        validate_memory_binding_scope(
            command.scope_kind,
            command.scope_ref.as_str(),
            command.agent_id.as_deref(),
            command.deployment_id.as_deref(),
        )?;
        self.get_active_memory_profile(command.tenant_id, command.memory_profile_id.as_str())?;
        let record = AgentMemoryBindingRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            memory_binding_id: command.memory_binding_id,
            memory_profile_id: command.memory_profile_id,
            agent_id: command.agent_id,
            deployment_id: command.deployment_id,
            scope_kind: command.scope_kind,
            scope_ref: command.scope_ref,
            active: command.active,
            default_binding: command.default_binding,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
        };
        self.repository.insert_memory_binding(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryBindingCreated,
            item_kind: "memory_binding",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_binding_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_binding(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryBindingRecord> {
        self.authorize(
            "agent.business.memory.binding.retrieve",
            command.requested_by,
            format!("agent.business.memory.binding.{}", command.item_id),
            "memory.binding.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryBindingId",
            Some("memory.binding."),
        )?;
        let record = self
            .repository
            .get_memory_binding(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory binding not found"))?;
        validate_safe_text_field(
            record.scope_ref.as_str(),
            "scopeRef",
            MAX_MEMORY_SCOPE_REF_CHARS,
        )?;
        validate_memory_binding_scope(
            record.scope_kind,
            record.scope_ref.as_str(),
            record.agent_id.as_deref(),
            record.deployment_id.as_deref(),
        )?;
        self.get_active_memory_profile(command.tenant_id, record.memory_profile_id.as_str())?;
        Ok(record)
    }

    pub fn create_memory_namespace(
        &mut self,
        command: AgentMemoryNamespaceCreateCommand,
    ) -> KernelResult<AgentMemoryNamespaceRecord> {
        self.authorize(
            "agent.business.memory.namespace.create",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.namespace.{}",
                command.memory_namespace_id
            ),
            "memory.namespace.create",
        )?;
        validate_standard_id(
            command.memory_namespace_id.as_str(),
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        validate_optional_agent_id(command.agent_id.as_deref())?;
        validate_optional_plain_ref(command.user_ref.as_deref(), "userRef")?;
        validate_optional_plain_ref(command.session_ref.as_deref(), "sessionRef")?;
        validate_optional_plain_ref(command.thread_ref.as_deref(), "threadRef")?;
        let record = AgentMemoryNamespaceRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            memory_namespace_id: command.memory_namespace_id,
            agent_id: command.agent_id,
            user_ref: command.user_ref,
            session_ref: command.session_ref,
            thread_ref: command.thread_ref,
            namespace_kind: command.namespace_kind,
            status: AgentBusinessStatus::Active,
            visibility: command.visibility,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
        };
        self.repository.insert_memory_namespace(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryNamespaceCreated,
            item_kind: "memory_namespace",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_namespace_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_namespace(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryNamespaceRecord> {
        self.authorize(
            "agent.business.memory.namespace.retrieve",
            command.requested_by,
            format!("agent.business.memory.namespace.{}", command.item_id),
            "memory.namespace.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        self.get_active_memory_namespace(command.tenant_id, command.item_id.as_str())
    }

    pub fn create_memory_record(
        &mut self,
        command: AgentMemoryRecordCreateCommand,
    ) -> KernelResult<AgentMemoryRecord> {
        self.authorize(
            "agent.business.memory.record.create",
            command.requested_by.clone(),
            format!("agent.business.memory.record.{}", command.memory_id),
            "memory.record.create",
        )?;
        validate_standard_id(
            command.memory_id.as_str(),
            "memoryId",
            Some("memory.record."),
        )?;
        validate_standard_id(
            command.memory_namespace_id.as_str(),
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        validate_optional_agent_id(command.agent_id.as_deref())?;
        validate_non_empty(command.content_format.as_str(), "contentFormat")?;
        validate_marketplace_json(command.content_json.as_str(), "contentJson")?;
        validate_score(command.salience_score, "salienceScore")?;
        validate_score(command.confidence_score, "confidenceScore")?;
        validate_score(command.freshness_score, "freshnessScore")?;
        if command.sensitivity_level < 0 || command.sensitivity_level > 4 {
            return Err(KernelError::validation(
                "sensitivityLevel must be between 0 and 4",
            ));
        }
        self.get_active_memory_namespace(command.tenant_id, command.memory_namespace_id.as_str())?;
        let record = AgentMemoryRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            organization_id: command.organization_id,
            memory_id: command.memory_id,
            memory_namespace_id: command.memory_namespace_id,
            agent_id: command.agent_id,
            memory_kind: command.memory_kind,
            content_format: command.content_format,
            content_json: command.content_json,
            summary: command.summary,
            salience_score: command.salience_score,
            confidence_score: command.confidence_score,
            freshness_score: command.freshness_score,
            sensitivity_level: command.sensitivity_level,
            source_count: 0,
            effective_at: command.effective_at,
            expires_at: command.expires_at,
            last_used_at: None,
            use_count: 0,
            status: AgentBusinessStatus::Active,
            version: 1,
            created_at: command.requested_at.clone(),
            updated_at: command.requested_at.clone(),
            deleted_at: None,
            redacted_at: None,
        };
        self.repository.insert_memory_record(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRecordCreated,
            item_kind: "memory_record",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_record(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryRecord> {
        self.authorize(
            "agent.business.memory.record.retrieve",
            command.requested_by,
            format!("agent.business.memory.record.{}", command.item_id),
            "memory.record.retrieve",
        )?;
        validate_standard_id(command.item_id.as_str(), "memoryId", Some("memory.record."))?;
        self.get_active_memory_record(command.tenant_id, command.item_id.as_str())
    }

    pub fn list_memory_records(
        &mut self,
        tenant_id: u64,
        memory_namespace_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentMemoryRecord>> {
        self.authorize(
            "agent.business.memory.record.list",
            requested_by,
            format!("agent.business.memory.namespace.{memory_namespace_id}"),
            "memory.record.list",
        )?;
        validate_standard_id(
            memory_namespace_id,
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        self.get_active_memory_namespace(tenant_id, memory_namespace_id)?;
        Ok(self
            .repository
            .list_memory_records(tenant_id, memory_namespace_id))
    }

    pub fn delete_memory_record(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryRecord> {
        self.authorize(
            "agent.business.memory.record.delete",
            command.requested_by.clone(),
            format!("agent.business.memory.record.{}", command.item_id),
            "memory.record.delete",
        )?;
        validate_standard_id(command.item_id.as_str(), "memoryId", Some("memory.record."))?;
        let mut record = self
            .repository
            .get_memory_record(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory record not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory record",
        )?;
        self.get_active_memory_namespace(command.tenant_id, record.memory_namespace_id.as_str())?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_memory_record(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRecordDeleted,
            item_kind: "memory_record",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_memory_record(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryRecord> {
        self.authorize(
            "agent.business.memory.record.restore",
            command.requested_by.clone(),
            format!("agent.business.memory.record.{}", command.item_id),
            "memory.record.restore",
        )?;
        validate_standard_id(command.item_id.as_str(), "memoryId", Some("memory.record."))?;
        let mut record = self
            .repository
            .get_memory_record(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory record not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory record",
        )?;
        self.get_active_memory_namespace(command.tenant_id, record.memory_namespace_id.as_str())?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_memory_record(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRecordRestored,
            item_kind: "memory_record",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn create_memory_source(
        &mut self,
        command: AgentMemorySourceCreateCommand,
    ) -> KernelResult<AgentMemorySourceRecord> {
        self.authorize(
            "agent.business.memory.source.create",
            command.requested_by.clone(),
            format!("agent.business.memory.source.{}", command.memory_source_id),
            "memory.source.create",
        )?;
        validate_standard_id(
            command.memory_source_id.as_str(),
            "memorySourceId",
            Some("memory.source."),
        )?;
        validate_standard_id(
            command.memory_id.as_str(),
            "memoryId",
            Some("memory.record."),
        )?;
        validate_non_empty(command.source_ref.as_str(), "sourceRef")?;
        validate_non_empty(command.source_hash.as_str(), "sourceHash")?;
        validate_marketplace_json(command.evidence_json.as_str(), "evidenceJson")?;
        self.get_active_memory_record(command.tenant_id, command.memory_id.as_str())?;
        let record = AgentMemorySourceRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            memory_source_id: command.memory_source_id,
            memory_id: command.memory_id,
            source_kind: command.source_kind,
            source_ref: command.source_ref,
            source_hash: command.source_hash,
            evidence_json: command.evidence_json,
            captured_at: command.captured_at,
            created_at: command.requested_at.clone(),
        };
        self.repository.insert_memory_source(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemorySourceCreated,
            item_kind: "memory_source",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_source_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_memory_sources(
        &mut self,
        tenant_id: u64,
        memory_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentMemorySourceRecord>> {
        self.authorize(
            "agent.business.memory.source.list",
            requested_by,
            format!("agent.business.memory.record.{memory_id}"),
            "memory.source.list",
        )?;
        validate_standard_id(memory_id, "memoryId", Some("memory.record."))?;
        self.get_active_memory_record(tenant_id, memory_id)?;
        Ok(self.repository.list_memory_sources(tenant_id, memory_id))
    }

    pub fn create_memory_relation(
        &mut self,
        command: AgentMemoryRelationCreateCommand,
    ) -> KernelResult<AgentMemoryRelationRecord> {
        self.authorize(
            "agent.business.memory.relation.create",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.relation.{}",
                command.memory_relation_id
            ),
            "memory.relation.create",
        )?;
        validate_standard_id(
            command.memory_relation_id.as_str(),
            "memoryRelationId",
            Some("memory.relation."),
        )?;
        validate_standard_id(
            command.from_memory_id.as_str(),
            "fromMemoryId",
            Some("memory.record."),
        )?;
        validate_standard_id(
            command.to_memory_id.as_str(),
            "toMemoryId",
            Some("memory.record."),
        )?;
        validate_score(command.weight, "weight")?;
        self.get_active_memory_record(command.tenant_id, command.from_memory_id.as_str())?;
        self.get_active_memory_record(command.tenant_id, command.to_memory_id.as_str())?;
        let record = AgentMemoryRelationRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            memory_relation_id: command.memory_relation_id,
            from_memory_id: command.from_memory_id,
            to_memory_id: command.to_memory_id,
            relation_kind: command.relation_kind,
            weight: command.weight,
            valid_from: command.valid_from,
            valid_until: command.valid_until,
            created_at: command.requested_at.clone(),
        };
        self.repository.insert_memory_relation(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRelationCreated,
            item_kind: "memory_relation",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_relation_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_memory_relations(
        &mut self,
        tenant_id: u64,
        memory_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentMemoryRelationRecord>> {
        self.authorize(
            "agent.business.memory.relation.list",
            requested_by,
            format!("agent.business.memory.record.{memory_id}"),
            "memory.relation.list",
        )?;
        validate_standard_id(memory_id, "memoryId", Some("memory.record."))?;
        self.get_active_memory_record(tenant_id, memory_id)?;
        Ok(self.repository.list_memory_relations(tenant_id, memory_id))
    }

    pub fn upsert_memory_retrieval_index(
        &mut self,
        command: AgentMemoryRetrievalIndexUpsertCommand,
    ) -> KernelResult<AgentMemoryRetrievalIndexRecord> {
        self.authorize(
            "agent.business.memory.retrieval_index.upsert",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.retrieval_index.{}",
                command.memory_index_id
            ),
            "memory.retrieval_index.upsert",
        )?;
        validate_standard_id(
            command.memory_index_id.as_str(),
            "memoryIndexId",
            Some("memory.index."),
        )?;
        validate_standard_id(
            command.memory_id.as_str(),
            "memoryId",
            Some("memory.record."),
        )?;
        validate_standard_id(
            command.index_provider_id.as_str(),
            "indexProviderId",
            Some("provider."),
        )?;
        validate_non_empty(command.external_ref.as_str(), "externalRef")?;
        validate_non_empty(command.content_hash.as_str(), "contentHash")?;
        if command.index_kind == AgentMemoryIndexKind::Vector
            && (command.embedding_model_id.is_none() || command.vector_dimension.is_none())
        {
            return Err(KernelError::validation(
                "vector memory index requires embeddingModelId and vectorDimension",
            ));
        }
        self.get_active_memory_record(command.tenant_id, command.memory_id.as_str())?;
        let record = AgentMemoryRetrievalIndexRecord {
            id: self.repository.next_id()?,
            tenant_id: command.tenant_id,
            memory_index_id: command.memory_index_id,
            memory_id: command.memory_id,
            index_kind: command.index_kind,
            index_provider_id: command.index_provider_id,
            external_ref: command.external_ref,
            embedding_model_id: command.embedding_model_id,
            vector_dimension: command.vector_dimension,
            content_hash: command.content_hash,
            indexed_at: command.requested_at.clone(),
            status: AgentBusinessStatus::Active,
        };
        self.repository
            .upsert_memory_retrieval_index(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRetrievalIndexUpserted,
            item_kind: "memory_retrieval_index",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_index_id.as_str(),
            status: record.status,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn list_memory_retrieval_indexes(
        &mut self,
        tenant_id: u64,
        memory_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentMemoryRetrievalIndexRecord>> {
        self.authorize(
            "agent.business.memory.retrieval_index.list",
            requested_by,
            format!("agent.business.memory.record.{memory_id}"),
            "memory.retrieval_index.list",
        )?;
        validate_standard_id(memory_id, "memoryId", Some("memory.record."))?;
        self.get_active_memory_record(tenant_id, memory_id)?;
        Ok(self
            .repository
            .list_memory_retrieval_indexes(tenant_id, memory_id))
    }

    pub fn list_memory_stores(
        &mut self,
        tenant_id: u64,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<AgentMemoryStoreRecord>> {
        self.authorize(
            "agent.business.memory.store.list",
            requested_by,
            format!("agent.business.memory.store.tenant.{tenant_id}"),
            "memory.store.list",
        )?;
        Ok(self.repository.list_memory_stores(tenant_id))
    }

    pub fn update_memory_profile(
        &mut self,
        command: AgentMemoryProfileCreateCommand,
    ) -> KernelResult<AgentMemoryProfileRecord> {
        self.authorize(
            "agent.business.memory.profile.update",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.profile.{}",
                command.memory_profile_id
            ),
            "memory.profile.update",
        )?;
        validate_standard_id(
            command.memory_profile_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
        )?;
        let mut record = self
            .repository
            .get_memory_profile(command.tenant_id, command.memory_profile_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory profile not found"))?;
        ensure_marketplace_update_allowed(
            record.is_deleted(),
            record.version,
            None,
            "agent memory profile",
        )?;
        validate_standard_id(
            command.memory_store_id.as_str(),
            "memoryStoreId",
            Some("memory.store."),
        )?;
        for (value, field) in [
            (command.write_policy_json.as_str(), "writePolicyJson"),
            (
                command.retrieval_policy_json.as_str(),
                "retrievalPolicyJson",
            ),
            (
                command.compaction_policy_json.as_str(),
                "compactionPolicyJson",
            ),
            (
                command.retention_policy_json.as_str(),
                "retentionPolicyJson",
            ),
            (command.privacy_policy_json.as_str(), "privacyPolicyJson"),
        ] {
            validate_marketplace_json(value, field)?;
        }
        self.get_active_memory_store(command.tenant_id, command.memory_store_id.as_str())?;

        record.organization_id = command.organization_id;
        record.owner_user_id = command.owner_user_id;
        record.code = command.code;
        record.display_name = command.display_name;
        record.description = command.description;
        record.memory_store_id = command.memory_store_id;
        record.write_policy_json = command.write_policy_json;
        record.retrieval_policy_json = command.retrieval_policy_json;
        record.compaction_policy_json = command.compaction_policy_json;
        record.retention_policy_json = command.retention_policy_json;
        record.privacy_policy_json = command.privacy_policy_json;
        record.visibility = command.visibility;
        record.mark_updated(command.requested_at.clone());

        self.repository.update_memory_profile(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryProfileUpdated,
            item_kind: "memory_profile",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_profile_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn delete_memory_profile(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryProfileRecord> {
        self.authorize(
            "agent.business.memory.profile.delete",
            command.requested_by.clone(),
            format!("agent.business.memory.profile.{}", command.item_id),
            "memory.profile.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
        )?;
        let mut record = self
            .repository
            .get_memory_profile(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory profile not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory profile",
        )?;
        self.get_active_memory_store(command.tenant_id, record.memory_store_id.as_str())?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_memory_profile(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryProfileDeleted,
            item_kind: "memory_profile",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_profile_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_memory_profile(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryProfileRecord> {
        self.authorize(
            "agent.business.memory.profile.restore",
            command.requested_by.clone(),
            format!("agent.business.memory.profile.{}", command.item_id),
            "memory.profile.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
        )?;
        let mut record = self
            .repository
            .get_memory_profile(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory profile not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory profile",
        )?;
        self.get_active_memory_store(command.tenant_id, record.memory_store_id.as_str())?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_memory_profile(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryProfileRestored,
            item_kind: "memory_profile",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_profile_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn update_memory_binding(
        &mut self,
        command: AgentMemoryBindingCreateCommand,
    ) -> KernelResult<AgentMemoryBindingRecord> {
        self.authorize(
            "agent.business.memory.binding.update",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.binding.{}",
                command.memory_binding_id
            ),
            "memory.binding.update",
        )?;
        validate_standard_id(
            command.memory_binding_id.as_str(),
            "memoryBindingId",
            Some("memory.binding."),
        )?;
        let mut record = self
            .repository
            .get_memory_binding(command.tenant_id, command.memory_binding_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory binding not found"))?;
        validate_standard_id(
            command.memory_profile_id.as_str(),
            "memoryProfileId",
            Some("memory.profile."),
        )?;
        validate_optional_agent_id(command.agent_id.as_deref())?;
        validate_optional_standard_ref(
            command.deployment_id.as_deref(),
            "deploymentId",
            "deployment.",
        )?;
        validate_safe_text_field(
            command.scope_ref.as_str(),
            "scopeRef",
            MAX_MEMORY_SCOPE_REF_CHARS,
        )?;
        validate_memory_binding_scope(
            command.scope_kind,
            command.scope_ref.as_str(),
            command.agent_id.as_deref(),
            command.deployment_id.as_deref(),
        )?;
        self.get_active_memory_profile(command.tenant_id, command.memory_profile_id.as_str())?;

        record.organization_id = command.organization_id;
        record.memory_profile_id = command.memory_profile_id;
        record.agent_id = command.agent_id;
        record.deployment_id = command.deployment_id;
        record.scope_kind = command.scope_kind;
        record.scope_ref = command.scope_ref;
        record.active = command.active;
        record.default_binding = command.default_binding;
        record.mark_updated(command.requested_at.clone());

        self.repository.update_memory_binding(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryBindingUpdated,
            item_kind: "memory_binding",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_binding_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn delete_memory_binding(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryBindingRecord> {
        self.authorize(
            "agent.business.memory.binding.delete",
            command.requested_by.clone(),
            format!("agent.business.memory.binding.{}", command.item_id),
            "memory.binding.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryBindingId",
            Some("memory.binding."),
        )?;
        let record = self
            .repository
            .get_memory_binding(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory binding not found"))?;
        self.get_active_memory_profile(command.tenant_id, record.memory_profile_id.as_str())?;
        let mut updated = record.clone();
        updated.active = false;
        updated.mark_updated(command.requested_at.clone());
        self.repository.update_memory_binding(updated.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryBindingDeleted,
            item_kind: "memory_binding",
            tenant_id: updated.tenant_id,
            organization_id: updated.organization_id,
            item_id: updated.memory_binding_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: updated.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(updated)
    }

    pub fn restore_memory_binding(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryBindingRecord> {
        self.authorize(
            "agent.business.memory.binding.restore",
            command.requested_by.clone(),
            format!("agent.business.memory.binding.{}", command.item_id),
            "memory.binding.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryBindingId",
            Some("memory.binding."),
        )?;
        let record = self
            .repository
            .get_memory_binding(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory binding not found"))?;
        self.get_active_memory_profile(command.tenant_id, record.memory_profile_id.as_str())?;
        let mut updated = record.clone();
        updated.active = true;
        updated.mark_updated(command.requested_at.clone());
        self.repository.update_memory_binding(updated.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryBindingRestored,
            item_kind: "memory_binding",
            tenant_id: updated.tenant_id,
            organization_id: updated.organization_id,
            item_id: updated.memory_binding_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: updated.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(updated)
    }

    pub fn update_memory_namespace(
        &mut self,
        command: AgentMemoryNamespaceCreateCommand,
    ) -> KernelResult<AgentMemoryNamespaceRecord> {
        self.authorize(
            "agent.business.memory.namespace.update",
            command.requested_by.clone(),
            format!(
                "agent.business.memory.namespace.{}",
                command.memory_namespace_id
            ),
            "memory.namespace.update",
        )?;
        validate_standard_id(
            command.memory_namespace_id.as_str(),
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        let mut record = self
            .repository
            .get_memory_namespace(command.tenant_id, command.memory_namespace_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory namespace not found"))?;
        validate_optional_agent_id(command.agent_id.as_deref())?;
        validate_optional_plain_ref(command.user_ref.as_deref(), "userRef")?;
        validate_optional_plain_ref(command.session_ref.as_deref(), "sessionRef")?;
        validate_optional_plain_ref(command.thread_ref.as_deref(), "threadRef")?;

        record.organization_id = command.organization_id;
        record.agent_id = command.agent_id;
        record.user_ref = command.user_ref;
        record.session_ref = command.session_ref;
        record.thread_ref = command.thread_ref;
        record.namespace_kind = command.namespace_kind;
        record.visibility = command.visibility;
        record.mark_updated(command.requested_at.clone());

        self.repository.update_memory_namespace(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryNamespaceUpdated,
            item_kind: "memory_namespace",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_namespace_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn delete_memory_namespace(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryNamespaceRecord> {
        self.authorize(
            "agent.business.memory.namespace.delete",
            command.requested_by.clone(),
            format!("agent.business.memory.namespace.{}", command.item_id),
            "memory.namespace.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        let mut record = self
            .repository
            .get_memory_namespace(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory namespace not found"))?;
        ensure_marketplace_delete_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory namespace",
        )?;
        record.mark_deleted(command.requested_at.clone());
        self.repository.update_memory_namespace(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryNamespaceDeleted,
            item_kind: "memory_namespace",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_namespace_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_memory_namespace(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryNamespaceRecord> {
        self.authorize(
            "agent.business.memory.namespace.restore",
            command.requested_by.clone(),
            format!("agent.business.memory.namespace.{}", command.item_id),
            "memory.namespace.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryNamespaceId",
            Some("memory.namespace."),
        )?;
        let mut record = self
            .repository
            .get_memory_namespace(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory namespace not found"))?;
        ensure_marketplace_restore_allowed(
            record.is_deleted(),
            record.version,
            command.expected_version,
            "agent memory namespace",
        )?;
        record.mark_restored(command.requested_at.clone());
        self.repository.update_memory_namespace(record.clone())?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryNamespaceRestored,
            item_kind: "memory_namespace",
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            item_id: record.memory_namespace_id.as_str(),
            status: record.status,
            visibility: record.visibility,
            version: record.version,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_source(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemorySourceRecord> {
        self.authorize(
            "agent.business.memory.source.retrieve",
            command.requested_by,
            format!("agent.business.memory.source.{}", command.item_id),
            "memory.source.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memorySourceId",
            Some("memory.source."),
        )?;
        self.repository
            .get_memory_source(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory source not found"))
    }

    pub fn delete_memory_source(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemorySourceRecord> {
        self.authorize(
            "agent.business.memory.source.delete",
            command.requested_by.clone(),
            format!("agent.business.memory.source.{}", command.item_id),
            "memory.source.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memorySourceId",
            Some("memory.source."),
        )?;
        let record = self
            .repository
            .get_memory_source(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory source not found"))?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemorySourceDeleted,
            item_kind: "memory_source",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_source_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_memory_source(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemorySourceRecord> {
        self.authorize(
            "agent.business.memory.source.restore",
            command.requested_by.clone(),
            format!("agent.business.memory.source.{}", command.item_id),
            "memory.source.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memorySourceId",
            Some("memory.source."),
        )?;
        let record = self
            .repository
            .get_memory_source(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory source not found"))?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemorySourceRestored,
            item_kind: "memory_source",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_source_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn get_memory_relation(
        &mut self,
        command: GetAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryRelationRecord> {
        self.authorize(
            "agent.business.memory.relation.retrieve",
            command.requested_by,
            format!("agent.business.memory.relation.{}", command.item_id),
            "memory.relation.retrieve",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryRelationId",
            Some("memory.relation."),
        )?;
        self.repository
            .get_memory_relation(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory relation not found"))
    }

    pub fn delete_memory_relation(
        &mut self,
        command: DeleteAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryRelationRecord> {
        self.authorize(
            "agent.business.memory.relation.delete",
            command.requested_by.clone(),
            format!("agent.business.memory.relation.{}", command.item_id),
            "memory.relation.delete",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryRelationId",
            Some("memory.relation."),
        )?;
        let record = self
            .repository
            .get_memory_relation(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory relation not found"))?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRelationDeleted,
            item_kind: "memory_relation",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_relation_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn restore_memory_relation(
        &mut self,
        command: RestoreAgentMarketplaceItemCommand,
    ) -> KernelResult<AgentMemoryRelationRecord> {
        self.authorize(
            "agent.business.memory.relation.restore",
            command.requested_by.clone(),
            format!("agent.business.memory.relation.{}", command.item_id),
            "memory.relation.restore",
        )?;
        validate_standard_id(
            command.item_id.as_str(),
            "memoryRelationId",
            Some("memory.relation."),
        )?;
        let record = self
            .repository
            .get_memory_relation(command.tenant_id, command.item_id.as_str())
            .ok_or_else(|| KernelError::validation("agent memory relation not found"))?;
        self.emit_memory_audit_event(AgentBusinessAuditEventInput {
            action: AgentAuditAction::MemoryRelationRestored,
            item_kind: "memory_relation",
            tenant_id: record.tenant_id,
            organization_id: 0,
            item_id: record.memory_relation_id.as_str(),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            version: 1,
            subject: command.requested_by,
            occurred_at: command.requested_at,
        })?;
        Ok(record)
    }

    pub fn update_agent(
        &mut self,
        command: UpdateAgentCommand,
    ) -> KernelResult<AgentBusinessRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        let policy_resource = format!("agent.business.{}", command.agent_id);
        self.authorize(
            "agent.business.update",
            command.requested_by.clone(),
            policy_resource,
            "update",
        )?;

        let mut record = self
            .repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;

        if record.is_deleted() {
            return Err(KernelError::validation("deleted agent cannot be updated"));
        }
        ensure_expected_version(record.version, command.expected_version, "agent")?;

        if let Some(display_name) = command.display_name {
            if display_name.trim().is_empty() {
                return Err(KernelError::validation(
                    "agent display_name cannot be empty",
                ));
            }
            record.display_name = display_name;
        }
        if let Some(description) = command.description {
            record.description = Some(description);
        }
        if let Some(manifest) = command.manifest {
            record.manifest = manifest;
        }
        if let Some(visibility) = command.visibility {
            record.visibility = visibility;
        }
        if let Some(tags) = command.tags {
            record.tags = tags;
        }
        if let Some(intent) = command.default_code_task_intent {
            record.default_code_task_intent = Some(intent);
        }
        if let Some(provider_id) = command.implementation_provider_id {
            if let Some(provider_id) = provider_id.as_deref() {
                validate_standard_id(provider_id, "implementationProviderId", Some("provider."))?;
            }
            record.implementation_provider_id = provider_id;
        }
        if let Some(implementation_kind) = command.implementation_kind {
            record.implementation_kind = implementation_kind;
        }
        if let Some(implementation_type) = command.implementation_type {
            record.implementation_type = implementation_type;
        }
        record.mark_updated(command.requested_at.clone());

        self.repository.update(record.clone())?;
        self.emit_audit_event(
            AgentAuditAction::Update,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn change_status(
        &mut self,
        command: ChangeAgentStatusCommand,
    ) -> KernelResult<AgentBusinessRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        let policy_resource = format!("agent.business.{}", command.agent_id);
        self.authorize(
            "agent.business.status.update",
            command.requested_by.clone(),
            policy_resource,
            "change_status",
        )?;

        let mut record = self
            .repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;

        if record.is_deleted() {
            return Err(KernelError::validation(
                "deleted agent status cannot be changed",
            ));
        }
        ensure_expected_version(record.version, command.expected_version, "agent")?;

        if !is_valid_status_transition(record.status, command.target_status) {
            return Err(KernelError::validation("invalid agent status transition"));
        }

        record.status = command.target_status;
        record.mark_updated(command.requested_at.clone());
        self.repository.update(record.clone())?;
        self.emit_audit_event(
            AgentAuditAction::ChangeStatus,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn delete_agent(
        &mut self,
        command: DeleteAgentCommand,
    ) -> KernelResult<AgentBusinessRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        let policy_resource = format!("agent.business.{}", command.agent_id);
        self.authorize(
            "agent.business.delete",
            command.requested_by.clone(),
            policy_resource,
            "delete",
        )?;

        let mut record = self
            .repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;

        if record.is_deleted() {
            return Err(KernelError::validation("agent already deleted"));
        }
        ensure_expected_version(record.version, command.expected_version, "agent")?;

        record.mark_deleted(command.requested_at.clone());
        self.repository.update(record.clone())?;
        self.emit_audit_event(
            AgentAuditAction::Delete,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn restore_agent(
        &mut self,
        command: RestoreAgentCommand,
    ) -> KernelResult<AgentBusinessRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        let policy_resource = format!("agent.business.{}", command.agent_id);
        self.authorize(
            "agent.business.restore",
            command.requested_by.clone(),
            policy_resource,
            "restore",
        )?;

        let mut record = self
            .repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))?;

        if !record.is_deleted() {
            return Err(KernelError::validation("agent is not deleted"));
        }
        ensure_expected_version(record.version, command.expected_version, "agent")?;

        record.mark_restored(command.requested_at.clone());
        self.repository.update(record.clone())?;
        self.emit_audit_event(
            AgentAuditAction::Restore,
            &record,
            command.requested_by,
            command.requested_at,
        )?;
        Ok(record)
    }

    pub fn get_agent(&mut self, command: GetAgentCommand) -> KernelResult<AgentBusinessRecord> {
        validate_agent_id(command.agent_id.as_str())?;
        let policy_resource = format!("agent.business.{}", command.agent_id);
        self.authorize(
            "agent.business.retrieve",
            command.requested_by,
            policy_resource,
            "retrieve",
        )?;

        self.repository
            .get(command.tenant_id, command.agent_id.as_str())
            .ok_or_else(|| KernelError::validation("agent not found"))
    }

    pub fn list_agents(
        &mut self,
        command: ListAgentsCommand,
    ) -> KernelResult<Vec<AgentBusinessRecord>> {
        self.authorize(
            "agent.business.list",
            command.requested_by,
            format!("agent.business.tenant.{}", command.query.tenant_id),
            "list",
        )?;
        Ok(self.repository.list(&command.query))
    }

    pub fn list_agent_audit_events(
        &mut self,
        tenant_id: u64,
        agent_id: &str,
        requested_by: PolicySubject,
    ) -> KernelResult<Vec<KernelEvent>> {
        validate_agent_id(agent_id)?;
        self.authorize(
            "agent.business.audit.read",
            requested_by,
            format!("agent.business.{}", agent_id),
            "audit.read",
        )?;
        self.audit_sink.list_events(tenant_id, agent_id)
    }

    fn authorize(
        &mut self,
        request_id: impl Into<String>,
        subject: PolicySubject,
        resource: impl Into<String>,
        action: impl Into<String>,
    ) -> KernelResult<()> {
        let policy_request = PolicyRequest::new(
            request_id,
            DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY,
            resource,
        )
        .with_category(PolicyCategory::ProductSpecific(
            DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY.to_string(),
        ))
        .with_subject(subject)
        .with_action(action)
        .with_redaction(KernelEventRedaction::TenantSensitive);

        let decision = self.policy_provider.evaluate(policy_request)?;
        if decision.decision != PolicyDecisionValue::Allow {
            return Err(KernelError::permission_required(
                decision
                    .safe_reason
                    .unwrap_or_else(|| "agent management denied".to_string()),
            ));
        }
        Ok(())
    }

    fn get_active_knowledge_base(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> KernelResult<AgentKnowledgeBaseRecord> {
        let record = self
            .repository
            .get_knowledge_base(tenant_id, knowledge_base_id)
            .ok_or_else(|| KernelError::validation("agent knowledge base not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation("agent knowledge base not found"));
        }
        Ok(record)
    }

    fn get_active_memory_store(
        &self,
        tenant_id: u64,
        memory_store_id: &str,
    ) -> KernelResult<AgentMemoryStoreRecord> {
        let record = self
            .repository
            .get_memory_store(tenant_id, memory_store_id)
            .ok_or_else(|| KernelError::validation("agent memory store not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation("agent memory store not found"));
        }
        Ok(record)
    }

    fn get_active_memory_profile(
        &self,
        tenant_id: u64,
        memory_profile_id: &str,
    ) -> KernelResult<AgentMemoryProfileRecord> {
        let record = self
            .repository
            .get_memory_profile(tenant_id, memory_profile_id)
            .ok_or_else(|| KernelError::validation("agent memory profile not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation("agent memory profile not found"));
        }
        self.get_active_memory_store(tenant_id, record.memory_store_id.as_str())?;
        Ok(record)
    }

    fn get_active_memory_namespace(
        &self,
        tenant_id: u64,
        memory_namespace_id: &str,
    ) -> KernelResult<AgentMemoryNamespaceRecord> {
        let record = self
            .repository
            .get_memory_namespace(tenant_id, memory_namespace_id)
            .ok_or_else(|| KernelError::validation("agent memory namespace not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation("agent memory namespace not found"));
        }
        Ok(record)
    }

    fn get_active_memory_record(
        &self,
        tenant_id: u64,
        memory_id: &str,
    ) -> KernelResult<AgentMemoryRecord> {
        let record = self
            .repository
            .get_memory_record(tenant_id, memory_id)
            .ok_or_else(|| KernelError::validation("agent memory record not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation("agent memory record not found"));
        }
        self.get_active_memory_namespace(tenant_id, record.memory_namespace_id.as_str())?;
        Ok(record)
    }

    fn get_active_knowledge_source(
        &self,
        tenant_id: u64,
        knowledge_source_id: &str,
    ) -> KernelResult<AgentKnowledgeSourceRecord> {
        let record = self
            .repository
            .get_knowledge_source(tenant_id, knowledge_source_id)
            .ok_or_else(|| KernelError::validation("agent knowledge source not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation("agent knowledge source not found"));
        }
        self.get_active_knowledge_base(tenant_id, record.knowledge_base_id.as_str())?;
        Ok(record)
    }

    fn get_active_knowledge_sync_job(
        &self,
        tenant_id: u64,
        sync_job_id: &str,
    ) -> KernelResult<AgentKnowledgeSyncJobRecord> {
        validate_standard_id(sync_job_id, "syncJobId", Some("knowledge.sync."))?;
        let record = self
            .repository
            .get_knowledge_sync_job(tenant_id, sync_job_id)
            .ok_or_else(|| KernelError::validation("agent knowledge sync job not found"))?;
        let base = self.get_active_knowledge_base(tenant_id, record.knowledge_base_id.as_str())?;
        if let Some(source_id) = record.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }
        Ok(record)
    }

    fn get_active_knowledge_document(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> KernelResult<AgentKnowledgeDocumentRecord> {
        let record = self
            .repository
            .get_knowledge_document(tenant_id, knowledge_document_id)
            .ok_or_else(|| KernelError::validation("agent knowledge document not found"))?;
        if record.is_deleted() {
            return Err(KernelError::validation(
                "agent knowledge document not found",
            ));
        }
        let base = self.get_active_knowledge_base(tenant_id, record.knowledge_base_id.as_str())?;
        if let Some(source_id) = record.knowledge_source_id.as_deref() {
            let source = self.get_active_knowledge_source(tenant_id, source_id)?;
            if source.knowledge_base_id != base.knowledge_base_id {
                return Err(KernelError::validation(
                    "agent knowledge source does not belong to knowledge base",
                ));
            }
        }
        Ok(record)
    }

    fn get_active_knowledge_chunk(
        &self,
        tenant_id: u64,
        knowledge_chunk_id: &str,
    ) -> KernelResult<AgentKnowledgeChunkRecord> {
        let record = self
            .repository
            .get_knowledge_chunk(tenant_id, knowledge_chunk_id)
            .ok_or_else(|| KernelError::validation("agent knowledge chunk not found"))?;
        if record.status == AgentBusinessStatus::Deleted {
            return Err(KernelError::validation("agent knowledge chunk not found"));
        }
        self.get_active_knowledge_document(tenant_id, record.knowledge_document_id.as_str())?;
        Ok(record)
    }

    fn emit_audit_event(
        &mut self,
        action: AgentAuditAction,
        record: &AgentBusinessRecord,
        subject: PolicySubject,
        occurred_at: String,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};agent_id={};tenant_id={};organization_id={};owner_user_id={};status={};visibility={}",
            action.event_type(),
            record.agent_id,
            record.tenant_id,
            record.organization_id,
            record.owner_user_id,
            record.status.as_str(),
            record.visibility.as_str()
        );
        let event = KernelEvent::new(
            format!("agent_audit_{}_{}", record.agent_id, record.version),
            action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", subject.subject_id.as_str())
        .with_context("subject_tenant_id", subject.tenant_id.as_str())
        .occurred_at(occurred_at)
        .with_payload_schema("sdkwork.agent.business.audit.v1");

        self.audit_sink.record(event)
    }

    fn deactivate_provider_bindings(
        &mut self,
        tenant_id: u64,
        agent_id: &str,
        updated_at: String,
    ) -> KernelResult<()> {
        for mut binding in self.repository.list_provider_bindings(tenant_id, agent_id) {
            if binding.active {
                binding.active = false;
                binding.mark_updated(updated_at.clone());
                self.repository.update_provider_binding(binding)?;
            }
        }
        Ok(())
    }

    fn emit_binding_audit_event(
        &mut self,
        action: AgentAuditAction,
        record: &AgentProviderBindingRecord,
        subject: PolicySubject,
        occurred_at: String,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};agent_id={};tenant_id={};binding_id={};provider_id={};implementation_kind={};active={}",
            action.event_type(),
            record.agent_id,
            record.tenant_id,
            record.binding_id,
            record.provider_id,
            record.implementation_kind.as_str(),
            record.active
        );
        let event = KernelEvent::new(
            format!("agent_binding_{}_{}", record.binding_id, record.version),
            action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", subject.subject_id.as_str())
        .with_context("subject_tenant_id", subject.tenant_id.as_str())
        .occurred_at(occurred_at)
        .with_payload_schema("sdkwork.agent.business.provider_binding.v1");

        self.audit_sink.record(event)
    }

    fn emit_deployment_audit_event(
        &mut self,
        action: AgentAuditAction,
        record: &AgentDeploymentRecord,
        subject: PolicySubject,
        occurred_at: String,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};agent_id={};tenant_id={};deployment_id={};binding_id={};provider_id_snapshot={};implementation_kind_snapshot={}",
            action.event_type(),
            record.agent_id,
            record.tenant_id,
            record.deployment_id,
            record.binding_id,
            record.provider_id_snapshot,
            record.implementation_kind_snapshot.as_str()
        );
        let event = KernelEvent::new(
            format!(
                "agent_deployment_{}_{}",
                record.deployment_id, record.version
            ),
            action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", subject.subject_id.as_str())
        .with_context("subject_tenant_id", subject.tenant_id.as_str())
        .occurred_at(occurred_at)
        .with_payload_schema("sdkwork.agent.business.deployment.v1");

        self.audit_sink.record(event)
    }

    fn emit_runtime_execution_audit_event(
        &mut self,
        action: AgentAuditAction,
        record: &AgentRuntimeExecutionRecord,
        subject: PolicySubject,
        occurred_at: String,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};agent_id={};tenant_id={};execution_id={};operation={};status={}",
            action.event_type(),
            record.agent_id,
            record.tenant_id,
            record.execution_id,
            record.operation.as_str(),
            record.status.as_str()
        );
        let event = KernelEvent::new(
            format!("agent_runtime_execution_{}", record.execution_id),
            action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", subject.subject_id.as_str())
        .with_context("subject_tenant_id", subject.tenant_id.as_str())
        .occurred_at(occurred_at)
        .with_payload_schema("sdkwork.agent.business.runtime_execution.v1");

        self.audit_sink.record(event)
    }

    fn emit_marketplace_audit_event(
        &mut self,
        input: AgentBusinessAuditEventInput<'_>,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};item_kind={};item_id={};tenant_id={};organization_id={};status={};visibility={}",
            input.action.event_type(),
            input.item_kind,
            input.item_id,
            input.tenant_id,
            input.organization_id,
            input.status.as_str(),
            input.visibility.as_str()
        );
        let event = KernelEvent::new(
            format!(
                "agent_marketplace_{}_{}_{}",
                input.item_kind, input.item_id, input.version
            ),
            input.action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", input.subject.subject_id.as_str())
        .with_context("subject_tenant_id", input.subject.tenant_id.as_str())
        .occurred_at(input.occurred_at)
        .with_payload_schema("sdkwork.agent.business.marketplace.v1");

        self.audit_sink.record(event)
    }

    fn emit_memory_audit_event(
        &mut self,
        input: AgentBusinessAuditEventInput<'_>,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};item_kind={};item_id={};tenant_id={};organization_id={};status={};visibility={}",
            input.action.event_type(),
            input.item_kind,
            input.item_id,
            input.tenant_id,
            input.organization_id,
            input.status.as_str(),
            input.visibility.as_str()
        );
        let event = KernelEvent::new(
            format!(
                "agent_memory_{}_{}_{}",
                input.item_kind, input.item_id, input.version
            ),
            input.action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", input.subject.subject_id.as_str())
        .with_context("subject_tenant_id", input.subject.tenant_id.as_str())
        .occurred_at(input.occurred_at)
        .with_payload_schema("sdkwork.agent.business.memory.v1");

        self.audit_sink.record(event)
    }

    fn emit_knowledge_audit_event(
        &mut self,
        input: AgentBusinessAuditEventInput<'_>,
    ) -> KernelResult<()> {
        let payload = format!(
            "action={};item_kind={};item_id={};tenant_id={};organization_id={};status={};visibility={}",
            input.action.event_type(),
            input.item_kind,
            input.item_id,
            input.tenant_id,
            input.organization_id,
            input.status.as_str(),
            input.visibility.as_str()
        );
        let event = KernelEvent::new(
            format!(
                "agent_knowledge_{}_{}_{}",
                input.item_kind, input.item_id, input.version
            ),
            input.action.event_type(),
            KernelEventSeverity::Info,
            payload,
        )
        .from_source(KernelEventSource::Runtime)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_context("subject_id", input.subject.subject_id.as_str())
        .with_context("subject_tenant_id", input.subject.tenant_id.as_str())
        .occurred_at(input.occurred_at)
        .with_payload_schema("sdkwork.agent.business.knowledge.v1");

        self.audit_sink.record(event)
    }
}

trait KernelEventExt {
    fn with_context(self, key: impl Into<String>, value: impl Into<String>) -> Self;
}

impl KernelEventExt for KernelEvent {
    fn with_context(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let payload = format!("{};{}={}", self.payload, key.into(), value.into());
        KernelEvent { payload, ..self }
    }
}

fn is_valid_status_transition(from: AgentBusinessStatus, to: AgentBusinessStatus) -> bool {
    matches!(
        (from, to),
        (AgentBusinessStatus::Draft, AgentBusinessStatus::Active)
            | (AgentBusinessStatus::Draft, AgentBusinessStatus::Archived)
            | (AgentBusinessStatus::Active, AgentBusinessStatus::Disabled)
            | (AgentBusinessStatus::Active, AgentBusinessStatus::Archived)
            | (AgentBusinessStatus::Disabled, AgentBusinessStatus::Active)
            | (AgentBusinessStatus::Disabled, AgentBusinessStatus::Archived)
            | (AgentBusinessStatus::Archived, AgentBusinessStatus::Active)
            | (AgentBusinessStatus::Archived, AgentBusinessStatus::Disabled)
            | (AgentBusinessStatus::Deleted, AgentBusinessStatus::Active)
            | (_, AgentBusinessStatus::Deleted)
    ) || from == to
}

fn knowledge_sync_job_audit_sequence(status: AgentKnowledgeSyncJobStatus) -> u64 {
    match status {
        AgentKnowledgeSyncJobStatus::Queued => 1,
        AgentKnowledgeSyncJobStatus::Running => 2,
        AgentKnowledgeSyncJobStatus::Succeeded => 3,
        AgentKnowledgeSyncJobStatus::Failed => 4,
        AgentKnowledgeSyncJobStatus::Cancelled => 5,
    }
}

fn validate_marketplace_identity(
    item_id: &str,
    id_field_name: &str,
    required_prefix: Option<&str>,
    code: &str,
    display_name: &str,
) -> KernelResult<()> {
    validate_standard_id(item_id, id_field_name, required_prefix)?;
    validate_non_empty(code, "code")?;
    validate_non_empty(display_name, "displayName")?;
    if code.trim() != code {
        return Err(KernelError::validation(
            "code must not contain leading or trailing whitespace",
        ));
    }
    if code.chars().count() > 128 {
        return Err(KernelError::validation(
            "code must be at most 128 characters",
        ));
    }
    if !code
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_'))
    {
        return Err(KernelError::validation(
            "code must use lowercase slug characters",
        ));
    }
    Ok(())
}

fn validate_non_empty(value: &str, field_name: &str) -> KernelResult<()> {
    if value.trim().is_empty() {
        return Err(KernelError::validation(format!("{field_name} is required")));
    }
    Ok(())
}

fn validate_json_payload(value: &str, field_name: &str) -> KernelResult<()> {
    let _: serde_json::Value = serde_json::from_str(value).map_err(|error| {
        KernelError::validation(format!("{field_name} must be valid JSON: {error}"))
    })?;
    Ok(())
}

fn normalize_prompt_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn validate_marketplace_json(value: &str, field_name: &str) -> KernelResult<()> {
    validate_non_empty(value, field_name)?;
    reject_secret_material(value, field_name)?;
    let _: serde_json::Value = serde_json::from_str(value).map_err(|error| {
        KernelError::validation(format!("{field_name} must be valid JSON: {error}"))
    })?;
    Ok(())
}

fn validate_marketplace_labels(values: &[String], field_name: &str) -> KernelResult<()> {
    let mut seen = HashSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain empty values"
            )));
        }
        if value.trim() != value {
            return Err(KernelError::validation(format!(
                "{field_name} values must not contain leading or trailing whitespace"
            )));
        }
        if value.chars().count() > 64 {
            return Err(KernelError::validation(format!(
                "{field_name} values must be at most 64 characters"
            )));
        }
        if !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_'))
        {
            return Err(KernelError::validation(format!(
                "{field_name} values must use lowercase slug characters"
            )));
        }
        if !seen.insert(value.as_str()) {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain duplicate value: {value}"
            )));
        }
    }
    Ok(())
}

fn validate_optional_standard_ref(
    value: Option<&str>,
    field_name: &str,
    required_prefix: &str,
) -> KernelResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    validate_standard_id(value, field_name, Some(required_prefix)).map_err(|_| {
        KernelError::validation(format!("{field_name} must be a standard id reference"))
    })?;
    reject_secret_material(value, field_name)
}

fn reject_secret_material(value: &str, field_name: &str) -> KernelResult<()> {
    let normalized = value.to_lowercase();
    for marker in [
        "api_key=",
        "apikey=",
        "access_token=",
        "refresh_token=",
        "secret=",
        "password=",
        "bearer ",
        "sk-",
    ] {
        if normalized.contains(marker) {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain plaintext secret material"
            )));
        }
    }
    Ok(())
}

fn validate_mcp_transport_reference_pair(
    transport_kind: AgentMcpTransportKind,
    endpoint_ref: Option<&str>,
    command_ref: Option<&str>,
) -> KernelResult<()> {
    match transport_kind {
        AgentMcpTransportKind::Stdio => {
            if command_ref.is_none() {
                return Err(KernelError::validation(
                    "commandRef is required for stdio MCP transport",
                ));
            }
        }
        AgentMcpTransportKind::Http
        | AgentMcpTransportKind::Sse
        | AgentMcpTransportKind::WebSocket => {
            if endpoint_ref.is_none() {
                return Err(KernelError::validation(
                    "endpointRef is required for network MCP transport",
                ));
            }
        }
    }
    Ok(())
}

fn validate_non_empty_memory_modes(values: &[AgentMemoryIndexKind]) -> KernelResult<()> {
    if values.is_empty() {
        return Err(KernelError::validation(
            "retrievalModes must contain at least one mode",
        ));
    }
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value.as_str()) {
            return Err(KernelError::validation(format!(
                "retrievalModes must not contain duplicate mode: {}",
                value.as_str()
            )));
        }
    }
    Ok(())
}

fn validate_non_empty_knowledge_modes(values: &[AgentKnowledgeIndexKind]) -> KernelResult<()> {
    if values.is_empty() {
        return Err(KernelError::validation(
            "retrievalModes must contain at least one mode",
        ));
    }
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value.as_str()) {
            return Err(KernelError::validation(format!(
                "retrievalModes must not contain duplicate mode: {}",
                value.as_str()
            )));
        }
    }
    Ok(())
}

fn knowledge_search_terms(query: &str) -> KernelResult<Vec<String>> {
    validate_non_empty(query, "query")?;
    reject_secret_material(query, "query")?;
    if query.chars().count() > MAX_KNOWLEDGE_SEARCH_QUERY_CHARS {
        return Err(KernelError::validation(format!(
            "query must be at most {MAX_KNOWLEDGE_SEARCH_QUERY_CHARS} characters"
        )));
    }
    let terms: Vec<String> = query
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .filter(|term| !term.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if terms.is_empty() {
        return Err(KernelError::validation(
            "query must contain searchable terms",
        ));
    }
    Ok(terms)
}

fn knowledge_search_score(
    index: &AgentKnowledgeIndexRecord,
    base: &AgentKnowledgeBaseRecord,
    document: Option<&AgentKnowledgeDocumentRecord>,
    chunk: Option<&AgentKnowledgeChunkRecord>,
    source: Option<&AgentKnowledgeSourceRecord>,
    terms: &[String],
) -> f32 {
    let mut score = 0.0;
    let base_text = [
        base.display_name.as_str(),
        base.description.as_deref().unwrap_or_default(),
        base.code.as_str(),
    ]
    .join(" ");
    score += weighted_term_score(&base_text, terms, 0.4);

    if let Some(document) = document {
        score += weighted_term_score(document.title.as_str(), terms, 4.0);
        score += weighted_term_score(document.summary.as_deref().unwrap_or_default(), terms, 3.0);
        score += weighted_term_score(document.tags.join(" ").as_str(), terms, 2.0);
        score += weighted_term_score(document.categories.join(" ").as_str(), terms, 1.2);
        score += weighted_term_score(document.metadata_json.as_str(), terms, 0.7);
        score += weighted_term_score(document.content_ref.as_str(), terms, 0.4);
    }

    if let Some(chunk) = chunk {
        score += weighted_term_score(chunk.heading.as_deref().unwrap_or_default(), terms, 3.5);
        score += weighted_term_score(chunk.summary.as_deref().unwrap_or_default(), terms, 3.0);
        score += weighted_term_score(chunk.metadata_json.as_str(), terms, 0.7);
        score += weighted_term_score(chunk.content_ref.as_str(), terms, 0.5);
    }

    if let Some(source) = source {
        score += weighted_term_score(source.source_ref.as_str(), terms, 0.6);
        score += weighted_term_score(source.metadata_json.as_str(), terms, 0.4);
    }

    score += weighted_term_score(index.external_ref.as_str(), terms, 1.0);
    score += weighted_term_score(index.knowledge_index_id.as_str(), terms, 0.3);

    if score > 0.0 {
        score += match index.index_kind {
            AgentKnowledgeIndexKind::Hybrid => 0.35,
            AgentKnowledgeIndexKind::LlmRerank => 0.3,
            AgentKnowledgeIndexKind::Wiki | AgentKnowledgeIndexKind::Graph => 0.2,
            AgentKnowledgeIndexKind::Keyword | AgentKnowledgeIndexKind::FullText => 0.15,
            AgentKnowledgeIndexKind::Exact => 0.1,
            _ => 0.0,
        };
    }
    score
}

fn weighted_term_score(text: &str, terms: &[String], weight: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let normalized = text.to_ascii_lowercase();
    terms
        .iter()
        .filter(|term| normalized.contains(term.as_str()))
        .count() as f32
        * weight
}

fn validate_safe_text_field(value: &str, field_name: &str, max_chars: usize) -> KernelResult<()> {
    validate_non_empty(value, field_name)?;
    reject_secret_material(value, field_name)?;
    if value.trim() != value {
        return Err(KernelError::validation(format!(
            "{field_name} must not contain leading or trailing whitespace"
        )));
    }
    if value.chars().count() > max_chars {
        return Err(KernelError::validation(format!(
            "{field_name} must be at most {max_chars} characters"
        )));
    }
    Ok(())
}

fn validate_trust_level(value: i16) -> KernelResult<()> {
    if !(0..=5).contains(&value) {
        return Err(KernelError::validation(
            "trustLevel must be between 0 and 5",
        ));
    }
    Ok(())
}

fn validate_optional_agent_id(value: Option<&str>) -> KernelResult<()> {
    if let Some(value) = value {
        validate_agent_id(value)?;
    }
    Ok(())
}

fn validate_knowledge_binding_scope(
    scope_kind: AgentKnowledgeBindingScopeKind,
    scope_ref: &str,
    agent_id: Option<&str>,
    deployment_id: Option<&str>,
) -> KernelResult<()> {
    match scope_kind {
        AgentKnowledgeBindingScopeKind::Agent => {
            let Some(agent_id) = agent_id else {
                return Err(KernelError::validation(
                    "agentId is required for agent knowledge binding scope",
                ));
            };
            if scope_ref != agent_id {
                return Err(KernelError::validation(
                    "scopeRef must match agentId for agent knowledge binding scope",
                ));
            }
        }
        AgentKnowledgeBindingScopeKind::Deployment => {
            if agent_id.is_none() {
                return Err(KernelError::validation(
                    "agentId is required for deployment knowledge binding scope",
                ));
            }
            let Some(deployment_id) = deployment_id else {
                return Err(KernelError::validation(
                    "deploymentId is required for deployment knowledge binding scope",
                ));
            };
            if scope_ref != deployment_id {
                return Err(KernelError::validation(
                    "scopeRef must match deploymentId for deployment knowledge binding scope",
                ));
            }
        }
        AgentKnowledgeBindingScopeKind::User
        | AgentKnowledgeBindingScopeKind::Session
        | AgentKnowledgeBindingScopeKind::Organization
        | AgentKnowledgeBindingScopeKind::Tenant => {}
    }
    Ok(())
}

fn validate_memory_binding_scope(
    scope_kind: AgentMemoryBindingScopeKind,
    scope_ref: &str,
    agent_id: Option<&str>,
    deployment_id: Option<&str>,
) -> KernelResult<()> {
    match scope_kind {
        AgentMemoryBindingScopeKind::Agent => {
            let Some(agent_id) = agent_id else {
                return Err(KernelError::validation(
                    "agentId is required for agent memory binding scope",
                ));
            };
            if scope_ref != agent_id {
                return Err(KernelError::validation(
                    "scopeRef must match agentId for agent memory binding scope",
                ));
            }
        }
        AgentMemoryBindingScopeKind::Deployment => {
            if agent_id.is_none() {
                return Err(KernelError::validation(
                    "agentId is required for deployment memory binding scope",
                ));
            }
            let Some(deployment_id) = deployment_id else {
                return Err(KernelError::validation(
                    "deploymentId is required for deployment memory binding scope",
                ));
            };
            if scope_ref != deployment_id {
                return Err(KernelError::validation(
                    "scopeRef must match deploymentId for deployment memory binding scope",
                ));
            }
        }
        AgentMemoryBindingScopeKind::User
        | AgentMemoryBindingScopeKind::Session
        | AgentMemoryBindingScopeKind::Organization
        | AgentMemoryBindingScopeKind::Tenant => {}
    }
    Ok(())
}

fn validate_agent_id(value: &str) -> KernelResult<()> {
    validate_standard_id(value, "agentId", Some("agent."))
}

fn validate_optional_plain_ref(value: Option<&str>, field_name: &str) -> KernelResult<()> {
    if let Some(value) = value {
        validate_non_empty(value, field_name)?;
        reject_secret_material(value, field_name)?;
        if value.trim() != value {
            return Err(KernelError::validation(format!(
                "{field_name} must not contain leading or trailing whitespace"
            )));
        }
        if value.chars().count() > 128 {
            return Err(KernelError::validation(format!(
                "{field_name} must be at most 128 characters"
            )));
        }
    }
    Ok(())
}

fn validate_score(value: f32, field_name: &str) -> KernelResult<()> {
    if !(0.0..=1.0).contains(&value) || !value.is_finite() {
        return Err(KernelError::validation(format!(
            "{field_name} must be between 0 and 1"
        )));
    }
    Ok(())
}

fn ensure_marketplace_update_allowed(
    is_deleted: bool,
    actual_version: u64,
    expected_version: Option<u64>,
    entity_name: &str,
) -> KernelResult<()> {
    if is_deleted {
        return Err(KernelError::validation(format!(
            "deleted {entity_name} cannot be updated"
        )));
    }
    ensure_expected_version(actual_version, expected_version, entity_name)
}

fn ensure_marketplace_delete_allowed(
    is_deleted: bool,
    actual_version: u64,
    expected_version: Option<u64>,
    entity_name: &str,
) -> KernelResult<()> {
    if is_deleted {
        return Err(KernelError::validation(format!(
            "{entity_name} already deleted"
        )));
    }
    ensure_expected_version(actual_version, expected_version, entity_name)
}

fn ensure_marketplace_restore_allowed(
    is_deleted: bool,
    actual_version: u64,
    expected_version: Option<u64>,
    entity_name: &str,
) -> KernelResult<()> {
    if !is_deleted {
        return Err(KernelError::validation(format!(
            "{entity_name} is not deleted"
        )));
    }
    ensure_expected_version(actual_version, expected_version, entity_name)
}

fn ensure_expected_version(
    actual_version: u64,
    expected_version: Option<u64>,
    entity_name: &str,
) -> KernelResult<()> {
    let expected_version = expected_version.ok_or_else(|| {
        KernelError::validation(format!("{entity_name} mutation requires expectedVersion"))
    })?;
    if actual_version != expected_version {
        return Err(KernelError::conflict(format!(
            "{entity_name} version mismatch: expected={expected_version}, actual={actual_version}"
        )));
    }
    Ok(())
}
