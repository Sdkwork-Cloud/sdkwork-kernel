use crate::{KernelResult, ProviderHealth};

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
    pub source: String,
    pub content: String,
    pub trust_level: TrustLevel,
    pub provenance: Option<String>,
    pub redaction_classification: RedactionClassification,
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
            source: source.into(),
            content: content.into(),
            trust_level,
            provenance: None,
            redaction_classification,
        }
    }

    pub fn with_provenance(mut self, provenance: impl Into<String>) -> Self {
        self.provenance = Some(provenance.into());
        self
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
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
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
            trust_level,
            redaction_classification,
        }
    }

    pub fn requires_redaction(&self) -> bool {
        self.redaction_classification.requires_redaction()
    }
}

pub trait MemoryProvider {
    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>>;

    fn write(&mut self, record: MemoryRecord) -> KernelResult<()>;

    fn delete(&mut self, memory_record_id: &str) -> KernelResult<()>;

    fn export(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>>;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
