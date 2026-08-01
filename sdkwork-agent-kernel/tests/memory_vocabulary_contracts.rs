//! Contract tests for the external memory vocabulary mapping.
//!
//! The kernel tier/scope vocabulary maps onto the sibling sdkwork-memory
//! service types (`working/session/semantic/episodic/procedural/habit/
//! relationship/domain_knowledge` + scope strings) so integrations align
//! records without duplicating models.

use sdkwork_agent_kernel::{ExternalMemoryType, MemoryScope, MemoryTier};

#[test]
fn external_memory_type_vocabulary_is_stable() {
    let cases = vec![
        (ExternalMemoryType::Working, "working"),
        (ExternalMemoryType::Session, "session"),
        (ExternalMemoryType::Semantic, "semantic"),
        (ExternalMemoryType::Episodic, "episodic"),
        (ExternalMemoryType::Procedural, "procedural"),
        (ExternalMemoryType::Habit, "habit"),
        (ExternalMemoryType::Relationship, "relationship"),
        (ExternalMemoryType::DomainKnowledge, "domain_knowledge"),
    ];
    for (kind, word) in cases {
        assert_eq!(kind.as_str(), word);
        assert_eq!(ExternalMemoryType::from_str(word), Some(kind));
    }
    assert_eq!(ExternalMemoryType::from_str("unknown"), None);
}

#[test]
fn external_types_map_onto_kernel_tiers() {
    assert_eq!(
        ExternalMemoryType::Working.to_kernel_tier(),
        MemoryTier::Ephemeral
    );
    assert_eq!(
        ExternalMemoryType::Session.to_kernel_tier(),
        MemoryTier::ShortTerm
    );
    assert_eq!(
        ExternalMemoryType::Semantic.to_kernel_tier(),
        MemoryTier::LongTerm
    );
    assert_eq!(
        ExternalMemoryType::Episodic.to_kernel_tier(),
        MemoryTier::LongTerm
    );
    assert_eq!(
        ExternalMemoryType::Procedural.to_kernel_tier(),
        MemoryTier::LongTerm
    );
    assert_eq!(
        ExternalMemoryType::Relationship.to_kernel_tier(),
        MemoryTier::Permanent
    );
    assert_eq!(
        ExternalMemoryType::DomainKnowledge.to_kernel_tier(),
        MemoryTier::Permanent
    );
    assert_eq!(
        ExternalMemoryType::Habit.to_kernel_tier(),
        MemoryTier::Growing
    );
}

#[test]
fn kernel_tiers_expand_into_external_types() {
    assert_eq!(
        MemoryTier::Ephemeral.to_external_types(),
        vec![ExternalMemoryType::Working]
    );
    assert_eq!(
        MemoryTier::ShortTerm.to_external_types(),
        vec![ExternalMemoryType::Session]
    );
    assert_eq!(
        MemoryTier::LongTerm.to_external_types(),
        vec![
            ExternalMemoryType::Semantic,
            ExternalMemoryType::Episodic,
            ExternalMemoryType::Procedural,
        ]
    );
    assert_eq!(
        MemoryTier::Permanent.to_external_types(),
        vec![
            ExternalMemoryType::Relationship,
            ExternalMemoryType::DomainKnowledge,
        ]
    );
    assert_eq!(
        MemoryTier::Growing.to_external_types(),
        vec![ExternalMemoryType::Habit]
    );
}

#[test]
fn tier_mapping_round_trips() {
    // Every external type maps to a tier that contains it.
    for kind in [
        ExternalMemoryType::Working,
        ExternalMemoryType::Session,
        ExternalMemoryType::Semantic,
        ExternalMemoryType::Episodic,
        ExternalMemoryType::Procedural,
        ExternalMemoryType::Habit,
        ExternalMemoryType::Relationship,
        ExternalMemoryType::DomainKnowledge,
    ] {
        let tier = kind.to_kernel_tier();
        assert!(
            tier.to_external_types().contains(&kind),
            "{kind:?} must round trip through {tier:?}"
        );
    }
}

#[test]
fn scope_vocabulary_maps_both_directions() {
    let cases = vec![
        (MemoryScope::Session, "session"),
        (MemoryScope::User, "user"),
        (MemoryScope::Tenant, "tenant"),
        (MemoryScope::Organization, "organization"),
        (MemoryScope::Agent, "agent"),
        (MemoryScope::Application, "application"),
    ];
    for (scope, word) in cases {
        assert_eq!(scope.to_external_scope(), word);
        assert_eq!(MemoryScope::from_external_scope(word), Some(scope));
    }
    assert_eq!(MemoryScope::from_external_scope("team"), None);
}
