use sdkwork_agent_kernel::{
    AgentCategory, AgentClassification, AgentClassificationProvider, AutonomyLevel,
    CapabilityAssessment, CapabilityLevel, ClassificationQuery, KernelResult, ProviderHealth,
};

// ============================================================================
// Agent Category contracts
// ============================================================================

#[test]
fn agent_category_str_roundtrip_preserves_identity() {
    for category in AgentCategory::all() {
        let label = category.as_str();
        assert_eq!(AgentCategory::from_str(label), Some(*category));
    }
    assert_eq!(AgentCategory::from_str("nonexistent"), None);
}

#[test]
fn agent_category_all_returns_nine_categories() {
    assert_eq!(AgentCategory::all().len(), 9);
    assert!(AgentCategory::all().contains(&AgentCategory::Coding));
    assert!(AgentCategory::all().contains(&AgentCategory::Autonomous));
    assert!(AgentCategory::all().contains(&AgentCategory::Hybrid));
}

// ============================================================================
// Autonomy Level contracts
// ============================================================================

#[test]
fn autonomy_level_str_roundtrip_preserves_identity() {
    let all_levels = [
        AutonomyLevel::Manual,
        AutonomyLevel::Supervised,
        AutonomyLevel::SemiAutonomous,
        AutonomyLevel::FullyAutonomous,
    ];

    for level in &all_levels {
        let label = level.as_str();
        assert_eq!(AutonomyLevel::from_str(label), Some(*level));
    }
    assert_eq!(AutonomyLevel::from_str("unknown"), None);
}

#[test]
fn autonomy_level_ordering_is_monotonic() {
    assert!(AutonomyLevel::FullyAutonomous > AutonomyLevel::SemiAutonomous);
    assert!(AutonomyLevel::SemiAutonomous > AutonomyLevel::Supervised);
    assert!(AutonomyLevel::Supervised > AutonomyLevel::Manual);
}

#[test]
fn autonomy_level_default_is_supervised() {
    assert_eq!(AutonomyLevel::default(), AutonomyLevel::Supervised);
}

#[test]
fn autonomy_level_numeric_value_increases_with_autonomy() {
    assert_eq!(AutonomyLevel::Manual.numeric_value(), 0);
    assert_eq!(AutonomyLevel::Supervised.numeric_value(), 1);
    assert_eq!(AutonomyLevel::SemiAutonomous.numeric_value(), 2);
    assert_eq!(AutonomyLevel::FullyAutonomous.numeric_value(), 3);
}

// ============================================================================
// Capability Level contracts
// ============================================================================

#[test]
fn capability_level_str_roundtrip_preserves_identity() {
    let all_levels = [
        CapabilityLevel::Basic,
        CapabilityLevel::Intermediate,
        CapabilityLevel::Advanced,
        CapabilityLevel::Expert,
    ];

    for level in &all_levels {
        let label = level.as_str();
        assert_eq!(CapabilityLevel::from_str(label), Some(*level));
    }
    assert_eq!(CapabilityLevel::from_str("master"), None);
}

#[test]
fn capability_level_ordering_is_monotonic() {
    assert!(CapabilityLevel::Expert > CapabilityLevel::Advanced);
    assert!(CapabilityLevel::Advanced > CapabilityLevel::Intermediate);
    assert!(CapabilityLevel::Intermediate > CapabilityLevel::Basic);
}

#[test]
fn capability_assessment_preserves_evidence() {
    let assessment = CapabilityAssessment::new("code.generation", CapabilityLevel::Expert)
        .with_evidence("demonstrated via 500+ successful PR reviews");

    assert_eq!(assessment.capability_id, "code.generation");
    assert_eq!(assessment.level, CapabilityLevel::Expert);
    assert!(assessment
        .evidence
        .as_deref()
        .unwrap()
        .contains("500+ successful PR reviews"));
}

// ============================================================================
// Agent Classification builder contracts
// ============================================================================

#[test]
fn agent_classification_builder_preserves_categories_and_capabilities() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_secondary_category(AgentCategory::Autonomous)
        .with_autonomy_level(AutonomyLevel::SemiAutonomous)
        .with_capability(CapabilityAssessment::new(
            "code.generation",
            CapabilityLevel::Expert,
        ))
        .with_capability(CapabilityAssessment::new(
            "code.review",
            CapabilityLevel::Advanced,
        ))
        .with_tag("rust")
        .with_tag("typescript")
        .classified_at("2026-06-27T12:00:00Z")
        .with_metadata("source", "auto-classification");

    assert_eq!(classification.agent_id, "agent.codex");
    assert_eq!(classification.primary_category, AgentCategory::Coding);
    assert_eq!(
        classification.secondary_categories,
        vec![AgentCategory::Autonomous]
    );
    assert_eq!(classification.autonomy_level, AutonomyLevel::SemiAutonomous);
    assert_eq!(classification.capability_assessments.len(), 2);
    assert_eq!(
        classification.capability_level("code.generation"),
        Some(CapabilityLevel::Expert)
    );
    assert_eq!(
        classification.capability_level("code.review"),
        Some(CapabilityLevel::Advanced)
    );
    assert_eq!(classification.capability_level("nonexistent"), None);
    assert!(classification.tags.contains(&"rust".to_string()));
    assert!(classification.tags.contains(&"typescript".to_string()));
    assert_eq!(
        classification.classified_at.as_deref(),
        Some("2026-06-27T12:00:00Z")
    );
    assert_eq!(
        classification.metadata_value("source"),
        Some("auto-classification")
    );
}

