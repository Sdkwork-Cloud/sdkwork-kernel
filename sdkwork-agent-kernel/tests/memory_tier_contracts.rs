use sdkwork_agent_kernel::{
    KernelResult, MemoryProvider, MemoryRecord, MemoryScope, MemoryTier, ProviderHealth,
    RedactionClassification, TrustLevel,
};

// ============================================================================
// Memory Tier contracts
// ============================================================================

#[test]
fn memory_tier_str_roundtrip_preserves_identity() {
    let all_tiers = [
        MemoryTier::Ephemeral,
        MemoryTier::ShortTerm,
        MemoryTier::LongTerm,
        MemoryTier::Permanent,
        MemoryTier::Growing,
    ];

    for tier in &all_tiers {
        let label = tier.as_str();
        assert_eq!(MemoryTier::from_str(label), Some(*tier));
    }

    assert_eq!(MemoryTier::from_str("unknown"), None);
}

#[test]
fn memory_tier_default_is_short_term() {
    assert_eq!(MemoryTier::default(), MemoryTier::ShortTerm);
}

#[test]
fn memory_tier_is_persistent_classifies_long_term_permanent_and_growing() {
    assert!(!MemoryTier::Ephemeral.is_persistent());
    assert!(!MemoryTier::ShortTerm.is_persistent());
    assert!(MemoryTier::LongTerm.is_persistent());
    assert!(MemoryTier::Permanent.is_persistent());
    assert!(MemoryTier::Growing.is_persistent());
}

#[test]
fn memory_tier_can_evolve_only_for_growing() {
    assert!(!MemoryTier::Ephemeral.can_evolve());
    assert!(!MemoryTier::ShortTerm.can_evolve());
    assert!(!MemoryTier::LongTerm.can_evolve());
    assert!(!MemoryTier::Permanent.can_evolve());
    assert!(MemoryTier::Growing.can_evolve());
}

// ============================================================================
// Memory Record tier contracts
// ============================================================================

#[test]
fn memory_record_with_tier_preserves_tier_classification() {
    let record = MemoryRecord::new(
        "memory.permanent.1",
        MemoryScope::User,
        "user.1",
        "user core preferences",
        TrustLevel::UserSupplied,
        RedactionClassification::PersonalData,
    )
    .with_tier(MemoryTier::Permanent);

    assert_eq!(record.tier, MemoryTier::Permanent);
    assert!(record.is_persistent());
    assert!(record.is_permanent());
    assert!(!record.is_growing());
}

#[test]
fn memory_record_growing_tier_supports_evolution_check() {
    let record = MemoryRecord::new(
        "memory.growing.1",
        MemoryScope::Agent,
        "agent.codex",
        "accumulated knowledge about codebase",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing);

    assert_eq!(record.tier, MemoryTier::Growing);
    assert!(record.is_growing());
    assert!(record.tier.can_evolve());
}

#[test]
fn memory_record_consolidate_into_preserves_parent_link_and_increments_count() {
    let parent = MemoryRecord::new(
        "memory.growing.parent",
        MemoryScope::Agent,
        "agent.codex",
        "original content",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing)
    .with_metadata("source", "initial");

    let child = parent.consolidate_into("memory.growing.child", "consolidated content");

    assert_eq!(child.memory_record_id, "memory.growing.child");
    assert_eq!(child.content, "consolidated content");
    assert_eq!(
        child.parent_record_id.as_deref(),
        Some("memory.growing.parent")
    );
    assert_eq!(child.consolidation_count, parent.consolidation_count + 1);
    assert_eq!(child.tier, MemoryTier::Growing);
    assert_eq!(child.scope, parent.scope);
    assert_eq!(child.owner_context, parent.owner_context);
    assert_eq!(child.metadata_value("source"), Some("initial"));
}

