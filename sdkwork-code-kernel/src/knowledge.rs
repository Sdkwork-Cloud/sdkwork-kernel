use crate::Workspace;
use sdkwork_agent_kernel::{KernelEventRedaction, KernelResult, ProviderHealth};

pub trait KnowledgeProvider {
    fn search_documents(
        &self,
        workspace: &Workspace,
        query: KnowledgeQuery,
    ) -> KernelResult<Vec<KnowledgeSearchResult>>;

    fn get_document(
        &self,
        workspace: &Workspace,
        document_id: &str,
    ) -> KernelResult<KnowledgeDocument>;

    fn list_documents(
        &self,
        workspace: &Workspace,
        filter: KnowledgeDocumentFilter,
    ) -> KernelResult<Vec<KnowledgeDocument>>;

    fn health(&self) -> ProviderHealth;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeDocumentKind {
    Readme,
    Spec,
    Adr,
    GeneratedContract,
    ExternalReference,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDocument {
    pub document_id: String,
    pub kind: KnowledgeDocumentKind,
    pub path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub redaction_classification: KernelEventRedaction,
}

impl KnowledgeDocument {
    pub fn new(
        document_id: impl Into<String>,
        kind: KnowledgeDocumentKind,
        path: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            document_id: document_id.into(),
            kind,
            path: path.into(),
            title: title.into(),
            content: content.into(),
            tags: Vec::new(),
            redaction_classification: KernelEventRedaction::Unknown,
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_redaction(mut self, redaction_classification: KernelEventRedaction) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeQuery {
    pub query: String,
    pub kinds: Vec<KnowledgeDocumentKind>,
    pub paths: Vec<String>,
    pub include_external: bool,
}

impl KnowledgeQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            kinds: Vec::new(),
            paths: Vec::new(),
            include_external: false,
        }
    }

    pub fn with_kind(mut self, kind: KnowledgeDocumentKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    pub fn include_external(mut self) -> Self {
        self.include_external = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeSearchResult {
    pub document_id: String,
    pub kind: KnowledgeDocumentKind,
    pub score: f32,
    pub title: String,
}

impl KnowledgeSearchResult {
    pub fn new(
        document_id: impl Into<String>,
        kind: KnowledgeDocumentKind,
        score: f32,
        title: impl Into<String>,
    ) -> Self {
        Self {
            document_id: document_id.into(),
            kind,
            score,
            title: title.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDocumentFilter {
    pub kind: Option<KnowledgeDocumentKind>,
    pub tags: Vec<String>,
    pub include_external: bool,
}

impl KnowledgeDocumentFilter {
    pub fn new() -> Self {
        Self {
            kind: None,
            tags: Vec::new(),
            include_external: false,
        }
    }

    pub fn with_kind(mut self, kind: KnowledgeDocumentKind) -> Self {
        self.kind = Some(kind);
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
