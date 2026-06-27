use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest};

// ============================================================================
// Agent Category - primary functional classification of an agent
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCategory {
    /// Conversational agents focused on dialogue
    Conversational,
    /// Code generation and software engineering agents
    Coding,
    /// Autonomous goal-seeking agents
    Autonomous,
    /// Research and information gathering agents
    Research,
    /// Creative content generation agents
    Creative,
    /// Operational and task execution agents
    Operational,
    /// Data analysis and reasoning agents
    Analytical,
    /// Multi-modal agents handling text, image, audio
    MultiModal,
    /// Hybrid agents combining multiple categories
    Hybrid,
}

impl AgentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Conversational => "conversational",
            Self::Coding => "coding",
            Self::Autonomous => "autonomous",
            Self::Research => "research",
            Self::Creative => "creative",
            Self::Operational => "operational",
            Self::Analytical => "analytical",
            Self::MultiModal => "multi_modal",
            Self::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "conversational" => Some(Self::Conversational),
            "coding" => Some(Self::Coding),
            "autonomous" => Some(Self::Autonomous),
            "research" => Some(Self::Research),
            "creative" => Some(Self::Creative),
            "operational" => Some(Self::Operational),
            "analytical" => Some(Self::Analytical),
            "multi_modal" => Some(Self::MultiModal),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }

    pub fn all() -> &'static [AgentCategory] {
        &[
            Self::Conversational,
            Self::Coding,
            Self::Autonomous,
            Self::Research,
            Self::Creative,
            Self::Operational,
            Self::Analytical,
            Self::MultiModal,
            Self::Hybrid,
        ]
    }
}

// ============================================================================
// Autonomy Level - degree of autonomous operation
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum AutonomyLevel {
    /// Requires human approval for every action
    Manual,
    /// Operates under supervision with checkpoints
    #[default]
    Supervised,
    /// Autonomous within approved boundaries
    SemiAutonomous,
    /// Fully autonomous, no human approval required
    FullyAutonomous,
}

impl AutonomyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Supervised => "supervised",
            Self::SemiAutonomous => "semi_autonomous",
            Self::FullyAutonomous => "fully_autonomous",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "supervised" => Some(Self::Supervised),
            "semi_autonomous" => Some(Self::SemiAutonomous),
            "fully_autonomous" => Some(Self::FullyAutonomous),
            _ => None,
        }
    }

    pub fn numeric_value(&self) -> u8 {
        match self {
            Self::Manual => 0,
            Self::Supervised => 1,
            Self::SemiAutonomous => 2,
            Self::FullyAutonomous => 3,
        }
    }
}

// ============================================================================
// Capability Level - proficiency level for a specific capability
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityLevel {
    Basic,
    Intermediate,
    Advanced,
    Expert,
}

impl CapabilityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Intermediate => "intermediate",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "basic" => Some(Self::Basic),
            "intermediate" => Some(Self::Intermediate),
            "advanced" => Some(Self::Advanced),
            "expert" => Some(Self::Expert),
            _ => None,
        }
    }

    pub fn numeric_value(&self) -> u8 {
        match self {
            Self::Basic => 0,
            Self::Intermediate => 1,
            Self::Advanced => 2,
            Self::Expert => 3,
        }
    }
}

// ============================================================================
// Capability Assessment - a capability and its assessed level
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAssessment {
    pub capability_id: String,
    pub level: CapabilityLevel,
    pub evidence: Option<String>,
}

impl CapabilityAssessment {
    pub fn new(capability_id: impl Into<String>, level: CapabilityLevel) -> Self {
        Self {
            capability_id: capability_id.into(),
            level,
            evidence: None,
        }
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }
}

// ============================================================================
// Agent Classification - full classification of an agent
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentClassification {
    pub agent_id: String,
    pub primary_category: AgentCategory,
    pub secondary_categories: Vec<AgentCategory>,
    pub autonomy_level: AutonomyLevel,
    pub capability_assessments: Vec<CapabilityAssessment>,
    pub tags: Vec<String>,
    pub classification_version: String,
    pub classified_at: Option<String>,
    pub metadata: Vec<(String, String)>,
}

