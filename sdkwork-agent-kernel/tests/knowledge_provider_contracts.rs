use sdkwork_agent_kernel::{
    KernelResult, KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind,
    KnowledgeProvider, KnowledgeRetrievalMethod, KnowledgeSearchRequest, KnowledgeSearchResult,
    ProviderHealth, ProviderManifest, RedactionClassification, TraceContext, TrustLevel,
};

#[test]
fn knowledge_provider_supports_non_vector_retrieval_methods() {
    let provider = FakeKnowledgeProvider;
    let request = KnowledgeSearchRequest::new("agent spi standard")
        .for_session("session.knowledge.1")
        .for_task("task.knowledge.1")
        .for_run("run.knowledge.1")
        .for_step("step.knowledge.1")
        .with_namespace("sdkwork.kernel")
        .with_tenant_id("tenant.sdkwork")
        .with_policy_context("policy-decision.knowledge.1")
        .with_trace_context(TraceContext::new("trace.knowledge.1", "span.knowledge.search"))
        .with_timeout_ms(25_000)
        .with_metadata("rag.pipeline", "wiki")
        .with_method(KnowledgeRetrievalMethod::Keyword)
        .with_method(KnowledgeRetrievalMethod::Graph)
        .with_method(KnowledgeRetrievalMethod::Structured)
        .with_filter("doc.kind", "wiki_section")
        .with_top_k(5);

    assert!(request.supports_method(KnowledgeRetrievalMethod::Keyword));
    assert!(request.supports_method(KnowledgeRetrievalMethod::Graph));
    assert!(request.supports_method(KnowledgeRetrievalMethod::Structured));
    assert!(!request.supports_method(KnowledgeRetrievalMethod::Vector));
    assert_eq!(request.session_id.as_deref(), Some("session.knowledge.1"));
    assert_eq!(request.task_id.as_deref(), Some("task.knowledge.1"));
    assert_eq!(request.run_id.as_deref(), Some("run.knowledge.1"));
    assert_eq!(request.step_id.as_deref(), Some("step.knowledge.1"));
    assert_eq!(
        request.policy_decision_id.as_deref(),
        Some("policy-decision.knowledge.1")
    );
    assert_eq!(
        request.trace_context.as_ref().unwrap().span_id,
        "span.knowledge.search"
    );
    assert_eq!(request.timeout_ms, Some(25_000));
    assert_eq!(request.metadata_value("rag.pipeline"), Some("wiki"));

    let results = provider
        .search(request)
        .expect("knowledge provider searches with non-vector methods");

    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].retrieval_method,
        KnowledgeRetrievalMethod::Keyword
    );
    assert_eq!(results[1].retrieval_method, KnowledgeRetrievalMethod::Graph);
    assert_eq!(results[0].document_kind, KnowledgeDocumentKind::WikiSection);
}

#[test]
fn knowledge_documents_convert_to_safe_context_frames() {
    let document = KnowledgeDocument::new(
        "knowledge.wiki.agent_spi",
        KnowledgeDocumentKind::WikiSection,
        "Agent SPI",
        "Provider families are standardized kernel contracts.",
    )
    .with_namespace("sdkwork.kernel")
    .with_source_uri("wiki://sdkwork/kernel/agent-spi")
    .with_tag("spi")
    .with_retrieval_method(KnowledgeRetrievalMethod::Keyword)
    .with_content_type("text/markdown")
    .with_trust_level(TrustLevel::RetrievedExternal)
    .with_redaction_classification(RedactionClassification::Internal)
    .with_metadata("section", "provider-family");

    let frame = document.to_context_frame("session.knowledge.1");

    assert_eq!(
        frame.context_frame_id,
        "context.knowledge.knowledge.wiki.agent_spi"
    );
    assert_eq!(frame.session_id, "session.knowledge.1");
    assert_eq!(frame.source, "knowledge");
    assert_eq!(frame.content_type, "text/markdown");
    assert_eq!(frame.content, document.content);
    assert_eq!(
        frame.provenance.as_deref(),
        Some("wiki://sdkwork/kernel/agent-spi")
    );
    assert_eq!(frame.metadata_value("section"), Some("provider-family"));
    assert_eq!(frame.trust_level, TrustLevel::RetrievedExternal);
    assert_eq!(
        frame.redaction_classification,
        RedactionClassification::Internal
    );
    assert!(frame.is_untrusted());
}

#[test]
fn knowledge_provider_manifest_declares_standard_capabilities() {
    let provider = FakeKnowledgeProvider;
    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_family, "knowledge");
    assert_eq!(manifest.provider_id, "provider.knowledge.fake-wiki");
    assert!(manifest
        .capabilities
        .contains(&"knowledge.search".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"knowledge.read".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"knowledge.list".to_string()));
}

#[test]
fn knowledge_filter_matches_kind_namespace_tags_and_external_policy() {
    let internal = KnowledgeDocument::new(
        "knowledge.internal",
        KnowledgeDocumentKind::Spec,
        "Internal Spec",
        "internal content",
    )
    .with_namespace("sdkwork.kernel")
    .with_tag("spec");
    let external = KnowledgeDocument::new(
        "knowledge.external",
        KnowledgeDocumentKind::ExternalReference,
        "External Reference",
        "external content",
    )
    .with_namespace("sdkwork.kernel")
    .with_source_uri("https://example.invalid/reference")
    .with_tag("spec");

    let filter = KnowledgeDocumentFilter::new()
        .with_kind(KnowledgeDocumentKind::Spec)
        .with_namespace("sdkwork.kernel")
        .with_tag("spec");

    assert!(filter.matches(&internal));
    assert!(!filter.matches(&external));

    let external_filter = KnowledgeDocumentFilter::new()
        .with_kind(KnowledgeDocumentKind::ExternalReference)
        .with_namespace("sdkwork.kernel")
        .with_tag("spec");
    assert!(!external_filter.matches(&external));
    assert!(external_filter.include_external().matches(&external));
}

struct FakeKnowledgeProvider;

impl KnowledgeProvider for FakeKnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.knowledge.fake-wiki",
            "knowledge",
            "fake-wiki",
            "0.1.0",
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        Ok(vec![
            KnowledgeSearchResult::new(
                "knowledge.wiki.agent_spi",
                KnowledgeDocumentKind::WikiSection,
                "Agent SPI",
                KnowledgeRetrievalMethod::Keyword,
            )
            .with_snippet(format!("keyword result for {}", request.query))
            .with_score(0.82)
            .with_source_uri("wiki://sdkwork/kernel/agent-spi")
            .with_trust_level(TrustLevel::RetrievedExternal)
            .with_redaction_classification(RedactionClassification::Internal),
            KnowledgeSearchResult::new(
                "knowledge.graph.provider_family",
                KnowledgeDocumentKind::WikiSection,
                "Provider Family",
                KnowledgeRetrievalMethod::Graph,
            )
            .with_score(0.73)
            .with_source_uri("wiki://sdkwork/kernel/provider-family"),
        ])
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::WikiPage,
            "Agent SPI",
            "read document",
        ))
    }

    fn list(&self, _filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        Ok(vec![KnowledgeDocument::new(
            "knowledge.wiki.agent_spi",
            KnowledgeDocumentKind::WikiPage,
            "Agent SPI",
            "listed document",
        )])
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
