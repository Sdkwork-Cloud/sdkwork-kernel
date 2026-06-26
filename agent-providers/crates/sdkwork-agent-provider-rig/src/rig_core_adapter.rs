use sdkwork_agent_kernel::{KnowledgeDocument, KnowledgeSearchRequest};

use crate::provider::RigKnowledgeProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigVectorSearchPlan {
    pub query: String,
    pub samples: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RigCoreKnowledgeAdapter;

impl RigCoreKnowledgeAdapter {
    pub fn vector_search_plan(request: &KnowledgeSearchRequest) -> RigVectorSearchPlan {
        RigVectorSearchPlan {
            query: request.query.clone(),
            samples: request.top_k as u64,
        }
    }

    pub fn provider_from_documents(
        documents: impl IntoIterator<Item = KnowledgeDocument>,
    ) -> RigKnowledgeProvider {
        documents
            .into_iter()
            .fold(RigKnowledgeProvider::new(), |provider, document| {
                provider.with_document(document)
            })
    }
}
