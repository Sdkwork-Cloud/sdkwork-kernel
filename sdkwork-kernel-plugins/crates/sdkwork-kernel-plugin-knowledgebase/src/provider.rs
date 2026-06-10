use crate::ids::SDKWORK_KNOWLEDGEBASE_PROVIDER_ID;
use crate::manifest::sdkwork_knowledgebase_provider_manifest;
use sdkwork_agent_kernel::{
    KernelError, KernelErrorSource, KernelResult, KnowledgeDocument, KnowledgeDocumentFilter,
    KnowledgeDocumentKind, KnowledgeProvider, KnowledgeRetrievalMethod, KnowledgeSearchRequest,
    KnowledgeSearchResult, ProviderHealth, ProviderManifest, RedactionClassification, TrustLevel,
};
use sdkwork_knowledgebase_contract::{
    KnowledgeContextFragment, KnowledgeFilter, KnowledgeRetrievalBinding,
    KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
};

pub trait KnowledgebaseRetrievalClient {
    fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, String>;

    fn read_document(&self, document_id: &str) -> Result<KnowledgeDocument, String>;

    fn list_documents(
        &self,
        filter: KnowledgeDocumentFilter,
    ) -> Result<Vec<KnowledgeDocument>, String>;
}

pub struct SdkworkKnowledgebaseProvider<C> {
    client: C,
    tenant_id: u64,
}

impl<C> SdkworkKnowledgebaseProvider<C> {
    pub fn new(client: C, tenant_id: u64) -> Self {
        Self { client, tenant_id }
    }
}

impl<C> KnowledgeProvider for SdkworkKnowledgebaseProvider<C>
where
    C: KnowledgebaseRetrievalClient,
{
    fn provider_manifest(&self) -> ProviderManifest {
        sdkwork_knowledgebase_provider_manifest()
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        let retrieval_request = self.to_retrieval_request(&request)?;
        let result = self.client.retrieve(retrieval_request).map_err(|message| {
            KernelError::provider_error("knowledgebase.retrieve_failed", message)
                .with_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
                .from_source(KernelErrorSource::Provider)
        })?;

        Ok(result.hits.into_iter().map(hit_to_search_result).collect())
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        self.client.read_document(document_id).map_err(|message| {
            KernelError::provider_error("knowledgebase.read_failed", message)
                .with_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
                .from_source(KernelErrorSource::Provider)
        })
    }

    fn list(&self, filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        self.client.list_documents(filter).map_err(|message| {
            KernelError::provider_error("knowledgebase.list_failed", message)
                .with_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
                .from_source(KernelErrorSource::Provider)
        })
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

impl<C> SdkworkKnowledgebaseProvider<C>
where
    C: KnowledgebaseRetrievalClient,
{
    fn to_retrieval_request(
        &self,
        request: &KnowledgeSearchRequest,
    ) -> KernelResult<KnowledgeRetrievalRequest> {
        if request.query.trim().is_empty() {
            return Err(KernelError::validation(
                "knowledge search query must not be blank",
            ));
        }

        Ok(KnowledgeRetrievalRequest {
            tenant_id: parse_tenant_id(request.tenant_id.as_deref()).unwrap_or(self.tenant_id),
            actor_id: None,
            query: request.query.clone(),
            retrieval_profile_id: request
                .metadata_value("sdkwork.knowledge.retrieval_profile_id")
                .and_then(|value| value.parse::<u64>().ok()),
            bindings: vec![KnowledgeRetrievalBinding {
                space_id: parse_namespace_space_id(request.namespace.as_deref())?,
                collection_id: request
                    .metadata_value("sdkwork.knowledge.collection_id")
                    .and_then(|value| value.parse::<u64>().ok()),
                source_filter: None,
                document_filter: None,
                priority: 0,
                top_k: Some(request.top_k.min(u32::MAX as usize) as u32),
                min_score: request
                    .metadata_value("sdkwork.knowledge.min_score")
                    .and_then(|value| value.parse::<f64>().ok()),
            }],
            methods: request.methods.iter().copied().map(map_method).collect(),
            top_k: Some(request.top_k.min(u32::MAX as usize) as u32),
            include_citations: true,
            include_trace: true,
            context_budget_tokens: request
                .metadata_value("sdkwork.knowledge.context_budget_tokens")
                .and_then(|value| value.parse::<u32>().ok()),
            metadata: request_filters(request),
        })
    }
}

fn request_filters(request: &KnowledgeSearchRequest) -> Vec<KnowledgeFilter> {
    request
        .filters
        .iter()
        .chain(request.metadata.iter())
        .map(|(key, value)| KnowledgeFilter {
            key: key.clone(),
            value: value.clone(),
        })
        .collect()
}

fn parse_tenant_id(value: Option<&str>) -> Option<u64> {
    value.and_then(|value| value.parse::<u64>().ok())
}

fn parse_namespace_space_id(value: Option<&str>) -> KernelResult<u64> {
    let Some(value) = value else {
        return Err(KernelError::validation(
            "knowledgebase provider requires namespace to contain a knowledge space id",
        ));
    };

    let value = value.strip_prefix("space:").unwrap_or(value);
    value
        .parse::<u64>()
        .map_err(|_| KernelError::validation("knowledge namespace must be a numeric space id"))
}

fn map_method(
    method: KnowledgeRetrievalMethod,
) -> sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod {
    match method {
        KnowledgeRetrievalMethod::Exact => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Exact
        }
        KnowledgeRetrievalMethod::Keyword => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Keyword
        }
        KnowledgeRetrievalMethod::FullText => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::FullText
        }
        KnowledgeRetrievalMethod::Structured => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Structured
        }
        KnowledgeRetrievalMethod::Graph => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Graph
        }
        KnowledgeRetrievalMethod::Vector => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Vector
        }
        KnowledgeRetrievalMethod::Hybrid => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid
        }
        KnowledgeRetrievalMethod::LlmRerank => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::LlmRerank
        }
        KnowledgeRetrievalMethod::External => {
            sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::External
        }
    }
}

