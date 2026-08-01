use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest};

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

// ============================================================================
// External Memory Vocabulary Mapping
// ============================================================================

/// Memory types of the sibling sdkwork-memory service. The kernel maps this
/// external vocabulary onto its tier hierarchy so integrations can align
/// records without duplicating models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalMemoryType {
    Working,
    Session,
    Semantic,
    Episodic,
    Procedural,
    Habit,
    Relationship,
    DomainKnowledge,
}

impl ExternalMemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Session => "session",
            Self::Semantic => "semantic",
            Self::Episodic => "episodic",
            Self::Procedural => "procedural",
            Self::Habit => "habit",
            Self::Relationship => "relationship",
            Self::DomainKnowledge => "domain_knowledge",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "working" => Some(Self::Working),
            "session" => Some(Self::Session),
            "semantic" => Some(Self::Semantic),
            "episodic" => Some(Self::Episodic),
            "procedural" => Some(Self::Procedural),
            "habit" => Some(Self::Habit),
            "relationship" => Some(Self::Relationship),
            "domain_knowledge" => Some(Self::DomainKnowledge),
            _ => None,
        }
    }

    /// Map the external memory type onto the kernel tier hierarchy.
    pub fn to_kernel_tier(&self) -> MemoryTier {
        match self {
            Self::Working => MemoryTier::Ephemeral,
            Self::Session => MemoryTier::ShortTerm,
            Self::Semantic | Self::Episodic | Self::Procedural => MemoryTier::LongTerm,
            Self::Relationship | Self::DomainKnowledge => MemoryTier::Permanent,
            Self::Habit => MemoryTier::Growing,
        }
    }
}

impl MemoryTier {
    /// External memory types that land on this tier.
    pub fn to_external_types(&self) -> Vec<ExternalMemoryType> {
        match self {
            Self::Ephemeral => vec![ExternalMemoryType::Working],
            Self::ShortTerm => vec![ExternalMemoryType::Session],
            Self::LongTerm => vec![
                ExternalMemoryType::Semantic,
                ExternalMemoryType::Episodic,
                ExternalMemoryType::Procedural,
            ],
            Self::Permanent => vec![
                ExternalMemoryType::Relationship,
                ExternalMemoryType::DomainKnowledge,
            ],
            Self::Growing => vec![ExternalMemoryType::Habit],
        }
    }
}

impl MemoryScope {
    /// External scope string used by the sibling memory service.
    pub fn to_external_scope(&self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::User => "user",
            Self::Tenant => "tenant",
            Self::Organization => "organization",
            Self::Agent => "agent",
            Self::Application => "application",
        }
    }

    pub fn from_external_scope(value: &str) -> Option<Self> {
        match value {
            "session" => Some(Self::Session),
            "user" => Some(Self::User),
            "tenant" => Some(Self::Tenant),
            "organization" => Some(Self::Organization),
            "agent" => Some(Self::Agent),
            "application" => Some(Self::Application),
            _ => None,
        }
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

    fn rank(&self, frames: &[ContextFrame]) -> KernelResult<Vec<ContextRanking>> {
        Ok(frames
            .iter()
            .enumerate()
            .map(|(i, f)| ContextRanking {
                frame_id: f.context_frame_id.clone(),
                relevance_score: 1.0 - (i as f64 * 0.01),
                reason: None,
            })
            .collect())
    }

    fn trim(
        &self,
        frames: Vec<ContextFrame>,
        max_tokens: usize,
    ) -> KernelResult<Vec<ContextFrame>> {
        let _ = max_tokens;
        Ok(frames)
    }

    fn explain(&self, frame: &ContextFrame) -> KernelResult<ContextExplanation> {
        Ok(ContextExplanation {
            frame_id: frame.context_frame_id.clone(),
            reason: "default context explanation".to_string(),
            source_factors: Vec::new(),
        })
    }

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextRanking {
    pub frame_id: String,
    pub relevance_score: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextExplanation {
    pub frame_id: String,
    pub reason: String,
    pub source_factors: Vec<String>,
}

// ============================================================================
// Memory Tier - persistence and evolution behavior of a memory record
// ============================================================================

/// Describes the persistence and evolution behavior of a memory record.
///
/// This is orthogonal to [`MemoryScope`], which defines ownership and
/// visibility. A record with `MemoryScope::User` and `MemoryTier::Permanent`
/// is a permanent user-level memory that never expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryTier {
    /// In-memory only, lost when the process exits
    Ephemeral,
    /// Session-scoped, auto-expiring after session ends
    #[default]
    ShortTerm,
    /// Persisted with an explicit retention policy
    LongTerm,
    /// Never deleted, survives across sessions and restarts
    Permanent,
    /// Accumulates and evolves over time through consolidation
    Growing,
}

impl MemoryTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::ShortTerm => "short_term",
            Self::LongTerm => "long_term",
            Self::Permanent => "permanent",
            Self::Growing => "growing",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ephemeral" => Some(Self::Ephemeral),
            "short_term" => Some(Self::ShortTerm),
            "long_term" => Some(Self::LongTerm),
            "permanent" => Some(Self::Permanent),
            "growing" => Some(Self::Growing),
            _ => None,
        }
    }

    pub fn is_persistent(&self) -> bool {
        matches!(self, Self::LongTerm | Self::Permanent | Self::Growing)
    }

    pub fn can_evolve(&self) -> bool {
        matches!(self, Self::Growing)
    }
}

