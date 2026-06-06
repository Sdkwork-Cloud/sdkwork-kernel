use crate::{KernelResult, ProviderHealth, ProviderManifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    TrustedSystem,
    TrustedHost,
    UserSupplied,
    ToolOutput,
    RetrievedExternal,
    AgentMessage,
    UnknownUntrusted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionClassification {
    Public,
    Internal,
    TenantSensitive,
    PersonalData,
    Secret,
    Regulated,
}

impl RedactionClassification {
    pub fn requires_redaction(self) -> bool {
        !matches!(self, Self::Public | Self::Internal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFrame {
    pub context_frame_id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub source: String,
    pub content: String,
    pub content_type: String,
    pub trust_level: TrustLevel,
    pub provenance: Option<String>,
    pub redaction_classification: RedactionClassification,
    pub created_at: Option<String>,
    pub metadata: Vec<(String, String)>,
}

impl ContextFrame {
    pub fn new(
        context_frame_id: impl Into<String>,
        session_id: impl Into<String>,
        source: impl Into<String>,
        content: impl Into<String>,
        trust_level: TrustLevel,
        redaction_classification: RedactionClassification,
    ) -> Self {
        Self {
            context_frame_id: context_frame_id.into(),
            session_id: session_id.into(),
            task_id: None,
            source: source.into(),
            content: content.into(),
            content_type: "text/plain".to_string(),
            trust_level,
            provenance: None,
            redaction_classification,
            created_at: None,
            metadata: Vec::new(),
        }
    }

    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    pub fn with_provenance(mut self, provenance: impl Into<String>) -> Self {
        self.provenance = Some(provenance.into());
        self
    }

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn is_untrusted(&self) -> bool {
        matches!(
            self.trust_level,
            TrustLevel::UserSupplied
                | TrustLevel::ToolOutput
                | TrustLevel::RetrievedExternal
                | TrustLevel::AgentMessage
                | TrustLevel::UnknownUntrusted
        )
    }
}

pub trait ContextProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.context.unspecified",
            "context",
            "context-provider",
            "0.0.0",
            vec!["context.collect".to_string()],
        )
    }

    fn collect(&self, session_id: &str) -> KernelResult<Vec<ContextFrame>>;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Session,
    User,
    Tenant,
    Organization,
    Agent,
    Application,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub memory_record_id: String,
    pub scope: MemoryScope,
    pub owner_context: String,
    pub content: String,
    pub content_type: String,
    pub source: Option<String>,
    pub trust_level: TrustLevel,
    pub retention_policy: Option<String>,
    pub redaction_classification: RedactionClassification,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub policy_decision_id: Option<String>,
    pub metadata: Vec<(String, String)>,
}

impl MemoryRecord {
    pub fn new(
        memory_record_id: impl Into<String>,
        scope: MemoryScope,
        owner_context: impl Into<String>,
        content: impl Into<String>,
        trust_level: TrustLevel,
        redaction_classification: RedactionClassification,
    ) -> Self {
        Self {
            memory_record_id: memory_record_id.into(),
            scope,
            owner_context: owner_context.into(),
            content: content.into(),
            content_type: "text/plain".to_string(),
            source: None,
            trust_level,
            retention_policy: None,
            redaction_classification,
            created_at: None,
            updated_at: None,
            policy_decision_id: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_retention_policy(mut self, retention_policy: impl Into<String>) -> Self {
        self.retention_policy = Some(retention_policy.into());
        self
    }

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn updated_at(mut self, updated_at: impl Into<String>) -> Self {
        self.updated_at = Some(updated_at.into());
        self
    }

    pub fn with_policy_decision(mut self, policy_decision_id: impl Into<String>) -> Self {
        self.policy_decision_id = Some(policy_decision_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn requires_redaction(&self) -> bool {
        self.redaction_classification.requires_redaction()
    }
}

pub trait MemoryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.memory.unspecified",
            "memory",
            "memory-provider",
            "0.0.0",
            vec![
                "memory.query".to_string(),
                "memory.write".to_string(),
                "memory.delete".to_string(),
                "memory.export".to_string(),
            ],
        )
    }

    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>>;

    fn write(&mut self, record: MemoryRecord) -> KernelResult<()>;

    fn delete(&mut self, memory_record_id: &str) -> KernelResult<()>;

    fn export(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>>;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