#[test]
fn agent_classification_all_categories_includes_primary_and_secondary() {
    let classification = AgentClassification::new("agent.hybrid", AgentCategory::Coding)
        .with_secondary_category(AgentCategory::Research)
        .with_secondary_category(AgentCategory::Analytical);

    let all = classification.all_categories();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&AgentCategory::Coding));
    assert!(all.contains(&AgentCategory::Research));
    assert!(all.contains(&AgentCategory::Analytical));
}

#[test]
fn agent_classification_has_category_checks_primary_and_secondary() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_secondary_category(AgentCategory::Autonomous);

    assert!(classification.has_category(AgentCategory::Coding));
    assert!(classification.has_category(AgentCategory::Autonomous));
    assert!(!classification.has_category(AgentCategory::Creative));
}

#[test]
fn agent_classification_secondary_category_deduplicates_and_excludes_primary() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_secondary_category(AgentCategory::Coding)
        .with_secondary_category(AgentCategory::Research)
        .with_secondary_category(AgentCategory::Research);

    assert_eq!(classification.secondary_categories.len(), 1);
    assert_eq!(
        classification.secondary_categories[0],
        AgentCategory::Research
    );
}

#[test]
fn agent_classification_highest_capability_level_returns_max() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_capability(CapabilityAssessment::new(
            "code.gen",
            CapabilityLevel::Basic,
        ))
        .with_capability(CapabilityAssessment::new(
            "code.review",
            CapabilityLevel::Expert,
        ))
        .with_capability(CapabilityAssessment::new(
            "code.test",
            CapabilityLevel::Intermediate,
        ));

    assert_eq!(
        classification.highest_capability_level(),
        Some(CapabilityLevel::Expert)
    );
}

#[test]
fn agent_classification_tag_deduplicates() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_tag("rust")
        .with_tag("rust")
        .with_tag("typescript");

    assert_eq!(classification.tags.len(), 2);
}

#[test]
fn agent_classification_validate_rejects_empty_agent_id() {
    let classification = AgentClassification::new("", AgentCategory::Coding);
    assert!(classification.validate().is_err());
}

#[test]
fn agent_classification_validate_rejects_empty_capability_id() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_capability(CapabilityAssessment::new("", CapabilityLevel::Advanced));
    assert!(classification.validate().is_err());
}

#[test]
fn agent_classification_validate_rejects_blank_tags() {
    let mut classification = AgentClassification::new("agent.codex", AgentCategory::Coding);
    classification.tags.push("  ".to_string());
    assert!(classification.validate().is_err());
}

#[test]
fn agent_classification_validate_accepts_well_formed_classification() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_autonomy_level(AutonomyLevel::SemiAutonomous)
        .with_capability(CapabilityAssessment::new(
            "code.gen",
            CapabilityLevel::Expert,
        ))
        .with_tag("rust");

    assert!(classification.validate().is_ok());
}

// ============================================================================
// Classification Query contracts
// ============================================================================

#[test]
fn classification_query_matches_by_category() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_secondary_category(AgentCategory::Autonomous);

    let query = ClassificationQuery::new().in_category(AgentCategory::Coding);
    assert!(query.matches(&classification));

    let query = ClassificationQuery::new().in_category(AgentCategory::Autonomous);
    assert!(query.matches(&classification));

    let query = ClassificationQuery::new().in_category(AgentCategory::Creative);
    assert!(!query.matches(&classification));
}

#[test]
fn classification_query_matches_by_autonomy_level() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_autonomy_level(AutonomyLevel::FullyAutonomous);

    let query = ClassificationQuery::new().with_autonomy(AutonomyLevel::FullyAutonomous);
    assert!(query.matches(&classification));

    let query = ClassificationQuery::new().with_autonomy(AutonomyLevel::Manual);
    assert!(!query.matches(&classification));
}

#[test]
fn classification_query_matches_by_tag() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_tag("rust")
        .with_tag("production");

    let query = ClassificationQuery::new().with_tag("rust");
    assert!(query.matches(&classification));

    let query = ClassificationQuery::new().with_tag("python");
    assert!(!query.matches(&classification));
}