// ============================================================================
// Memory Scope - ownership and visibility of a memory record
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Session,
    User,
    Tenant,
    Organization,
    Agent,
    Application,
}

impl MemoryScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::User => "user",
            Self::Tenant => "tenant",
            Self::Organization => "organization",
            Self::Agent => "agent",
            Self::Application => "application",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "session" => Some(Self::Session),
            "user" => Some(Self::User),
            "tenant" => Some(Self::Tenant),
            "organization" => Some(Self::Organization),
            "agent" => Some(Self::Agent),
            "application" => Some(Self::Application),
            _ => None,
        }
    }

    pub fn all() -> &'static [MemoryScope] {
        &[
            MemoryScope::Session,
            MemoryScope::User,
            MemoryScope::Tenant,
            MemoryScope::Organization,
            MemoryScope::Agent,
            MemoryScope::Application,
        ]
    }
}

// ============================================================================
// Memory Record - a single piece of memory with tier support
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub memory_record_id: String,
    pub scope: MemoryScope,
    pub tier: MemoryTier,
    pub owner_context: String,
    pub content: String,
    pub content_type: String,
    pub source: Option<String>,
    pub trust_level: TrustLevel,
    pub retention_policy: Option<String>,
    pub redaction_classification: RedactionClassification,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub expires_at: Option<String>,
    pub consolidation_count: u32,
    pub parent_record_id: Option<String>,
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
            tier: MemoryTier::default(),
            owner_context: owner_context.into(),
            content: content.into(),
            content_type: "text/plain".to_string(),
            source: None,
            trust_level,
            retention_policy: None,
            redaction_classification,
            created_at: None,
            updated_at: None,
            expires_at: None,
            consolidation_count: 0,
            parent_record_id: None,
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

    pub fn with_tier(mut self, tier: MemoryTier) -> Self {
        self.tier = tier;
        self
    }

    pub fn with_expires_at(mut self, expires_at: impl Into<String>) -> Self {
        self.expires_at = Some(expires_at.into());
        self
    }

    pub fn with_parent(mut self, parent_record_id: impl Into<String>) -> Self {
        self.parent_record_id = Some(parent_record_id.into());
        self
    }

    pub fn with_consolidation_count(mut self, count: u32) -> Self {
        self.consolidation_count = count;
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

    pub fn is_persistent(&self) -> bool {
        self.tier.is_persistent()
    }

    pub fn is_permanent(&self) -> bool {
        self.tier == MemoryTier::Permanent
    }

    pub fn is_growing(&self) -> bool {
        self.tier == MemoryTier::Growing
    }

    pub fn has_expired(&self, current_time: &str) -> bool {
        match &self.expires_at {
            Some(expires_at) => current_time >= expires_at.as_str(),
            None => false,
        }
    }

    /// Create a consolidated child record from this record.
    /// Used by growing memory to track evolution history.
    pub fn consolidate_into(
        &self,
        new_record_id: impl Into<String>,
        new_content: impl Into<String>,
    ) -> Self {
        Self {
            memory_record_id: new_record_id.into(),
            scope: self.scope,
            tier: self.tier,
            owner_context: self.owner_context.clone(),
            content: new_content.into(),
            content_type: self.content_type.clone(),
            source: self.source.clone(),
            trust_level: self.trust_level,
            retention_policy: self.retention_policy.clone(),
            redaction_classification: self.redaction_classification,
            created_at: None,
            updated_at: None,
            expires_at: None,
            consolidation_count: self.consolidation_count.saturating_add(1),
            parent_record_id: Some(self.memory_record_id.clone()),
            policy_decision_id: None,
            metadata: self.metadata.clone(),
        }
    }
}