fn hit_to_search_result(hit: KnowledgeContextFragment) -> KnowledgeSearchResult {
    let mut result = KnowledgeSearchResult::new(
        hit.document_id.to_string(),
        KnowledgeDocumentKind::Other,
        hit.title,
        map_contract_method(hit.retrieval_method),
    )
    .with_snippet(hit.content)
    .with_trust_level(TrustLevel::TrustedHost)
    .with_redaction_classification(RedactionClassification::TenantSensitive)
    .with_metadata("sdkwork.knowledge.chunk_id", hit.chunk_id.to_string())
    .with_metadata("sdkwork.knowledge.document_id", hit.document_id.to_string())
    .with_metadata("sdkwork.knowledge.space_id", hit.space_id.to_string())
    .with_metadata("sdkwork.knowledge.rank", hit.rank.to_string());

    if let Some(score) = hit.score {
        result = result.with_score(score);
    }

    if let Some(document_version_id) = hit.document_version_id {
        result = result.with_metadata(
            "sdkwork.knowledge.document_version_id",
            document_version_id.to_string(),
        );
    }

    if let Some(collection_id) = hit.collection_id {
        result = result.with_metadata("sdkwork.knowledge.collection_id", collection_id.to_string());
    }

    if let Some(token_count) = hit.token_count {
        result = result.with_metadata("sdkwork.knowledge.token_count", token_count.to_string());
    }

    if let Some(citation) = hit.citation {
        if let Some(source_uri) = citation.source_uri {
            result = result.with_source_uri(source_uri);
        }
        if let Some(locator) = citation.locator {
            result = result.with_metadata("sdkwork.knowledge.citation.locator", locator);
        }
    }

    result
}

fn map_contract_method(
    method: sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod,
) -> KnowledgeRetrievalMethod {
    match method {
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Exact => {
            KnowledgeRetrievalMethod::Exact
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Keyword => {
            KnowledgeRetrievalMethod::Keyword
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::FullText => {
            KnowledgeRetrievalMethod::FullText
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Structured => {
            KnowledgeRetrievalMethod::Structured
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Graph => {
            KnowledgeRetrievalMethod::Graph
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Vector => {
            KnowledgeRetrievalMethod::Vector
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid => {
            KnowledgeRetrievalMethod::Hybrid
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::LlmRerank => {
            KnowledgeRetrievalMethod::LlmRerank
        }
        sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::External => {
            KnowledgeRetrievalMethod::External
        }
    }
}