impl AgentClassification {
    pub fn new(agent_id: impl Into<String>, primary_category: AgentCategory) -> Self {
        Self {
            agent_id: agent_id.into(),
            primary_category,
            secondary_categories: Vec::new(),
            autonomy_level: AutonomyLevel::default(),
            capability_assessments: Vec::new(),
            tags: Vec::new(),
            classification_version: "1.0.0".to_string(),
            classified_at: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_secondary_category(mut self, category: AgentCategory) -> Self {
        if !self.secondary_categories.contains(&category) && category != self.primary_category {
            self.secondary_categories.push(category);
        }
        self
    }

    pub fn with_autonomy_level(mut self, level: AutonomyLevel) -> Self {
        self.autonomy_level = level;
        self
    }

    pub fn with_capability(mut self, assessment: CapabilityAssessment) -> Self {
        self.capability_assessments.push(assessment);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
        self
    }

    pub fn classified_at(mut self, classified_at: impl Into<String>) -> Self {
        self.classified_at = Some(classified_at.into());
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

    pub fn all_categories(&self) -> Vec<AgentCategory> {
        let mut categories = vec![self.primary_category];
        for category in &self.secondary_categories {
            if !categories.contains(category) {
                categories.push(*category);
            }
        }
        categories
    }

    pub fn has_category(&self, category: AgentCategory) -> bool {
        self.primary_category == category || self.secondary_categories.contains(&category)
    }

    pub fn capability_level(&self, capability_id: &str) -> Option<CapabilityLevel> {
        self.capability_assessments
            .iter()
            .find(|assessment| assessment.capability_id == capability_id)
            .map(|assessment| assessment.level)
    }

    pub fn highest_capability_level(&self) -> Option<CapabilityLevel> {
        self.capability_assessments
            .iter()
            .map(|assessment| assessment.level)
            .max()
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.agent_id.trim().is_empty() {
            return Err(KernelError::validation("agent_id must not be empty"));
        }
        for assessment in &self.capability_assessments {
            if assessment.capability_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "capability_id in assessment must not be empty",
                ));
            }
        }
        for tag in &self.tags {
            if tag.trim().is_empty() {
                return Err(KernelError::validation(
                    "tags must not contain blank values",
                ));
            }
        }
        Ok(())
    }
}

// ============================================================================
// Classification Query - filter for querying agent classifications
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassificationQuery {
    pub category: Option<AgentCategory>,
    pub autonomy_level: Option<AutonomyLevel>,
    pub tag: Option<String>,
    pub min_capability_level: Option<CapabilityLevel>,
    pub capability_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl ClassificationQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn in_category(mut self, category: AgentCategory) -> Self {
        self.category = Some(category);
        self
    }

    pub fn with_autonomy(mut self, level: AutonomyLevel) -> Self {
        self.autonomy_level = Some(level);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn with_min_capability(mut self, level: CapabilityLevel) -> Self {
        self.min_capability_level = Some(level);
        self
    }

    pub fn with_capability_id(mut self, capability_id: impl Into<String>) -> Self {
        self.capability_id = Some(capability_id.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn matches(&self, classification: &AgentClassification) -> bool {
        if let Some(category) = self.category {
            if !classification.has_category(category) {
                return false;
            }
        }
        if let Some(autonomy) = self.autonomy_level {
            if classification.autonomy_level != autonomy {
                return false;
            }
        }
        if let Some(tag) = &self.tag {
            if !classification.tags.contains(tag) {
                return false;
            }
        }
        if let Some(capability_id) = &self.capability_id {
            if classification.capability_level(capability_id).is_none() {
                return false;
            }
        }
        if let Some(min_level) = self.min_capability_level {
            let has_matching = classification
                .capability_assessments
                .iter()
                .any(|assessment| assessment.level >= min_level);
            if !has_matching {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Agent Classification Provider - the SPI trait
// ============================================================================

pub trait AgentClassificationProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.agent_classification.unspecified",
            "agent_classification",
            "agent-classification-provider",
            "0.0.0",
            vec![
                "agent.classify".to_string(),
                "agent.classification.get".to_string(),
                "agent.classification.list".to_string(),
                "agent.classification.search".to_string(),
            ],
        )
    }

    fn classify(&mut self, agent_id: &str) -> KernelResult<AgentClassification>;

    fn get_classification(&self, agent_id: &str) -> KernelResult<AgentClassification>;

    fn list_classifications(
        &self,
        query: &ClassificationQuery,
    ) -> KernelResult<Vec<AgentClassification>>;

    fn search_by_capability(
        &self,
        capability_id: &str,
        min_level: CapabilityLevel,
    ) -> KernelResult<Vec<AgentClassification>> {
        Ok(self
            .list_classifications(&ClassificationQuery::new())?
            .into_iter()
            .filter(|classification| {
                classification
                    .capability_level(capability_id)
                    .is_some_and(|level| level >= min_level)
            })
            .collect())
    }

    fn supports_category(&self, category: AgentCategory) -> bool {
        let _ = category;
        true
    }

    fn health(&self) -> ProviderHealth;
}
