use sdkwork_agent_kernel::{
    ContextFrame, ContextProvider, KernelResult, MemoryProvider, MemoryRecord, MemoryScope,
    RedactionClassification, TrustLevel,
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
    .with_provenance("tool-call.1");

    assert_eq!(frame.context_frame_id, "context.1");
    assert_eq!(frame.trust_level, TrustLevel::ToolOutput);
    assert_eq!(frame.provenance.as_deref(), Some("tool-call.1"));
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
    );

    assert_eq!(record.scope, MemoryScope::Session);
    assert_eq!(record.owner_context, "session.1");
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