#[test]
fn memory_record_has_expired_checks_expires_at_against_current_time() {
    let expired = MemoryRecord::new(
        "memory.expired.1",
        MemoryScope::Session,
        "session.1",
        "ephemeral data",
        TrustLevel::ToolOutput,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::ShortTerm)
    .with_expires_at("2026-06-27T00:00:00Z");

    assert!(expired.has_expired("2026-06-27T12:00:00Z"));
    assert!(!expired.has_expired("2026-06-26T12:00:00Z"));

    let no_expiry = MemoryRecord::new(
        "memory.permanent.1",
        MemoryScope::User,
        "user.1",
        "permanent data",
        TrustLevel::TrustedSystem,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Permanent);

    assert!(!no_expiry.has_expired("2099-01-01T00:00:00Z"));
}

// ============================================================================
// Memory Provider query_by_tier default implementation contracts
// ============================================================================

#[test]
fn memory_provider_query_by_tier_filters_records_by_tier() {
    let mut provider = FakeTierMemoryProvider::default();

    let ephemeral = MemoryRecord::new(
        "memory.ephemeral.1",
        MemoryScope::Session,
        "session.1",
        "ephemeral context",
        TrustLevel::ToolOutput,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Ephemeral);

    let permanent = MemoryRecord::new(
        "memory.permanent.1",
        MemoryScope::Session,
        "session.1",
        "permanent fact",
        TrustLevel::TrustedSystem,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Permanent);

    let growing = MemoryRecord::new(
        "memory.growing.1",
        MemoryScope::Session,
        "session.1",
        "growing knowledge",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing);

    provider.write(ephemeral).expect("write ephemeral");
    provider.write(permanent).expect("write permanent");
    provider.write(growing).expect("write growing");

    let permanent_records = provider
        .query_by_tier(MemoryScope::Session, "session.1", MemoryTier::Permanent)
        .expect("query permanent");
    assert_eq!(permanent_records.len(), 1);
    assert_eq!(permanent_records[0].memory_record_id, "memory.permanent.1");

    let growing_records = provider
        .query_by_tier(MemoryScope::Session, "session.1", MemoryTier::Growing)
        .expect("query growing");
    assert_eq!(growing_records.len(), 1);
    assert_eq!(growing_records[0].memory_record_id, "memory.growing.1");

    let ephemeral_records = provider
        .query_by_tier(MemoryScope::Session, "session.1", MemoryTier::Ephemeral)
        .expect("query ephemeral");
    assert_eq!(ephemeral_records.len(), 1);
}

// ============================================================================
// Memory Provider consolidate default implementation contracts
// ============================================================================

#[test]
fn memory_provider_consolidate_merges_growing_records() {
    let mut provider = FakeTierMemoryProvider::default();

    let record_a = MemoryRecord::new(
        "memory.growing.a",
        MemoryScope::Agent,
        "agent.codex",
        "fact A about codebase",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing);

    let record_b = MemoryRecord::new(
        "memory.growing.b",
        MemoryScope::Agent,
        "agent.codex",
        "fact B about codebase",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing);

    provider.write(record_a).expect("write A");
    provider.write(record_b).expect("write B");

    let consolidated = provider
        .consolidate(MemoryScope::Agent, "agent.codex")
        .expect("consolidate succeeds");

    assert_eq!(consolidated.len(), 1);
    let merged = &consolidated[0];
    assert!(merged.content.contains("fact A about codebase"));
    assert!(merged.content.contains("fact B about codebase"));
    assert!(merged.content.contains("---"));
    assert_eq!(merged.consolidation_count, 1);
    assert_eq!(merged.parent_record_id.as_deref(), Some("memory.growing.a"));
    assert_eq!(merged.tier, MemoryTier::Growing);

    // Original records should be deleted
    let remaining = provider
        .query_by_tier(MemoryScope::Agent, "agent.codex", MemoryTier::Growing)
        .expect("query after consolidate");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].memory_record_id, merged.memory_record_id);
}

