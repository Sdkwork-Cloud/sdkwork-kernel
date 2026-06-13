use sdkwork_agent_kernel::{
    ContextExplanation, ContextFrame, ContextProvider, ContextRanking, KernelResult, MemoryProvider,
    MemoryRecord, MemoryScope, RedactionClassification, TrustLevel,
};

#[test]
fn context_frame_preserves_source_trust_and_redaction_metadata() {
    let frame = ContextFrame::new(
        "context.1",
        "session.1",
        "tool.stdout",
        "build completed",
        TrustLevel::ToolOutput,
        RedactionClassification::Internal,
    )
    .for_task("task.1")
    .with_content_type("text/plain")
    .with_provenance("tool-call.1")
    .created_at("2026-05-27T12:00:00Z")
    .with_metadata("tool.call_id", "tool-call.1");

    assert_eq!(frame.context_frame_id, "context.1");
    assert_eq!(frame.session_id, "session.1");
    assert_eq!(frame.task_id.as_deref(), Some("task.1"));
    assert_eq!(frame.content_type, "text/plain");
    assert_eq!(frame.trust_level, TrustLevel::ToolOutput);
    assert_eq!(frame.provenance.as_deref(), Some("tool-call.1"));
    assert_eq!(frame.created_at.as_deref(), Some("2026-05-27T12:00:00Z"));
    assert_eq!(frame.metadata_value("tool.call_id"), Some("tool-call.1"));
    assert!(frame.is_untrusted());
}

#[test]
fn memory_record_scope_and_classification_are_explicit() {
    let record = MemoryRecord::new(
        "memory.1",
        MemoryScope::Session,
        "session.1",
        "user prefers concise answers",
        TrustLevel::UserSupplied,
        RedactionClassification::PersonalData,
    )
    .with_content_type("text/plain")
    .with_source("chat.user")
    .with_retention_policy("delete_after_30_days")
    .created_at("2026-05-27T12:00:00Z")
    .updated_at("2026-05-27T12:00:01Z")
    .with_policy_decision("policy-decision.1")
    .with_metadata("memory.profile_id", "profile.user.preferences");

    assert_eq!(record.scope, MemoryScope::Session);
    assert_eq!(record.owner_context, "session.1");
    assert_eq!(record.content_type, "text/plain");
    assert_eq!(record.source.as_deref(), Some("chat.user"));
    assert_eq!(
        record.retention_policy.as_deref(),
        Some("delete_after_30_days")
    );
    assert_eq!(
        record.policy_decision_id.as_deref(),
        Some("policy-decision.1")
    );
    assert_eq!(
        record.metadata_value("memory.profile_id"),
        Some("profile.user.preferences")
    );
    assert!(record.requires_redaction());
}

#[test]
fn context_provider_trait_supports_deterministic_fake_context() {
    let provider = FakeContextProvider;
    let frames = provider.collect("session.1").expect("context collected");

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].source, "fake.context");
    assert_eq!(frames[0].trust_level, TrustLevel::TrustedHost);
}

#[test]
fn memory_provider_trait_supports_query_write_delete_and_export() {
    let mut provider = FakeMemoryProvider::default();
    let record = MemoryRecord::new(
        "memory.1",
        MemoryScope::User,
        "user.1",
        "prefers code examples",
        TrustLevel::UserSupplied,
        RedactionClassification::PersonalData,
    );

    provider.write(record.clone()).expect("write succeeds");
    assert_eq!(
        provider.query(MemoryScope::User, "user.1").unwrap(),
        vec![record]
    );

    let exported = provider.export(MemoryScope::User, "user.1").unwrap();
    assert_eq!(exported.len(), 1);

    provider.delete("memory.1").expect("delete succeeds");
    assert!(provider
        .query(MemoryScope::User, "user.1")
        .unwrap()
        .is_empty());
}

#[test]
fn context_provider_rank_returns_scored_frames() {
    let provider = FakeContextProvider;
    let frames = provider.collect("session.1").expect("context collected");
    let rankings = provider.rank(&frames).expect("ranking succeeds");

    assert_eq!(rankings.len(), 1);
    assert_eq!(rankings[0].frame_id, "context.fake");
    assert!(rankings[0].relevance_score > 0.0);
    assert!(rankings[0].relevance_score <= 1.0);
}

#[test]
fn context_provider_trim_passes_through_with_default() {
    let provider = FakeContextProvider;
    let frames = provider.collect("session.1").expect("context collected");
    let trimmed = provider.trim(frames.clone(), 100).expect("trim succeeds");

    assert_eq!(trimmed.len(), frames.len());
    assert_eq!(trimmed[0].context_frame_id, frames[0].context_frame_id);
}

#[test]
fn context_provider_explain_returns_default_explanation() {
    let provider = FakeContextProvider;
    let frames = provider.collect("session.1").expect("context collected");
    let explanation = provider.explain(&frames[0]).expect("explain succeeds");

    assert_eq!(explanation.frame_id, "context.fake");
    assert!(!explanation.reason.is_empty());
}

struct FakeContextProvider;

impl ContextProvider for FakeContextProvider {
    fn collect(&self, session_id: &str) -> KernelResult<Vec<ContextFrame>> {
        Ok(vec![ContextFrame::new(
            "context.fake",
            session_id,
            "fake.context",
            "fake context",
            TrustLevel::TrustedHost,
            RedactionClassification::Internal,
        )])
    }
}

#[derive(Default)]
struct FakeMemoryProvider {
    records: Vec<MemoryRecord>,
}

impl MemoryProvider for FakeMemoryProvider {
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
}