#[test]
fn classification_query_matches_by_capability_id_and_min_level() {
    let classification = AgentClassification::new("agent.codex", AgentCategory::Coding)
        .with_capability(CapabilityAssessment::new(
            "code.gen",
            CapabilityLevel::Advanced,
        ))
        .with_capability(CapabilityAssessment::new(
            "code.review",
            CapabilityLevel::Basic,
        ));

    let query = ClassificationQuery::new().with_capability_id("code.gen");
    assert!(query.matches(&classification));

    let query = ClassificationQuery::new().with_capability_id("nonexistent");
    assert!(!query.matches(&classification));

    let query = ClassificationQuery::new().with_min_capability(CapabilityLevel::Advanced);
    assert!(query.matches(&classification));

    let query = ClassificationQuery::new().with_min_capability(CapabilityLevel::Expert);
    assert!(!query.matches(&classification));
}

// ============================================================================
// Agent Classification Provider SPI contracts
// ============================================================================

#[test]
fn classification_provider_manifest_declares_standard_capabilities() {
    let provider = FakeClassificationProvider::default();
    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_family, "agent_classification");
    assert!(manifest
        .capabilities
        .contains(&"agent.classify".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"agent.classification.list".to_string()));
}

#[test]
fn classification_provider_classify_and_get_round_trip() {
    let mut provider = FakeClassificationProvider::default();

    let classification = provider.classify("agent.codex").expect("classify succeeds");
    assert_eq!(classification.agent_id, "agent.codex");
    assert_eq!(classification.primary_category, AgentCategory::Coding);

    let retrieved = provider
        .get_classification("agent.codex")
        .expect("get succeeds");
    assert_eq!(retrieved.agent_id, "agent.codex");
}

#[test]
fn classification_provider_list_filters_by_query() {
    let provider = FakeClassificationProvider::with_samples();

    let coding_agents = provider
        .list_classifications(&ClassificationQuery::new().in_category(AgentCategory::Coding))
        .expect("list succeeds");
    assert!(!coding_agents.is_empty());
    assert!(coding_agents
        .iter()
        .all(|c| c.has_category(AgentCategory::Coding)));
}

#[test]
fn classification_provider_search_by_capability_filters_by_level() {
    let provider = FakeClassificationProvider::with_samples();

    let experts = provider
        .search_by_capability("code.generation", CapabilityLevel::Expert)
        .expect("search succeeds");
    assert!(!experts.is_empty());
    assert!(experts.iter().all(|c| c
        .capability_level("code.generation")
        .is_some_and(|level| level >= CapabilityLevel::Expert)));
}

#[test]
fn classification_provider_supports_category_returns_true_by_default() {
    let provider = FakeClassificationProvider::default();
    assert!(provider.supports_category(AgentCategory::Coding));
    assert!(provider.supports_category(AgentCategory::Creative));
    assert!(provider.supports_category(AgentCategory::MultiModal));
}

// ============================================================================
// Fake Classification Provider
// ============================================================================

#[derive(Default)]
struct FakeClassificationProvider {
    classifications: Vec<AgentClassification>,
}

impl FakeClassificationProvider {
    fn with_samples() -> Self {
        let codex = AgentClassification::new("agent.codex", AgentCategory::Coding)
            .with_autonomy_level(AutonomyLevel::SemiAutonomous)
            .with_capability(CapabilityAssessment::new(
                "code.generation",
                CapabilityLevel::Expert,
            ))
            .with_capability(CapabilityAssessment::new(
                "code.review",
                CapabilityLevel::Advanced,
            ))
            .with_tag("rust");

        let claude = AgentClassification::new("agent.claude", AgentCategory::Coding)
            .with_autonomy_level(AutonomyLevel::Supervised)
            .with_capability(CapabilityAssessment::new(
                "code.generation",
                CapabilityLevel::Advanced,
            ))
            .with_tag("typescript");

        let researcher = AgentClassification::new("agent.researcher", AgentCategory::Research)
            .with_autonomy_level(AutonomyLevel::FullyAutonomous)
            .with_capability(CapabilityAssessment::new(
                "research.synthesis",
                CapabilityLevel::Expert,
            ));

        Self {
            classifications: vec![codex, claude, researcher],
        }
    }
}

impl AgentClassificationProvider for FakeClassificationProvider {
    fn classify(&mut self, agent_id: &str) -> KernelResult<AgentClassification> {
        let classification = AgentClassification::new(agent_id, AgentCategory::Coding)
            .with_autonomy_level(AutonomyLevel::SemiAutonomous)
            .with_capability(CapabilityAssessment::new(
                "code.generation",
                CapabilityLevel::Advanced,
            ));
        self.classifications.push(classification.clone());
        Ok(classification)
    }

    fn get_classification(&self, agent_id: &str) -> KernelResult<AgentClassification> {
        self.classifications
            .iter()
            .find(|c| c.agent_id == agent_id)
            .cloned()
            .ok_or_else(|| {
                sdkwork_agent_kernel::KernelError::validation("classification not found")
            })
    }

    fn list_classifications(
        &self,
        query: &ClassificationQuery,
    ) -> KernelResult<Vec<AgentClassification>> {
        Ok(self
            .classifications
            .iter()
            .filter(|c| query.matches(c))
            .cloned()
            .collect())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