#[test]
fn memory_provider_consolidate_with_single_record_returns_as_is() {
    let mut provider = FakeTierMemoryProvider::default();

    let single = MemoryRecord::new(
        "memory.growing.single",
        MemoryScope::Agent,
        "agent.codex",
        "only record",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing);

    provider.write(single).expect("write single");

    let result = provider
        .consolidate(MemoryScope::Agent, "agent.codex")
        .expect("consolidate succeeds");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].memory_record_id, "memory.growing.single");
    assert_eq!(result[0].consolidation_count, 0);
}

// ============================================================================
// Memory Provider evolve default implementation contracts
// ============================================================================

#[test]
fn memory_provider_evolve_appends_content_to_growing_record() {
    let mut provider = FakeTierMemoryProvider::default();

    let original = MemoryRecord::new(
        "memory.growing.evolve",
        MemoryScope::Agent,
        "agent.codex",
        "initial knowledge",
        TrustLevel::AgentMessage,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Growing);

    provider.write(original).expect("write original");

    let evolved = provider
        .evolve(
            MemoryScope::Agent,
            "agent.codex",
            "memory.growing.evolve",
            "new observation about the codebase".to_string(),
        )
        .expect("evolve succeeds");

    assert!(evolved.content.contains("initial knowledge"));
    assert!(evolved
        .content
        .contains("new observation about the codebase"));
    assert_eq!(evolved.consolidation_count, 1);
    assert_eq!(
        evolved.parent_record_id.as_deref(),
        Some("memory.growing.evolve")
    );
    assert_eq!(evolved.tier, MemoryTier::Growing);

    // Original should be replaced
    let records = provider
        .query_by_tier(MemoryScope::Agent, "agent.codex", MemoryTier::Growing)
        .expect("query after evolve");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].memory_record_id, evolved.memory_record_id);
}

#[test]
fn memory_provider_evolve_rejects_non_growing_record() {
    let mut provider = FakeTierMemoryProvider::default();

    let permanent = MemoryRecord::new(
        "memory.permanent.evolve",
        MemoryScope::Agent,
        "agent.codex",
        "permanent fact",
        TrustLevel::TrustedSystem,
        RedactionClassification::Internal,
    )
    .with_tier(MemoryTier::Permanent);

    provider.write(permanent).expect("write permanent");

    let result = provider.evolve(
        MemoryScope::Agent,
        "agent.codex",
        "memory.permanent.evolve",
        "additional content".to_string(),
    );
    assert!(result.is_err());
}

#[test]
fn memory_provider_evolve_rejects_unknown_record() {
    let mut provider = FakeTierMemoryProvider::default();

    let result = provider.evolve(
        MemoryScope::Agent,
        "agent.codex",
        "memory.nonexistent",
        "content".to_string(),
    );
    assert!(result.is_err());
}

// ============================================================================
// Memory Provider manifest declares tier-aware capabilities
// ============================================================================

#[test]
fn memory_provider_manifest_declares_tier_aware_capabilities() {
    let provider = FakeTierMemoryProvider::default();
    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_family, "memory");
    assert!(manifest
        .capabilities
        .contains(&"memory.consolidate".to_string()));
    assert!(manifest.capabilities.contains(&"memory.evolve".to_string()));
    assert!(manifest.capabilities.contains(&"memory.query".to_string()));
}

// ============================================================================
// Fake Memory Provider with tier support
// ============================================================================

#[derive(Default)]
struct FakeTierMemoryProvider {
    records: Vec<MemoryRecord>,
}

impl MemoryProvider for FakeTierMemoryProvider {
    fn query(&self, scope: MemoryScope, owner_context: &str) -> KernelResult<Vec<MemoryRecord>> {
        Ok(self
            .records
            .iter()
            .filter(|record| record.scope == scope && record.owner_context == owner_context)
            .cloned()
            .collect())
    }

    fn write(&mut self, record: MemoryRecord) -> KernelResult<()> {
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
