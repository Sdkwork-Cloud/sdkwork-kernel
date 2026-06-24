use sdkwork_agent_kernel::AgentManifest;
use sdkwork_code_kernel::CodeTaskIntent;

pub const DEFAULT_AGENT_MANAGEMENT_POLICY_CATEGORY: &str = "agent.business.manage";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBusinessStatus {
    Draft,
    Active,
    Disabled,
    Archived,
    Deleted,
}

impl AgentBusinessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Archived => "archived",
            Self::Deleted => "deleted",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "active" => Some(Self::Active),
            "disabled" => Some(Self::Disabled),
            "archived" => Some(Self::Archived),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }

    pub fn as_db_code(&self) -> i16 {
        match self {
            Self::Draft => 0,
            Self::Active => 1,
            Self::Disabled => 2,
            Self::Archived => 3,
            Self::Deleted => 4,
        }
    }

    pub fn from_db_code(value: i16) -> Option<Self> {
        match value {
            0 => Some(Self::Draft),
            1 => Some(Self::Active),
            2 => Some(Self::Disabled),
            3 => Some(Self::Archived),
            4 => Some(Self::Deleted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentVisibility {
    Private,
    Organization,
    Tenant,
    Public,
}

impl AgentVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Organization => "organization",
            Self::Tenant => "tenant",
            Self::Public => "public",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "private" => Some(Self::Private),
            "organization" => Some(Self::Organization),
            "tenant" => Some(Self::Tenant),
            "public" => Some(Self::Public),
            _ => None,
        }
    }

    pub fn as_db_code(&self) -> i16 {
        match self {
            Self::Private => 0,
            Self::Organization => 1,
            Self::Tenant => 2,
            Self::Public => 3,
        }
    }

    pub fn from_db_code(value: i16) -> Option<Self> {
        match value {
            0 => Some(Self::Private),
            1 => Some(Self::Organization),
            2 => Some(Self::Tenant),
            3 => Some(Self::Public),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAuditAction {
    Create,
    Update,
    Delete,
    Restore,
    ChangeStatus,
    RuntimeExecutionCompleted,
    ProviderBindingChanged,
    DeploymentCreated,
    SkillPackageCreated,
    SkillPackageUpdated,
    SkillPackageDeleted,
    SkillPackageRestored,
    McpServerCreated,
    McpServerUpdated,
    McpServerDeleted,
    McpServerRestored,
    MemoryStoreCreated,
    MemoryStoreUpdated,
    MemoryStoreDeleted,
    MemoryStoreRestored,
    MemoryProfileCreated,
    MemoryBindingCreated,
    MemoryNamespaceCreated,
    MemoryRecordCreated,
    MemoryRecordDeleted,
    MemoryRecordRestored,
    MemorySourceCreated,
    MemoryRelationCreated,
    MemoryRetrievalIndexUpserted,
    MemoryProfileUpdated,
    MemoryProfileDeleted,
    MemoryProfileRestored,
    MemoryBindingUpdated,
    MemoryBindingDeleted,
    MemoryBindingRestored,
    MemoryNamespaceUpdated,
    MemoryNamespaceDeleted,
    MemoryNamespaceRestored,
    MemorySourceDeleted,
    MemorySourceRestored,
    MemoryRelationDeleted,
    MemoryRelationRestored,
    KnowledgeBaseCreated,
    KnowledgeBaseUpdated,
    KnowledgeBaseDeleted,
    KnowledgeBaseRestored,
    KnowledgeSourceCreated,
    KnowledgeSourceUpdated,
    KnowledgeSourceDeleted,
    KnowledgeSourceRestored,
    KnowledgeDocumentCreated,
    KnowledgeDocumentUpdated,
    KnowledgeDocumentDeleted,
    KnowledgeDocumentRestored,
    KnowledgeChunkCreated,
    KnowledgeIndexUpserted,
    KnowledgeBindingCreated,
    KnowledgeSyncJobCreated,
    KnowledgeSyncJobStarted,
    KnowledgeSyncJobCompleted,
    KnowledgeSyncJobFailed,
    KnowledgeSyncJobCancelled,
}

impl AgentAuditAction {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Create => "agent.business.created",
            Self::Update => "agent.business.updated",
            Self::Delete => "agent.business.deleted",
            Self::Restore => "agent.business.restored",
            Self::ChangeStatus => "agent.business.status_changed",
            Self::RuntimeExecutionCompleted => "agent.business.runtime.executed",
            Self::ProviderBindingChanged => "agent.business.provider_binding_changed",
            Self::DeploymentCreated => "agent.business.deployment_created",
            Self::SkillPackageCreated => "agent.business.skill.created",
            Self::SkillPackageUpdated => "agent.business.skill.updated",
            Self::SkillPackageDeleted => "agent.business.skill.deleted",
            Self::SkillPackageRestored => "agent.business.skill.restored",
            Self::McpServerCreated => "agent.business.mcp.created",
            Self::McpServerUpdated => "agent.business.mcp.updated",
            Self::McpServerDeleted => "agent.business.mcp.deleted",
            Self::McpServerRestored => "agent.business.mcp.restored",
            Self::MemoryStoreCreated => "agent.business.memory.store.created",
            Self::MemoryStoreUpdated => "agent.business.memory.store.updated",
            Self::MemoryStoreDeleted => "agent.business.memory.store.deleted",
            Self::MemoryStoreRestored => "agent.business.memory.store.restored",
            Self::MemoryProfileCreated => "agent.business.memory.profile.created",
            Self::MemoryBindingCreated => "agent.business.memory.binding.created",
            Self::MemoryNamespaceCreated => "agent.business.memory.namespace.created",
            Self::MemoryRecordCreated => "agent.business.memory.record.created",
            Self::MemoryRecordDeleted => "agent.business.memory.record.deleted",
            Self::MemoryRecordRestored => "agent.business.memory.record.restored",
            Self::MemorySourceCreated => "agent.business.memory.source.created",
            Self::MemoryRelationCreated => "agent.business.memory.relation.created",
            Self::MemoryRetrievalIndexUpserted => "agent.business.memory.retrieval_index.upserted",
            Self::MemoryProfileUpdated => "agent.business.memory.profile.updated",
            Self::MemoryProfileDeleted => "agent.business.memory.profile.deleted",
            Self::MemoryProfileRestored => "agent.business.memory.profile.restored",
            Self::MemoryBindingUpdated => "agent.business.memory.binding.updated",
            Self::MemoryBindingDeleted => "agent.business.memory.binding.deleted",
            Self::MemoryBindingRestored => "agent.business.memory.binding.restored",
            Self::MemoryNamespaceUpdated => "agent.business.memory.namespace.updated",
            Self::MemoryNamespaceDeleted => "agent.business.memory.namespace.deleted",
            Self::MemoryNamespaceRestored => "agent.business.memory.namespace.restored",
            Self::MemorySourceDeleted => "agent.business.memory.source.deleted",
            Self::MemorySourceRestored => "agent.business.memory.source.restored",
            Self::MemoryRelationDeleted => "agent.business.memory.relation.deleted",
            Self::MemoryRelationRestored => "agent.business.memory.relation.restored",
            Self::KnowledgeBaseCreated => "agent.business.knowledge.base.created",
            Self::KnowledgeBaseUpdated => "agent.business.knowledge.base.updated",
            Self::KnowledgeBaseDeleted => "agent.business.knowledge.base.deleted",
            Self::KnowledgeBaseRestored => "agent.business.knowledge.base.restored",
            Self::KnowledgeSourceCreated => "agent.business.knowledge.source.created",
            Self::KnowledgeSourceUpdated => "agent.business.knowledge.source.updated",
            Self::KnowledgeSourceDeleted => "agent.business.knowledge.source.deleted",
            Self::KnowledgeSourceRestored => "agent.business.knowledge.source.restored",
            Self::KnowledgeDocumentCreated => "agent.business.knowledge.document.created",
            Self::KnowledgeDocumentUpdated => "agent.business.knowledge.document.updated",
            Self::KnowledgeDocumentDeleted => "agent.business.knowledge.document.deleted",
            Self::KnowledgeDocumentRestored => "agent.business.knowledge.document.restored",
            Self::KnowledgeChunkCreated => "agent.business.knowledge.chunk.created",
            Self::KnowledgeIndexUpserted => "agent.business.knowledge.index.upserted",
            Self::KnowledgeBindingCreated => "agent.business.knowledge.binding.created",
            Self::KnowledgeSyncJobCreated => "agent.business.knowledge.sync_job.created",
            Self::KnowledgeSyncJobStarted => "agent.business.knowledge.sync_job.started",
            Self::KnowledgeSyncJobCompleted => "agent.business.knowledge.sync_job.completed",
            Self::KnowledgeSyncJobFailed => "agent.business.knowledge.sync_job.failed",
            Self::KnowledgeSyncJobCancelled => "agent.business.knowledge.sync_job.cancelled",
        }
    }

    pub fn action_code(&self) -> &'static str {
        match self {
            Self::Create => "created",
            Self::Update => "updated",
            Self::Delete => "deleted",
            Self::Restore => "restored",
            Self::ChangeStatus => "status_changed",
            Self::RuntimeExecutionCompleted => "runtime_executed",
            Self::ProviderBindingChanged => "provider_binding_changed",
            Self::DeploymentCreated => "deployment_created",
            Self::SkillPackageCreated => "skill_created",
            Self::SkillPackageUpdated => "skill_updated",
            Self::SkillPackageDeleted => "skill_deleted",
            Self::SkillPackageRestored => "skill_restored",
            Self::McpServerCreated => "mcp_created",
            Self::McpServerUpdated => "mcp_updated",
            Self::McpServerDeleted => "mcp_deleted",
            Self::McpServerRestored => "mcp_restored",
            Self::MemoryStoreCreated => "memory_store_created",
            Self::MemoryStoreUpdated => "memory_store_updated",
            Self::MemoryStoreDeleted => "memory_store_deleted",
            Self::MemoryStoreRestored => "memory_store_restored",
            Self::MemoryProfileCreated => "memory_profile_created",
            Self::MemoryBindingCreated => "memory_binding_created",
            Self::MemoryNamespaceCreated => "memory_namespace_created",
            Self::MemoryRecordCreated => "memory_record_created",
            Self::MemoryRecordDeleted => "memory_record_deleted",
            Self::MemoryRecordRestored => "memory_record_restored",
            Self::MemorySourceCreated => "memory_source_created",
            Self::MemoryRelationCreated => "memory_relation_created",
            Self::MemoryRetrievalIndexUpserted => "memory_retrieval_index_upserted",
            Self::MemoryProfileUpdated => "memory_profile_updated",
            Self::MemoryProfileDeleted => "memory_profile_deleted",
            Self::MemoryProfileRestored => "memory_profile_restored",
            Self::MemoryBindingUpdated => "memory_binding_updated",
            Self::MemoryBindingDeleted => "memory_binding_deleted",
            Self::MemoryBindingRestored => "memory_binding_restored",
            Self::MemoryNamespaceUpdated => "memory_namespace_updated",
            Self::MemoryNamespaceDeleted => "memory_namespace_deleted",
            Self::MemoryNamespaceRestored => "memory_namespace_restored",
            Self::MemorySourceDeleted => "memory_source_deleted",
            Self::MemorySourceRestored => "memory_source_restored",
            Self::MemoryRelationDeleted => "memory_relation_deleted",
            Self::MemoryRelationRestored => "memory_relation_restored",
            Self::KnowledgeBaseCreated => "knowledge_base_created",
            Self::KnowledgeBaseUpdated => "knowledge_base_updated",
            Self::KnowledgeBaseDeleted => "knowledge_base_deleted",
            Self::KnowledgeBaseRestored => "knowledge_base_restored",
            Self::KnowledgeSourceCreated => "knowledge_source_created",
            Self::KnowledgeSourceUpdated => "knowledge_source_updated",
            Self::KnowledgeSourceDeleted => "knowledge_source_deleted",
            Self::KnowledgeSourceRestored => "knowledge_source_restored",
            Self::KnowledgeDocumentCreated => "knowledge_document_created",
            Self::KnowledgeDocumentUpdated => "knowledge_document_updated",
            Self::KnowledgeDocumentDeleted => "knowledge_document_deleted",
            Self::KnowledgeDocumentRestored => "knowledge_document_restored",
            Self::KnowledgeChunkCreated => "knowledge_chunk_created",
            Self::KnowledgeIndexUpserted => "knowledge_index_upserted",
            Self::KnowledgeBindingCreated => "knowledge_binding_created",
            Self::KnowledgeSyncJobCreated => "knowledge_sync_job_created",
            Self::KnowledgeSyncJobStarted => "knowledge_sync_job_started",
            Self::KnowledgeSyncJobCompleted => "knowledge_sync_job_completed",
            Self::KnowledgeSyncJobFailed => "knowledge_sync_job_failed",
            Self::KnowledgeSyncJobCancelled => "knowledge_sync_job_cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRuntimeExecutionOperation {
    PreviewResponse,
    PromptOptimization,
}

impl AgentRuntimeExecutionOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreviewResponse => "preview_response",
            Self::PromptOptimization => "prompt_optimization",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRuntimeExecutionStatus {
    Completed,
}

impl AgentRuntimeExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeExecutionRecord {
    pub tenant_id: u64,
    pub agent_id: String,
    pub execution_id: String,
    pub operation: AgentRuntimeExecutionOperation,
    pub status: AgentRuntimeExecutionStatus,
    pub input_payload_json: String,
    pub output_payload_json: String,
    pub requested_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentImplementationKind {
    ManifestOnly,
    TypedLocalProvider,
    ProcessAdapter,
    ProtocolAdapter,
}

impl AgentImplementationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ManifestOnly => "manifest-only",
            Self::TypedLocalProvider => "typed-local-provider",
            Self::ProcessAdapter => "process-adapter",
            Self::ProtocolAdapter => "protocol-adapter",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "manifest-only" => Some(Self::ManifestOnly),
            "typed-local-provider" => Some(Self::TypedLocalProvider),
            "process-adapter" => Some(Self::ProcessAdapter),
            "protocol-adapter" => Some(Self::ProtocolAdapter),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentImplementationType {
    SdkworkNative,
    RigRust,
    OpenAiAgents,
    LangChain,
    LangGraph,
    CrewAi,
    AutoGen,
    SemanticKernel,
    Custom,
}

impl AgentImplementationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SdkworkNative => "sdkwork-native",
            Self::RigRust => "rig-rust",
            Self::OpenAiAgents => "openai-agents",
            Self::LangChain => "langchain",
            Self::LangGraph => "langgraph",
            Self::CrewAi => "crewai",
            Self::AutoGen => "autogen",
            Self::SemanticKernel => "semantic-kernel",
            Self::Custom => "custom",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "sdkwork-native" => Some(Self::SdkworkNative),
            "rig-rust" => Some(Self::RigRust),
            "openai-agents" => Some(Self::OpenAiAgents),
            "langchain" => Some(Self::LangChain),
            "langgraph" => Some(Self::LangGraph),
            "crewai" => Some(Self::CrewAi),
            "autogen" => Some(Self::AutoGen),
            "semantic-kernel" => Some(Self::SemanticKernel),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

impl Default for AgentImplementationType {
    fn default() -> Self {
        Self::SdkworkNative
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBusinessRecord {
    pub id: u64,
    pub agent_id: String,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub manifest: AgentManifest,
    pub default_code_task_intent: Option<CodeTaskIntent>,
    pub implementation_provider_id: Option<String>,
    pub implementation_kind: Option<AgentImplementationKind>,
    pub implementation_type: AgentImplementationType,
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub tags: Vec<String>,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub agent_id: String,
    pub binding_id: String,
    pub provider_id: String,
    pub implementation_kind: AgentImplementationKind,
    pub configuration_profile_id: String,
    pub capabilities: Vec<String>,
    pub active: bool,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentProviderBindingRecord {
    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentDeploymentStatus {
    Created,
    Active,
    Failed,
    Archived,
}

impl AgentDeploymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Active => "active",
            Self::Failed => "failed",
            Self::Archived => "archived",
        }
    }

    pub fn as_db_code(&self) -> i16 {
        match self {
            Self::Created => 0,
            Self::Active => 1,
            Self::Failed => 2,
            Self::Archived => 3,
        }
    }

    pub fn from_db_code(value: i16) -> Option<Self> {
        match value {
            0 => Some(Self::Created),
            1 => Some(Self::Active),
            2 => Some(Self::Failed),
            3 => Some(Self::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeploymentRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub agent_id: String,
    pub deployment_id: String,
    pub binding_id: String,
    pub provider_id_snapshot: String,
    pub implementation_kind_snapshot: AgentImplementationKind,
    pub configuration_profile_id_snapshot: String,
    pub capabilities_snapshot: Vec<String>,
    pub status: AgentDeploymentStatus,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMcpTransportKind {
    Stdio,
    Http,
    Sse,
    WebSocket,
}

impl AgentMcpTransportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
            Self::Sse => "sse",
            Self::WebSocket => "websocket",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "stdio" => Some(Self::Stdio),
            "http" => Some(Self::Http),
            "sse" => Some(Self::Sse),
            "websocket" => Some(Self::WebSocket),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMcpAuthKind {
    None,
    OAuth2,
    ApiKeyRef,
    HostSecretRef,
}

impl AgentMcpAuthKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::OAuth2 => "oauth2",
            Self::ApiKeyRef => "api-key-ref",
            Self::HostSecretRef => "host-secret-ref",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "oauth2" => Some(Self::OAuth2),
            "api-key-ref" => Some(Self::ApiKeyRef),
            "host-secret-ref" => Some(Self::HostSecretRef),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemoryStoreKind {
    LocalPostgres,
    ExternalProvider,
    VectorStore,
    GraphStore,
    HybridStore,
    FileStore,
}

impl AgentMemoryStoreKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalPostgres => "local-postgres",
            Self::ExternalProvider => "external-provider",
            Self::VectorStore => "vector-store",
            Self::GraphStore => "graph-store",
            Self::HybridStore => "hybrid-store",
            Self::FileStore => "file-store",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "local-postgres" => Some(Self::LocalPostgres),
            "external-provider" => Some(Self::ExternalProvider),
            "vector-store" => Some(Self::VectorStore),
            "graph-store" => Some(Self::GraphStore),
            "hybrid-store" => Some(Self::HybridStore),
            "file-store" => Some(Self::FileStore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemoryIndexKind {
    Keyword,
    Sparse,
    Vector,
    Graph,
    Wiki,
    Rule,
    Hybrid,
}

impl AgentMemoryIndexKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Sparse => "sparse",
            Self::Vector => "vector",
            Self::Graph => "graph",
            Self::Wiki => "wiki",
            Self::Rule => "rule",
            Self::Hybrid => "hybrid",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "keyword" => Some(Self::Keyword),
            "sparse" => Some(Self::Sparse),
            "vector" => Some(Self::Vector),
            "graph" => Some(Self::Graph),
            "wiki" => Some(Self::Wiki),
            "rule" => Some(Self::Rule),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemoryBindingScopeKind {
    Agent,
    Deployment,
    User,
    Session,
    Organization,
    Tenant,
}

impl AgentMemoryBindingScopeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Deployment => "deployment",
            Self::User => "user",
            Self::Session => "session",
            Self::Organization => "organization",
            Self::Tenant => "tenant",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "agent" => Some(Self::Agent),
            "deployment" => Some(Self::Deployment),
            "user" => Some(Self::User),
            "session" => Some(Self::Session),
            "organization" => Some(Self::Organization),
            "tenant" => Some(Self::Tenant),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemoryNamespaceKind {
    Tenant,
    Organization,
    Agent,
    User,
    Session,
    Thread,
    Task,
}

impl AgentMemoryNamespaceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tenant => "tenant",
            Self::Organization => "organization",
            Self::Agent => "agent",
            Self::User => "user",
            Self::Session => "session",
            Self::Thread => "thread",
            Self::Task => "task",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "tenant" => Some(Self::Tenant),
            "organization" => Some(Self::Organization),
            "agent" => Some(Self::Agent),
            "user" => Some(Self::User),
            "session" => Some(Self::Session),
            "thread" => Some(Self::Thread),
            "task" => Some(Self::Task),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemoryRecordKind {
    Working,
    Episodic,
    Semantic,
    Procedural,
    Preference,
    Summary,
    Task,
    Correction,
    System,
}

impl AgentMemoryRecordKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
            Self::Preference => "preference",
            Self::Summary => "summary",
            Self::Task => "task",
            Self::Correction => "correction",
            Self::System => "system",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "working" => Some(Self::Working),
            "episodic" => Some(Self::Episodic),
            "semantic" => Some(Self::Semantic),
            "procedural" => Some(Self::Procedural),
            "preference" => Some(Self::Preference),
            "summary" => Some(Self::Summary),
            "task" => Some(Self::Task),
            "correction" => Some(Self::Correction),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemorySourceKind {
    ConversationMessage,
    ToolResult,
    Document,
    KnowledgeRef,
    HumanFeedback,
    SystemRule,
    BusinessEvent,
}

impl AgentMemorySourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConversationMessage => "conversation-message",
            Self::ToolResult => "tool-result",
            Self::Document => "document",
            Self::KnowledgeRef => "knowledge-ref",
            Self::HumanFeedback => "human-feedback",
            Self::SystemRule => "system-rule",
            Self::BusinessEvent => "business-event",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "conversation-message" => Some(Self::ConversationMessage),
            "tool-result" => Some(Self::ToolResult),
            "document" => Some(Self::Document),
            "knowledge-ref" => Some(Self::KnowledgeRef),
            "human-feedback" => Some(Self::HumanFeedback),
            "system-rule" => Some(Self::SystemRule),
            "business-event" => Some(Self::BusinessEvent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMemoryRelationKind {
    Supports,
    Contradicts,
    Supersedes,
    Duplicates,
    DependsOn,
    PartOf,
    AboutEntity,
}

impl AgentMemoryRelationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Supersedes => "supersedes",
            Self::Duplicates => "duplicates",
            Self::DependsOn => "depends-on",
            Self::PartOf => "part-of",
            Self::AboutEntity => "about-entity",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "supports" => Some(Self::Supports),
            "contradicts" => Some(Self::Contradicts),
            "supersedes" => Some(Self::Supersedes),
            "duplicates" => Some(Self::Duplicates),
            "depends-on" => Some(Self::DependsOn),
            "part-of" => Some(Self::PartOf),
            "about-entity" => Some(Self::AboutEntity),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeBaseKind {
    Wiki,
    DocumentRepository,
    Database,
    ApiReference,
    Graph,
    Hybrid,
    ExternalProvider,
    FileStore,
}

impl AgentKnowledgeBaseKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wiki => "wiki",
            Self::DocumentRepository => "document-repository",
            Self::Database => "database",
            Self::ApiReference => "api-reference",
            Self::Graph => "graph",
            Self::Hybrid => "hybrid",
            Self::ExternalProvider => "external-provider",
            Self::FileStore => "file-store",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "wiki" => Some(Self::Wiki),
            "document-repository" => Some(Self::DocumentRepository),
            "database" => Some(Self::Database),
            "api-reference" => Some(Self::ApiReference),
            "graph" => Some(Self::Graph),
            "hybrid" => Some(Self::Hybrid),
            "external-provider" => Some(Self::ExternalProvider),
            "file-store" => Some(Self::FileStore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeIndexKind {
    Exact,
    Keyword,
    FullText,
    Structured,
    Graph,
    Wiki,
    Rule,
    Vector,
    Hybrid,
    LlmRerank,
    External,
}

impl AgentKnowledgeIndexKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Keyword => "keyword",
            Self::FullText => "full_text",
            Self::Structured => "structured",
            Self::Graph => "graph",
            Self::Wiki => "wiki",
            Self::Rule => "rule",
            Self::Vector => "vector",
            Self::Hybrid => "hybrid",
            Self::LlmRerank => "llm_rerank",
            Self::External => "external",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "exact" => Some(Self::Exact),
            "keyword" => Some(Self::Keyword),
            "full_text" => Some(Self::FullText),
            "structured" => Some(Self::Structured),
            "graph" => Some(Self::Graph),
            "wiki" => Some(Self::Wiki),
            "rule" => Some(Self::Rule),
            "vector" => Some(Self::Vector),
            "hybrid" => Some(Self::Hybrid),
            "llm_rerank" => Some(Self::LlmRerank),
            "external" => Some(Self::External),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeSourceKind {
    Upload,
    Wiki,
    Web,
    Database,
    Api,
    Filesystem,
    Manual,
    ExternalProvider,
}

impl AgentKnowledgeSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Wiki => "wiki",
            Self::Web => "web",
            Self::Database => "database",
            Self::Api => "api",
            Self::Filesystem => "filesystem",
            Self::Manual => "manual",
            Self::ExternalProvider => "external-provider",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "upload" => Some(Self::Upload),
            "wiki" => Some(Self::Wiki),
            "web" => Some(Self::Web),
            "database" => Some(Self::Database),
            "api" => Some(Self::Api),
            "filesystem" => Some(Self::Filesystem),
            "manual" => Some(Self::Manual),
            "external-provider" => Some(Self::ExternalProvider),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeDocumentKind {
    WikiPage,
    WikiSection,
    Article,
    Faq,
    ApiReference,
    Spec,
    Runbook,
    Policy,
    ExternalReference,
    Other,
}

impl AgentKnowledgeDocumentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WikiPage => "wiki-page",
            Self::WikiSection => "wiki-section",
            Self::Article => "article",
            Self::Faq => "faq",
            Self::ApiReference => "api-reference",
            Self::Spec => "spec",
            Self::Runbook => "runbook",
            Self::Policy => "policy",
            Self::ExternalReference => "external-reference",
            Self::Other => "other",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "wiki-page" => Some(Self::WikiPage),
            "wiki-section" => Some(Self::WikiSection),
            "article" => Some(Self::Article),
            "faq" => Some(Self::Faq),
            "api-reference" => Some(Self::ApiReference),
            "spec" => Some(Self::Spec),
            "runbook" => Some(Self::Runbook),
            "policy" => Some(Self::Policy),
            "external-reference" => Some(Self::ExternalReference),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeBindingScopeKind {
    Agent,
    Deployment,
    User,
    Session,
    Organization,
    Tenant,
}

impl AgentKnowledgeBindingScopeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Deployment => "deployment",
            Self::User => "user",
            Self::Session => "session",
            Self::Organization => "organization",
            Self::Tenant => "tenant",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "agent" => Some(Self::Agent),
            "deployment" => Some(Self::Deployment),
            "user" => Some(Self::User),
            "session" => Some(Self::Session),
            "organization" => Some(Self::Organization),
            "tenant" => Some(Self::Tenant),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeSyncJobKind {
    Import,
    Refresh,
    Reindex,
    Delete,
}

impl AgentKnowledgeSyncJobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Refresh => "refresh",
            Self::Reindex => "reindex",
            Self::Delete => "delete",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "import" => Some(Self::Import),
            "refresh" => Some(Self::Refresh),
            "reindex" => Some(Self::Reindex),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKnowledgeSyncJobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl AgentKnowledgeSyncJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn from_code(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentDomainParseError;

macro_rules! impl_domain_from_str {
    ($domain_type:ty) => {
        impl std::str::FromStr for $domain_type {
            type Err = AgentDomainParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                <$domain_type>::from_code(value).ok_or(AgentDomainParseError)
            }
        }
    };
}

macro_rules! impl_domain_from_str_compat {
    ($domain_type:ty) => {
        impl $domain_type {
            #[allow(clippy::should_implement_trait)]
            #[deprecated(note = "use std::str::FromStr or SDKWork domain code parsing")]
            pub fn from_str(value: &str) -> Option<Self> {
                Self::from_code(value)
            }
        }
    };
}

impl_domain_from_str!(AgentBusinessStatus);
impl_domain_from_str!(AgentVisibility);
impl_domain_from_str!(AgentImplementationKind);
impl_domain_from_str!(AgentMcpTransportKind);
impl_domain_from_str!(AgentMcpAuthKind);
impl_domain_from_str!(AgentMemoryStoreKind);
impl_domain_from_str!(AgentMemoryIndexKind);
impl_domain_from_str!(AgentMemoryBindingScopeKind);
impl_domain_from_str!(AgentMemoryNamespaceKind);
impl_domain_from_str!(AgentMemoryRecordKind);
impl_domain_from_str!(AgentMemorySourceKind);
impl_domain_from_str!(AgentMemoryRelationKind);
impl_domain_from_str!(AgentKnowledgeBaseKind);
impl_domain_from_str!(AgentKnowledgeIndexKind);
impl_domain_from_str!(AgentKnowledgeSourceKind);
impl_domain_from_str!(AgentKnowledgeDocumentKind);
impl_domain_from_str!(AgentKnowledgeBindingScopeKind);
impl_domain_from_str!(AgentKnowledgeSyncJobKind);
impl_domain_from_str!(AgentKnowledgeSyncJobStatus);

impl_domain_from_str_compat!(AgentBusinessStatus);
impl_domain_from_str_compat!(AgentVisibility);
impl_domain_from_str_compat!(AgentImplementationKind);
impl_domain_from_str_compat!(AgentMcpTransportKind);
impl_domain_from_str_compat!(AgentMcpAuthKind);
impl_domain_from_str_compat!(AgentMemoryStoreKind);
impl_domain_from_str_compat!(AgentMemoryIndexKind);
impl_domain_from_str_compat!(AgentMemoryBindingScopeKind);
impl_domain_from_str_compat!(AgentMemoryNamespaceKind);
impl_domain_from_str_compat!(AgentMemoryRecordKind);
impl_domain_from_str_compat!(AgentMemorySourceKind);
impl_domain_from_str_compat!(AgentMemoryRelationKind);
impl_domain_from_str_compat!(AgentKnowledgeBaseKind);
impl_domain_from_str_compat!(AgentKnowledgeIndexKind);
impl_domain_from_str_compat!(AgentKnowledgeSourceKind);
impl_domain_from_str_compat!(AgentKnowledgeDocumentKind);
impl_domain_from_str_compat!(AgentKnowledgeBindingScopeKind);
impl_domain_from_str_compat!(AgentKnowledgeSyncJobKind);
impl_domain_from_str_compat!(AgentKnowledgeSyncJobStatus);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMcpServerRecord {
    pub id: u64,
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
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryStoreRecord {
    pub id: u64,
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
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryProfileRecord {
    pub id: u64,
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
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryBindingRecord {
    pub id: u64,
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
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryNamespaceRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub user_ref: Option<String>,
    pub session_ref: Option<String>,
    pub thread_ref: Option<String>,
    pub namespace_kind: AgentMemoryNamespaceKind,
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRecord {
    pub id: u64,
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
    pub source_count: u32,
    pub effective_at: Option<String>,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub use_count: u64,
    pub status: AgentBusinessStatus,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub redacted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemorySourceRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub memory_source_id: String,
    pub memory_id: String,
    pub source_kind: AgentMemorySourceKind,
    pub source_ref: String,
    pub source_hash: String,
    pub evidence_json: String,
    pub captured_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRelationRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub memory_relation_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation_kind: AgentMemoryRelationKind,
    pub weight: f32,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryRetrievalIndexRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub memory_index_id: String,
    pub memory_id: String,
    pub index_kind: AgentMemoryIndexKind,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub indexed_at: String,
    pub status: AgentBusinessStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBaseRecord {
    pub id: u64,
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
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSourceRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub knowledge_source_id: String,
    pub knowledge_base_id: String,
    pub source_kind: AgentKnowledgeSourceKind,
    pub source_ref: String,
    pub source_hash: String,
    pub sync_policy_json: String,
    pub metadata_json: String,
    pub status: AgentBusinessStatus,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeDocumentRecord {
    pub id: u64,
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
    pub chunk_count: u32,
    pub status: AgentBusinessStatus,
    pub visibility: AgentVisibility,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeChunkRecord {
    pub id: u64,
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
    pub status: AgentBusinessStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeIndexRecord {
    pub id: u64,
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
    pub indexed_at: String,
    pub status: AgentBusinessStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentKnowledgeSearchResult {
    pub tenant_id: u64,
    pub knowledge_base_id: String,
    pub provider_id: String,
    pub knowledge_index_id: String,
    pub index_provider_id: String,
    pub retrieval_method: AgentKnowledgeIndexKind,
    pub knowledge_document_id: Option<String>,
    pub document_kind: Option<AgentKnowledgeDocumentKind>,
    pub knowledge_chunk_id: Option<String>,
    pub title: String,
    pub snippet: Option<String>,
    pub score: Option<f32>,
    pub source_ref: Option<String>,
    pub content_ref: Option<String>,
    pub external_ref: Option<String>,
    pub trust_level: i16,
    pub redaction_classification: String,
    pub metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBindingRecord {
    pub id: u64,
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
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobRecord {
    pub id: u64,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub sync_job_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub job_kind: AgentKnowledgeSyncJobKind,
    pub status: AgentKnowledgeSyncJobStatus,
    pub input_ref: String,
    pub input_json: String,
    pub output_json: Option<String>,
    pub error_json: Option<String>,
    pub requested_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentBusinessRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.into());
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentMcpServerRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentMemoryStoreRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentMemoryProfileRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentMemoryBindingRecord {
    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentMemoryNamespaceRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentMemoryRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentKnowledgeBaseRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentKnowledgeSourceRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}

impl AgentKnowledgeDocumentRecord {
    pub fn is_deleted(&self) -> bool {
        self.status == AgentBusinessStatus::Deleted || self.deleted_at.is_some()
    }

    pub fn mark_updated(&mut self, updated_at: impl Into<String>) {
        self.updated_at = updated_at.into();
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_deleted(&mut self, deleted_at: impl Into<String>) {
        let deleted_at = deleted_at.into();
        self.status = AgentBusinessStatus::Deleted;
        self.deleted_at = Some(deleted_at.clone());
        self.updated_at = deleted_at;
        self.version = self.version.saturating_add(1);
    }

    pub fn mark_restored(&mut self, restored_at: impl Into<String>) {
        self.status = AgentBusinessStatus::Active;
        self.deleted_at = None;
        self.updated_at = restored_at.into();
        self.version = self.version.saturating_add(1);
    }
}