// ============================================================================
// Memory Provider - the SPI trait with tier-aware operations
// ============================================================================

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
                "memory.consolidate".to_string(),
                "memory.evolve".to_string(),
            ],
        )
    }

    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>>;

    fn write(&mut self, record: MemoryRecord) -> KernelResult<()>;

    fn delete(&mut self, memory_record_id: &str) -> KernelResult<()>;

    fn export(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>>;

    /// Query records by both scope and tier.
    /// Default implementation filters query results by tier.
    fn query_by_tier(
        &self,
        scope: MemoryScope,
        owner_context: &str,
        tier: MemoryTier,
    ) -> KernelResult<Vec<MemoryRecord>> {
        Ok(self
            .query(scope, owner_context)?
            .into_iter()
            .filter(|record| record.tier == tier)
            .collect())
    }

    /// Consolidate growing memory records for a given scope and owner.
    ///
    /// This merges multiple growing records into a consolidated record,
    /// preserving the evolution history through `parent_record_id`.
    fn consolidate(
        &mut self,
        scope: MemoryScope,
        owner_context: &str,
    ) -> KernelResult<Vec<MemoryRecord>> {
        let growing_records = self.query_by_tier(scope, owner_context, MemoryTier::Growing)?;
        if growing_records.len() < 2 {
            return Ok(growing_records);
        }

        let consolidated_content = growing_records
            .iter()
            .map(|record| record.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let parent = &growing_records[0];
        let new_record = parent
            .consolidate_into(
                format!("{}.consolidated", parent.memory_record_id),
                consolidated_content,
            )
            .with_tier(MemoryTier::Growing);

        for record in &growing_records {
            self.delete(&record.memory_record_id)?;
        }
        self.write(new_record.clone())?;
        Ok(vec![new_record])
    }

    /// Evolve a growing memory record by appending new content.
    ///
    /// This creates a new version of the record with the additional content,
    /// preserving the parent link for history tracking. The caller must provide
    /// the scope and owner context so the provider can locate the record.
    fn evolve(
        &mut self,
        scope: MemoryScope,
        owner_context: &str,
        memory_record_id: &str,
        additional_content: String,
    ) -> KernelResult<MemoryRecord> {
        let record = self
            .query(scope, owner_context)?
            .into_iter()
            .find(|record| record.memory_record_id == memory_record_id)
            .ok_or_else(|| {
                KernelError::validation(format!(
                    "memory record not found for evolution: {memory_record_id}"
                ))
            })?;

        if !record.is_growing() {
            return Err(KernelError::validation(
                "only growing-tier memory records can be evolved",
            ));
        }

        let new_content = format!("{}\n\n{}", record.content, additional_content);
        let evolved = record.consolidate_into(
            format!(
                "{}.evolved.{}",
                record.memory_record_id,
                record.consolidation_count + 1
            ),
            new_content,
        );
        self.delete(memory_record_id)?;
        self.write(evolved.clone())?;
        Ok(evolved)
    }

    fn health(&self) -> ProviderHealth;
}
