use crate::{
    ContextFrame, KernelResult, ProviderHealth, ProviderManifest, RedactionClassification,
    TraceContext, TrustLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeRetrievalMethod {
    Exact,
    Keyword,
    FullText,
    Structured,
    Graph,
    Vector,
    Hybrid,
    LlmRerank,
    External,
}

impl KnowledgeRetrievalMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Keyword => "keyword",
            Self::FullText => "full_text",
            Self::Structured => "structured",
            Self::Graph => "graph",
            Self::Vector => "vector",
            Self::Hybrid => "hybrid",
            Self::LlmRerank => "llm_rerank",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeDocumentKind {
    WikiPage,
    WikiSection,
    Article,
    Faq,
    ApiReference,
    Spec,
    Runbook,
    Policy,
    ExternalReference,
    Other,
}

impl KnowledgeDocumentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WikiPage => "wiki_page",
            Self::WikiSection => "wiki_section",
            Self::Article => "article",
            Self::Faq => "faq",
            Self::ApiReference => "api_reference",
            Self::Spec => "spec",
            Self::Runbook => "runbook",
            Self::Policy => "policy",
            Self::ExternalReference => "external_reference",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeSearchRequest {
    pub query: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub tenant_id: Option<String>,
    pub namespace: Option<String>,
    pub top_k: usize,
    pub methods: Vec<KnowledgeRetrievalMethod>,
    pub filters: Vec<(String, String)>,
    pub include_external: bool,
    pub policy_decision_id: Option<String>,
    pub trace_context: Option<TraceContext>,
    pub timeout_ms: Option<u64>,
    pub metadata: Vec<(String, String)>,
}

impl KnowledgeSearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            session_id: None,
            task_id: None,
            run_id: None,
            step_id: None,
            tenant_id: None,
            namespace: None,
            top_k: 10,
            methods: Vec::new(),
            filters: Vec::new(),
            include_external: false,
            policy_decision_id: None,
            trace_context: None,
            timeout_ms: None,
            metadata: Vec::new(),
        }
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn for_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn for_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k.max(1);
        self
    }

    pub fn with_method(mut self, method: KnowledgeRetrievalMethod) -> Self {
        if !self.methods.contains(&method) {
            self.methods.push(method);
        }
        self
    }

    pub fn with_filter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.push((key.into(), value.into()));
        self
    }

    pub fn include_external(mut self) -> Self {
        self.include_external = true;
        self
    }

    pub fn with_policy_context(mut self, policy_decision_id: impl Into<String>) -> Self {
        self.policy_decision_id = Some(policy_decision_id.into());
        self
    }

    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn supports_method(&self, method: KnowledgeRetrievalMethod) -> bool {
        self.methods.contains(&method)
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeSearchResult {
    pub document_id: String,
    pub document_kind: KnowledgeDocumentKind,
    pub title: String,
    pub snippet: Option<String>,
    pub score: Option<f64>,
    pub retrieval_method: KnowledgeRetrievalMethod,
    pub source_uri: Option<String>,
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl KnowledgeSearchResult {
    pub fn new(
        document_id: impl Into<String>,
        document_kind: KnowledgeDocumentKind,
        title: impl Into<String>,
        retrieval_method: KnowledgeRetrievalMethod,
    ) -> Self {
        Self {
            document_id: document_id.into(),
            document_kind,
            title: title.into(),
            snippet: None,
            score: None,
            retrieval_method,
            source_uri: None,
            trust_level: TrustLevel::RetrievedExternal,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    pub fn with_source_uri(mut self, source_uri: impl Into<String>) -> Self {
        self.source_uri = Some(source_uri.into());
        self
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
        self
    }

    pub fn with_redaction_classification(
        mut self,
        redaction_classification: RedactionClassification,
    ) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDocument {
    pub document_id: String,
    pub kind: KnowledgeDocumentKind,
    pub namespace: Option<String>,
    pub title: String,
    pub content: String,
    pub content_type: String,
    pub source_uri: Option<String>,
    pub tags: Vec<String>,
    pub retrieval_methods: Vec<KnowledgeRetrievalMethod>,
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl KnowledgeDocument {
    pub fn new(
        document_id: impl Into<String>,
        kind: KnowledgeDocumentKind,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            document_id: document_id.into(),
            kind,
            namespace: None,
            title: title.into(),
            content: content.into(),
            content_type: "text/plain".to_string(),
            source_uri: None,
            tags: Vec::new(),
            retrieval_methods: Vec::new(),
            trust_level: TrustLevel::RetrievedExternal,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    pub fn with_source_uri(mut self, source_uri: impl Into<String>) -> Self {
        self.source_uri = Some(source_uri.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_retrieval_method(mut self, method: KnowledgeRetrievalMethod) -> Self {
        if !self.retrieval_methods.contains(&method) {
            self.retrieval_methods.push(method);
        }
        self
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
        self
    }

    pub fn with_redaction_classification(
        mut self,
        redaction_classification: RedactionClassification,
    ) -> Self {
        self.redaction_classification = redaction_classification;
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

    pub fn to_context_frame(&self, session_id: impl Into<String>) -> ContextFrame {
        let provenance = self
            .source_uri
            .clone()
            .unwrap_or_else(|| self.document_id.clone());

        let mut frame = ContextFrame::new(
            format!("context.knowledge.{}", self.document_id),
            session_id,
            "knowledge",
            self.content.clone(),
            self.trust_level,
            self.redaction_classification,
        )
        .with_content_type(self.content_type.clone())
        .with_provenance(provenance);

        for (key, value) in &self.metadata {
            frame = frame.with_metadata(key.clone(), value.clone());
        }

        frame
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDocumentFilter {
    pub kind: Option<KnowledgeDocumentKind>,
    pub namespace: Option<String>,
    pub tags: Vec<String>,
    pub include_external: bool,
}

impl KnowledgeDocumentFilter {
    pub fn new() -> Self {
        Self {
            kind: None,
            namespace: None,
            tags: Vec::new(),
            include_external: false,
        }
    }

    pub fn with_kind(mut self, kind: KnowledgeDocumentKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn include_external(mut self) -> Self {
        self.include_external = true;
        self
    }

    pub fn matches(&self, document: &KnowledgeDocument) -> bool {
        if let Some(kind) = self.kind {
            if document.kind != kind {
                return false;
            }
        }

        if let Some(namespace) = &self.namespace {
            if document.namespace.as_deref() != Some(namespace.as_str()) {
                return false;
            }
        }

        if !self.include_external && document.kind == KnowledgeDocumentKind::ExternalReference {
            return false;
        }

        self.tags
            .iter()
            .all(|tag| document.tags.iter().any(|document_tag| document_tag == tag))
    }
}

impl Default for KnowledgeDocumentFilter {
    fn default() -> Self {
        Self::new()
    }
}

pub trait KnowledgeProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.knowledge.unspecified",
            "knowledge",
            "knowledge-provider",
            "0.0.0",
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>>;

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument>;

    fn list(&self, filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>>;

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
