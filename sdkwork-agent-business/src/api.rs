#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiOperation {
    pub method: &'static str,
    pub path: &'static str,
    pub tag: &'static str,
    pub operation_id: &'static str,
}

pub const AGENT_OPEN_API_PREFIX: &str = "/agent/v3/api";
pub const AGENT_APP_API_PREFIX: &str = "/app/v3/api";
pub const AGENT_BACKEND_API_PREFIX: &str = "/backend/v3/api";

pub const AGENT_OPEN_API_OPERATIONS: &[ApiOperation] = &[
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/agent/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/agent/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents/{agentId}/restore",
        tag: "ai",
        operation_id: "agents.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.create",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
        tag: "ai",
        operation_id: "agents.providerBindings.activate",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.create",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents/{agentId}/preview_responses",
        tag: "ai",
        operation_id: "agents.previewResponses.create",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/agents/{agentId}/prompt_optimizations",
        tag: "ai",
        operation_id: "agents.promptOptimizations.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bases",
        tag: "ai",
        operation_id: "knowledgeBases.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases",
        tag: "ai",
        operation_id: "knowledgeBases.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/restore",
        tag: "ai",
        operation_id: "knowledgeBases.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sources",
        tag: "ai",
        operation_id: "knowledgeSources.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sources",
        tag: "ai",
        operation_id: "knowledgeSources.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/agent/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/agent/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_sources/{knowledgeSourceId}/restore",
        tag: "ai",
        operation_id: "knowledgeSources.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/documents",
        tag: "ai",
        operation_id: "knowledgeList.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/documents",
        tag: "ai",
        operation_id: "knowledgeDocuments.create",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/search",
        tag: "ai",
        operation_id: "knowledgeSearch.search",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeRead.read",
    },
    ApiOperation {
        method: "PATCH",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeDocuments.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeDocuments.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/restore",
        tag: "ai",
        operation_id: "knowledgeDocuments.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
        tag: "ai",
        operation_id: "knowledgeChunks.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
        tag: "ai",
        operation_id: "knowledgeChunks.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_chunks/{knowledgeChunkId}",
        tag: "ai",
        operation_id: "knowledgeChunks.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/indexes",
        tag: "ai",
        operation_id: "knowledgeIndexes.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_indexes",
        tag: "ai",
        operation_id: "knowledgeIndexes.upsert",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_indexes/{knowledgeIndexId}",
        tag: "ai",
        operation_id: "knowledgeIndexes.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/bindings",
        tag: "ai",
        operation_id: "knowledgeBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/bindings",
        tag: "ai",
        operation_id: "knowledgeBindings.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bindings/{knowledgeBindingId}",
        tag: "ai",
        operation_id: "knowledgeBindings.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/knowledge_sync_jobs/{syncJobId}",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_sync_jobs/{syncJobId}/start",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.start",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_sync_jobs/{syncJobId}/complete",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.complete",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_sync_jobs/{syncJobId}/fail",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.fail",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/knowledge_sync_jobs/{syncJobId}/cancel",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.cancel",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_stores",
        tag: "ai",
        operation_id: "memoryStores.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_stores/{memoryStoreId}",
        tag: "ai",
        operation_id: "memoryStores.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/agent/v3/api/ai/memory_stores/{memoryStoreId}",
        tag: "ai",
        operation_id: "memoryStores.update",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_stores/{memoryStoreId}/profiles",
        tag: "ai",
        operation_id: "memoryProfiles.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_profiles/{memoryProfileId}",
        tag: "ai",
        operation_id: "memoryProfiles.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_profiles/{memoryProfileId}/bindings",
        tag: "ai",
        operation_id: "memoryBindings.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_bindings/{memoryBindingId}",
        tag: "ai",
        operation_id: "memoryBindings.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_namespaces",
        tag: "ai",
        operation_id: "memoryNamespaces.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_namespaces/{memoryNamespaceId}",
        tag: "ai",
        operation_id: "memoryNamespaces.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_namespaces/{memoryNamespaceId}/records",
        tag: "ai",
        operation_id: "memoryRecords.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_namespaces/{memoryNamespaceId}/records",
        tag: "ai",
        operation_id: "memoryRecords.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_records/{memoryId}",
        tag: "ai",
        operation_id: "memoryRecords.retrieve",
    },
    ApiOperation {
        method: "DELETE",
        path: "/agent/v3/api/ai/memory_records/{memoryId}",
        tag: "ai",
        operation_id: "memoryRecords.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_records/{memoryId}/restore",
        tag: "ai",
        operation_id: "memoryRecords.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_records/{memoryId}/sources",
        tag: "ai",
        operation_id: "memorySources.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_records/{memoryId}/sources",
        tag: "ai",
        operation_id: "memorySources.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_records/{memoryId}/relations",
        tag: "ai",
        operation_id: "memoryRelations.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_records/{memoryId}/relations",
        tag: "ai",
        operation_id: "memoryRelations.create",
    },
    ApiOperation {
        method: "GET",
        path: "/agent/v3/api/ai/memory_records/{memoryId}/retrieval_indexes",
        tag: "ai",
        operation_id: "memoryRetrievalIndexes.list",
    },
    ApiOperation {
        method: "POST",
        path: "/agent/v3/api/ai/memory_retrieval_indexes",
        tag: "ai",
        operation_id: "memoryRetrievalIndexes.upsert",
    },
];

pub const AGENT_APP_API_OPERATIONS: &[ApiOperation] = &[
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/app/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/app/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/restore",
        tag: "ai",
        operation_id: "agents.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.create",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
        tag: "ai",
        operation_id: "agents.providerBindings.activate",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.create",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/preview_responses",
        tag: "ai",
        operation_id: "agents.previewResponses.create",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/agents/{agentId}/prompt_optimizations",
        tag: "ai",
        operation_id: "agents.promptOptimizations.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bases",
        tag: "ai",
        operation_id: "knowledgeBases.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases",
        tag: "ai",
        operation_id: "knowledgeBases.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/restore",
        tag: "ai",
        operation_id: "knowledgeBases.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sources",
        tag: "ai",
        operation_id: "knowledgeSources.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sources",
        tag: "ai",
        operation_id: "knowledgeSources.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/app/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/app/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_sources/{knowledgeSourceId}/restore",
        tag: "ai",
        operation_id: "knowledgeSources.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/documents",
        tag: "ai",
        operation_id: "knowledgeList.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/documents",
        tag: "ai",
        operation_id: "knowledgeDocuments.create",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/search",
        tag: "ai",
        operation_id: "knowledgeSearch.search",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeRead.read",
    },
    ApiOperation {
        method: "PATCH",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeDocuments.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeDocuments.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/restore",
        tag: "ai",
        operation_id: "knowledgeDocuments.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
        tag: "ai",
        operation_id: "knowledgeChunks.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
        tag: "ai",
        operation_id: "knowledgeChunks.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_chunks/{knowledgeChunkId}",
        tag: "ai",
        operation_id: "knowledgeChunks.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/indexes",
        tag: "ai",
        operation_id: "knowledgeIndexes.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_indexes",
        tag: "ai",
        operation_id: "knowledgeIndexes.upsert",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_indexes/{knowledgeIndexId}",
        tag: "ai",
        operation_id: "knowledgeIndexes.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/bindings",
        tag: "ai",
        operation_id: "knowledgeBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/bindings",
        tag: "ai",
        operation_id: "knowledgeBindings.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bindings/{knowledgeBindingId}",
        tag: "ai",
        operation_id: "knowledgeBindings.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/knowledge_sync_jobs/{syncJobId}",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_sync_jobs/{syncJobId}/start",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.start",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_sync_jobs/{syncJobId}/complete",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.complete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_sync_jobs/{syncJobId}/fail",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.fail",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/knowledge_sync_jobs/{syncJobId}/cancel",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.cancel",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_stores",
        tag: "ai",
        operation_id: "memoryStores.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_stores/{memoryStoreId}",
        tag: "ai",
        operation_id: "memoryStores.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/app/v3/api/ai/memory_stores/{memoryStoreId}",
        tag: "ai",
        operation_id: "memoryStores.update",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_stores/{memoryStoreId}/profiles",
        tag: "ai",
        operation_id: "memoryProfiles.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_profiles/{memoryProfileId}",
        tag: "ai",
        operation_id: "memoryProfiles.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_profiles/{memoryProfileId}/bindings",
        tag: "ai",
        operation_id: "memoryBindings.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_bindings/{memoryBindingId}",
        tag: "ai",
        operation_id: "memoryBindings.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_namespaces",
        tag: "ai",
        operation_id: "memoryNamespaces.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_namespaces/{memoryNamespaceId}",
        tag: "ai",
        operation_id: "memoryNamespaces.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_namespaces/{memoryNamespaceId}/records",
        tag: "ai",
        operation_id: "memoryRecords.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_namespaces/{memoryNamespaceId}/records",
        tag: "ai",
        operation_id: "memoryRecords.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_records/{memoryId}",
        tag: "ai",
        operation_id: "memoryRecords.retrieve",
    },
    ApiOperation {
        method: "DELETE",
        path: "/app/v3/api/ai/memory_records/{memoryId}",
        tag: "ai",
        operation_id: "memoryRecords.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_records/{memoryId}/restore",
        tag: "ai",
        operation_id: "memoryRecords.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_records/{memoryId}/sources",
        tag: "ai",
        operation_id: "memorySources.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_records/{memoryId}/sources",
        tag: "ai",
        operation_id: "memorySources.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_records/{memoryId}/relations",
        tag: "ai",
        operation_id: "memoryRelations.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_records/{memoryId}/relations",
        tag: "ai",
        operation_id: "memoryRelations.create",
    },
    ApiOperation {
        method: "GET",
        path: "/app/v3/api/ai/memory_records/{memoryId}/retrieval_indexes",
        tag: "ai",
        operation_id: "memoryRetrievalIndexes.list",
    },
    ApiOperation {
        method: "POST",
        path: "/app/v3/api/ai/memory_retrieval_indexes",
        tag: "ai",
        operation_id: "memoryRetrievalIndexes.upsert",
    },
];

pub const AGENT_BACKEND_API_OPERATIONS: &[ApiOperation] = &[
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents",
        tag: "ai",
        operation_id: "agents.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/backend/v3/api/ai/agents/{agentId}",
        tag: "ai",
        operation_id: "agents.update",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/status",
        tag: "ai",
        operation_id: "agents.status.update",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/restore",
        tag: "ai",
        operation_id: "agents.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}/audit_events",
        tag: "ai",
        operation_id: "agents.auditEvents.list",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
        tag: "ai",
        operation_id: "agents.providerBindings.create",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
        tag: "ai",
        operation_id: "agents.providerBindings.activate",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/agents/{agentId}/deployments",
        tag: "ai",
        operation_id: "agents.deployments.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bases",
        tag: "ai",
        operation_id: "knowledgeBases.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases",
        tag: "ai",
        operation_id: "knowledgeBases.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}",
        tag: "ai",
        operation_id: "knowledgeBases.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/restore",
        tag: "ai",
        operation_id: "knowledgeBases.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sources",
        tag: "ai",
        operation_id: "knowledgeSources.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sources",
        tag: "ai",
        operation_id: "knowledgeSources.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/backend/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/backend/v3/api/ai/knowledge_sources/{knowledgeSourceId}",
        tag: "ai",
        operation_id: "knowledgeSources.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_sources/{knowledgeSourceId}/restore",
        tag: "ai",
        operation_id: "knowledgeSources.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/documents",
        tag: "ai",
        operation_id: "knowledgeList.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/documents",
        tag: "ai",
        operation_id: "knowledgeDocuments.create",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/search",
        tag: "ai",
        operation_id: "knowledgeSearch.search",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeRead.read",
    },
    ApiOperation {
        method: "PATCH",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeDocuments.update",
    },
    ApiOperation {
        method: "DELETE",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}",
        tag: "ai",
        operation_id: "knowledgeDocuments.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/restore",
        tag: "ai",
        operation_id: "knowledgeDocuments.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
        tag: "ai",
        operation_id: "knowledgeChunks.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
        tag: "ai",
        operation_id: "knowledgeChunks.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_chunks/{knowledgeChunkId}",
        tag: "ai",
        operation_id: "knowledgeChunks.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_documents/{knowledgeDocumentId}/indexes",
        tag: "ai",
        operation_id: "knowledgeIndexes.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_indexes",
        tag: "ai",
        operation_id: "knowledgeIndexes.upsert",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_indexes/{knowledgeIndexId}",
        tag: "ai",
        operation_id: "knowledgeIndexes.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/bindings",
        tag: "ai",
        operation_id: "knowledgeBindings.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/bindings",
        tag: "ai",
        operation_id: "knowledgeBindings.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bindings/{knowledgeBindingId}",
        tag: "ai",
        operation_id: "knowledgeBindings.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/knowledge_sync_jobs/{syncJobId}",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_sync_jobs/{syncJobId}/start",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.start",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_sync_jobs/{syncJobId}/complete",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.complete",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_sync_jobs/{syncJobId}/fail",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.fail",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/knowledge_sync_jobs/{syncJobId}/cancel",
        tag: "ai",
        operation_id: "knowledgeSyncJobs.cancel",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_stores",
        tag: "ai",
        operation_id: "memoryStores.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_stores/{memoryStoreId}",
        tag: "ai",
        operation_id: "memoryStores.retrieve",
    },
    ApiOperation {
        method: "PATCH",
        path: "/backend/v3/api/ai/memory_stores/{memoryStoreId}",
        tag: "ai",
        operation_id: "memoryStores.update",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_stores/{memoryStoreId}/profiles",
        tag: "ai",
        operation_id: "memoryProfiles.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_profiles/{memoryProfileId}",
        tag: "ai",
        operation_id: "memoryProfiles.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_profiles/{memoryProfileId}/bindings",
        tag: "ai",
        operation_id: "memoryBindings.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_bindings/{memoryBindingId}",
        tag: "ai",
        operation_id: "memoryBindings.retrieve",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_namespaces",
        tag: "ai",
        operation_id: "memoryNamespaces.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_namespaces/{memoryNamespaceId}",
        tag: "ai",
        operation_id: "memoryNamespaces.retrieve",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_namespaces/{memoryNamespaceId}/records",
        tag: "ai",
        operation_id: "memoryRecords.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_namespaces/{memoryNamespaceId}/records",
        tag: "ai",
        operation_id: "memoryRecords.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_records/{memoryId}",
        tag: "ai",
        operation_id: "memoryRecords.retrieve",
    },
    ApiOperation {
        method: "DELETE",
        path: "/backend/v3/api/ai/memory_records/{memoryId}",
        tag: "ai",
        operation_id: "memoryRecords.delete",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_records/{memoryId}/restore",
        tag: "ai",
        operation_id: "memoryRecords.restore",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_records/{memoryId}/sources",
        tag: "ai",
        operation_id: "memorySources.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_records/{memoryId}/sources",
        tag: "ai",
        operation_id: "memorySources.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_records/{memoryId}/relations",
        tag: "ai",
        operation_id: "memoryRelations.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_records/{memoryId}/relations",
        tag: "ai",
        operation_id: "memoryRelations.create",
    },
    ApiOperation {
        method: "GET",
        path: "/backend/v3/api/ai/memory_records/{memoryId}/retrieval_indexes",
        tag: "ai",
        operation_id: "memoryRetrievalIndexes.list",
    },
    ApiOperation {
        method: "POST",
        path: "/backend/v3/api/ai/memory_retrieval_indexes",
        tag: "ai",
        operation_id: "memoryRetrievalIndexes.upsert",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_binding_and_deployment_operations_are_registered() {
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "GET",
            "/agent/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.list",
        );
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "POST",
            "/agent/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.create",
        );
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "POST",
            "/agent/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
            "agents.providerBindings.activate",
        );
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "GET",
            "/agent/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.list",
        );
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "POST",
            "/agent/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.create",
        );
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "POST",
            "/agent/v3/api/ai/agents/{agentId}/preview_responses",
            "agents.previewResponses.create",
        );
        assert_operation(
            AGENT_OPEN_API_OPERATIONS,
            "POST",
            "/agent/v3/api/ai/agents/{agentId}/prompt_optimizations",
            "agents.promptOptimizations.create",
        );

        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "GET",
            "/app/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.list",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.create",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
            "agents.providerBindings.activate",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "GET",
            "/app/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.list",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.create",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/preview_responses",
            "agents.previewResponses.create",
        );
        assert_operation(
            AGENT_APP_API_OPERATIONS,
            "POST",
            "/app/v3/api/ai/agents/{agentId}/prompt_optimizations",
            "agents.promptOptimizations.create",
        );

        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "GET",
            "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.list",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "POST",
            "/backend/v3/api/ai/agents/{agentId}/provider_bindings",
            "agents.providerBindings.create",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "POST",
            "/backend/v3/api/ai/agents/{agentId}/provider_bindings/{bindingId}/activate",
            "agents.providerBindings.activate",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "GET",
            "/backend/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.list",
        );
        assert_operation(
            AGENT_BACKEND_API_OPERATIONS,
            "POST",
            "/backend/v3/api/ai/agents/{agentId}/deployments",
            "agents.deployments.create",
        );
    }

    #[test]
    fn knowledge_rag_operations_are_registered_for_all_api_boundaries() {
        for (operations, prefix) in [
            (AGENT_OPEN_API_OPERATIONS, AGENT_OPEN_API_PREFIX),
            (AGENT_APP_API_OPERATIONS, AGENT_APP_API_PREFIX),
            (AGENT_BACKEND_API_OPERATIONS, AGENT_BACKEND_API_PREFIX),
        ] {
            for (method, path_suffix, operation_id) in [
                ("GET", "/ai/knowledge_bases", "knowledgeBases.list"),
                ("POST", "/ai/knowledge_bases", "knowledgeBases.create"),
                (
                    "GET",
                    "/ai/knowledge_bases/{knowledgeBaseId}",
                    "knowledgeBases.retrieve",
                ),
                (
                    "PATCH",
                    "/ai/knowledge_bases/{knowledgeBaseId}",
                    "knowledgeBases.update",
                ),
                (
                    "DELETE",
                    "/ai/knowledge_bases/{knowledgeBaseId}",
                    "knowledgeBases.delete",
                ),
                (
                    "POST",
                    "/ai/knowledge_bases/{knowledgeBaseId}/restore",
                    "knowledgeBases.restore",
                ),
                (
                    "GET",
                    "/ai/knowledge_bases/{knowledgeBaseId}/sources",
                    "knowledgeSources.list",
                ),
                (
                    "POST",
                    "/ai/knowledge_bases/{knowledgeBaseId}/sources",
                    "knowledgeSources.create",
                ),
                (
                    "GET",
                    "/ai/knowledge_sources/{knowledgeSourceId}",
                    "knowledgeSources.retrieve",
                ),
                (
                    "PATCH",
                    "/ai/knowledge_sources/{knowledgeSourceId}",
                    "knowledgeSources.update",
                ),
                (
                    "DELETE",
                    "/ai/knowledge_sources/{knowledgeSourceId}",
                    "knowledgeSources.delete",
                ),
                (
                    "POST",
                    "/ai/knowledge_sources/{knowledgeSourceId}/restore",
                    "knowledgeSources.restore",
                ),
                (
                    "GET",
                    "/ai/knowledge_bases/{knowledgeBaseId}/documents",
                    "knowledgeList.list",
                ),
                (
                    "POST",
                    "/ai/knowledge_bases/{knowledgeBaseId}/documents",
                    "knowledgeDocuments.create",
                ),
                (
                    "POST",
                    "/ai/knowledge_bases/{knowledgeBaseId}/search",
                    "knowledgeSearch.search",
                ),
                (
                    "GET",
                    "/ai/knowledge_documents/{knowledgeDocumentId}",
                    "knowledgeRead.read",
                ),
                (
                    "PATCH",
                    "/ai/knowledge_documents/{knowledgeDocumentId}",
                    "knowledgeDocuments.update",
                ),
                (
                    "DELETE",
                    "/ai/knowledge_documents/{knowledgeDocumentId}",
                    "knowledgeDocuments.delete",
                ),
                (
                    "POST",
                    "/ai/knowledge_documents/{knowledgeDocumentId}/restore",
                    "knowledgeDocuments.restore",
                ),
                (
                    "GET",
                    "/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
                    "knowledgeChunks.list",
                ),
                (
                    "POST",
                    "/ai/knowledge_documents/{knowledgeDocumentId}/chunks",
                    "knowledgeChunks.create",
                ),
                (
                    "GET",
                    "/ai/knowledge_chunks/{knowledgeChunkId}",
                    "knowledgeChunks.retrieve",
                ),
                (
                    "GET",
                    "/ai/knowledge_documents/{knowledgeDocumentId}/indexes",
                    "knowledgeIndexes.list",
                ),
                ("POST", "/ai/knowledge_indexes", "knowledgeIndexes.upsert"),
                (
                    "GET",
                    "/ai/knowledge_indexes/{knowledgeIndexId}",
                    "knowledgeIndexes.retrieve",
                ),
                (
                    "GET",
                    "/ai/knowledge_bases/{knowledgeBaseId}/bindings",
                    "knowledgeBindings.list",
                ),
                (
                    "POST",
                    "/ai/knowledge_bases/{knowledgeBaseId}/bindings",
                    "knowledgeBindings.create",
                ),
                (
                    "GET",
                    "/ai/knowledge_bindings/{knowledgeBindingId}",
                    "knowledgeBindings.retrieve",
                ),
                (
                    "GET",
                    "/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
                    "knowledgeSyncJobs.list",
                ),
                (
                    "POST",
                    "/ai/knowledge_bases/{knowledgeBaseId}/sync_jobs",
                    "knowledgeSyncJobs.create",
                ),
                (
                    "GET",
                    "/ai/knowledge_sync_jobs/{syncJobId}",
                    "knowledgeSyncJobs.retrieve",
                ),
                (
                    "POST",
                    "/ai/knowledge_sync_jobs/{syncJobId}/start",
                    "knowledgeSyncJobs.start",
                ),
                (
                    "POST",
                    "/ai/knowledge_sync_jobs/{syncJobId}/complete",
                    "knowledgeSyncJobs.complete",
                ),
                (
                    "POST",
                    "/ai/knowledge_sync_jobs/{syncJobId}/fail",
                    "knowledgeSyncJobs.fail",
                ),
                (
                    "POST",
                    "/ai/knowledge_sync_jobs/{syncJobId}/cancel",
                    "knowledgeSyncJobs.cancel",
                ),
            ] {
                assert_operation(
                    operations,
                    method,
                    format!("{prefix}{path_suffix}").as_str(),
                    operation_id,
                );
            }
        }
    }

    #[test]
    fn memory_rag_operations_are_registered_for_all_api_boundaries() {
        for (operations, prefix) in [
            (AGENT_OPEN_API_OPERATIONS, AGENT_OPEN_API_PREFIX),
            (AGENT_APP_API_OPERATIONS, AGENT_APP_API_PREFIX),
            (AGENT_BACKEND_API_OPERATIONS, AGENT_BACKEND_API_PREFIX),
        ] {
            for (method, path_suffix, operation_id) in [
                ("POST", "/ai/memory_stores", "memoryStores.create"),
                (
                    "GET",
                    "/ai/memory_stores/{memoryStoreId}",
                    "memoryStores.retrieve",
                ),
                (
                    "PATCH",
                    "/ai/memory_stores/{memoryStoreId}",
                    "memoryStores.update",
                ),
                (
                    "POST",
                    "/ai/memory_stores/{memoryStoreId}/profiles",
                    "memoryProfiles.create",
                ),
                (
                    "GET",
                    "/ai/memory_profiles/{memoryProfileId}",
                    "memoryProfiles.retrieve",
                ),
                (
                    "POST",
                    "/ai/memory_profiles/{memoryProfileId}/bindings",
                    "memoryBindings.create",
                ),
                (
                    "GET",
                    "/ai/memory_bindings/{memoryBindingId}",
                    "memoryBindings.retrieve",
                ),
                ("POST", "/ai/memory_namespaces", "memoryNamespaces.create"),
                (
                    "GET",
                    "/ai/memory_namespaces/{memoryNamespaceId}",
                    "memoryNamespaces.retrieve",
                ),
                (
                    "GET",
                    "/ai/memory_namespaces/{memoryNamespaceId}/records",
                    "memoryRecords.list",
                ),
                (
                    "POST",
                    "/ai/memory_namespaces/{memoryNamespaceId}/records",
                    "memoryRecords.create",
                ),
                (
                    "GET",
                    "/ai/memory_records/{memoryId}",
                    "memoryRecords.retrieve",
                ),
                (
                    "DELETE",
                    "/ai/memory_records/{memoryId}",
                    "memoryRecords.delete",
                ),
                (
                    "POST",
                    "/ai/memory_records/{memoryId}/restore",
                    "memoryRecords.restore",
                ),
                (
                    "GET",
                    "/ai/memory_records/{memoryId}/sources",
                    "memorySources.list",
                ),
                (
                    "POST",
                    "/ai/memory_records/{memoryId}/sources",
                    "memorySources.create",
                ),
                (
                    "GET",
                    "/ai/memory_records/{memoryId}/relations",
                    "memoryRelations.list",
                ),
                (
                    "POST",
                    "/ai/memory_records/{memoryId}/relations",
                    "memoryRelations.create",
                ),
                (
                    "GET",
                    "/ai/memory_records/{memoryId}/retrieval_indexes",
                    "memoryRetrievalIndexes.list",
                ),
                (
                    "POST",
                    "/ai/memory_retrieval_indexes",
                    "memoryRetrievalIndexes.upsert",
                ),
            ] {
                assert_operation(
                    operations,
                    method,
                    format!("{prefix}{path_suffix}").as_str(),
                    operation_id,
                );
            }
        }
    }

    #[test]
    fn openapi_specs_expose_provider_binding_and_deployment_contracts() {
        let open_openapi = include_str!("../specs/openapi/agent-business-open-openapi-3.1.2.yaml");
        let app_openapi = include_str!("../specs/openapi/agent-business-app-openapi-3.1.2.yaml");
        let backend_openapi =
            include_str!("../specs/openapi/agent-business-backend-openapi-3.1.2.yaml");

        for (label, openapi, prefix) in [
            ("open", open_openapi, "/agent/v3/api"),
            ("app", app_openapi, "/app/v3/api"),
            ("backend", backend_openapi, "/backend/v3/api"),
        ] {
            for required in [
                format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                format!("{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"),
                format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "- $ref: '#/components/parameters/Page'".to_string(),
                "- $ref: '#/components/parameters/PageSize'".to_string(),
                "operationId: agents.providerBindings.list".to_string(),
                "operationId: agents.providerBindings.create".to_string(),
                "operationId: agents.providerBindings.activate".to_string(),
                "operationId: agents.deployments.list".to_string(),
                "operationId: agents.deployments.create".to_string(),
                "AgentImplementationKind:".to_string(),
                "DeploymentStatus:".to_string(),
                "enum: [created, active, failed, archived]".to_string(),
                "AgentProviderBindingRecord:".to_string(),
                "AgentProviderBindingResponse:".to_string(),
                "AgentProviderBindingListResponse:".to_string(),
                "CreateAgentProviderBindingRequest:".to_string(),
                "ActivateAgentProviderBindingRequest:".to_string(),
                "AgentDeploymentRecord:".to_string(),
                "AgentDeploymentResponse:".to_string(),
                "AgentDeploymentListResponse:".to_string(),
                "CreateAgentDeploymentRequest:".to_string(),
                "AgentImplementationType:".to_string(),
                "enum: [sdkwork-native, rig-rust, openai-agents, langchain, langgraph, crewai, autogen, semantic-kernel, custom]".to_string(),
                "implementationProviderId:".to_string(),
                "implementationKind:".to_string(),
                "implementationType:".to_string(),
                "required: [items, pageInfo]".to_string(),
                "pattern: '^binding\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^profile\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^deployment\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^[a-z0-9_-]+(\\.[a-z0-9_-]+)+$'".to_string(),
                "uniqueItems: true".to_string(),
            ] {
                assert!(
                    openapi.contains(required.as_str()),
                    "{label} OpenAPI must contain {required}"
                );
            }

            if label != "backend" {
                for required in [
                    format!("{prefix}/ai/agents/{{agentId}}/preview_responses:"),
                    format!("{prefix}/ai/agents/{{agentId}}/prompt_optimizations:"),
                    "operationId: agents.previewResponses.create".to_string(),
                    "operationId: agents.promptOptimizations.create".to_string(),
                    "CreateAgentPreviewResponseRequest:".to_string(),
                    "CreateAgentPromptOptimizationRequest:".to_string(),
                    "AgentRuntimeExecutionRecord:".to_string(),
                    "AgentRuntimeExecutionResponse:".to_string(),
                ] {
                    assert!(
                        openapi.contains(required.as_str()),
                        "{label} OpenAPI must contain {required}"
                    );
                }
            }

            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                "get:",
                "post:",
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                "post:",
                &format!(
                    "{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"
                ),
                &["'400':", "'403':", "'404':", "'409':"],
            );
            assert_operation_block_excludes(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/provider_bindings:"),
                "post:",
                &format!(
                    "{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"
                ),
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!(
                    "{prefix}/ai/agents/{{agentId}}/provider_bindings/{{bindingId}}/activate:"
                ),
                "post:",
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                &["'400':", "'403':", "'404':"],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "get:",
                "post:",
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "post:",
                deployment_post_block_end(label, prefix).as_str(),
                &["'400':", "'403':", "'404':", "'409':"],
            );
            assert_operation_block_excludes(
                label,
                openapi,
                &format!("{prefix}/ai/agents/{{agentId}}/deployments:"),
                "post:",
                deployment_post_block_end(label, prefix).as_str(),
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentRecord:",
                "agentId:",
                "tenantId:",
                &["pattern: '^agent\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentProviderBindingRecord:",
                "agentId:",
                "bindingId:",
                &["pattern: '^agent\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentDeploymentRecord:",
                "agentId:",
                "deploymentId:",
                &["pattern: '^agent\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            let create_agent_agent_id_until = if label == "backend" {
                "organizationId:"
            } else {
                "code:"
            };
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateAgentRequest:",
                "agentId:",
                create_agent_agent_id_until,
                &["pattern: '^agent\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            if label != "backend" {
                assert_schema_block_excludes(
                    label,
                    openapi,
                    "CreateAgentRequest:",
                    "UpdateAgentRequest:",
                    &["organizationId:", "ownerUserId:"],
                );
            }
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentRecord:",
                "implementationProviderId:",
                "implementationKind:",
                &["pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentRecord:",
                "implementationKind:",
                "status:",
                &[
                    "implementationType:",
                    "$ref: '#/components/schemas/AgentImplementationType'",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateAgentRequest:",
                "implementationProviderId:",
                "implementationKind:",
                &["pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateAgentRequest:",
                "implementationKind:",
                "visibility:",
                &[
                    "implementationType:",
                    "$ref: '#/components/schemas/AgentImplementationType'",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateAgentRequest:",
                "managementProfile:",
                "implementationProviderId:",
                &["$ref: '#/components/schemas/AgentManagementProfile'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpdateAgentRequest:",
                "managementProfile:",
                "expectedVersion:",
                &[
                    "$ref: '#/components/schemas/AgentManagementProfile'",
                    "implementationProviderId:",
                    "pattern: '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
                    "implementationKind:",
                    "$ref: '#/components/schemas/AgentImplementationKind'",
                    "implementationType:",
                    "$ref: '#/components/schemas/AgentImplementationType'",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentRecord:",
                "managementProfile:",
                "implementationProviderId:",
                &["$ref: '#/components/schemas/AgentManagementProfile'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "author:",
                "avatar:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "knowledgeBaseIds:",
                "systemPrompt:",
                &[
                    "type: array",
                    "pattern: '^knowledge\\.base\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
                    "maxItems: 128",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "debugMode:",
                "iconName:",
                &["type: boolean"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "jsonMode:",
                "knowledgeBaseIds:",
                &["type: boolean"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "memoryEnabled:",
                "model:",
                &["type: boolean"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "model:",
                "skillIds:",
                &["pattern: '^model\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "skillIds:",
                "suggestedPrompts:",
                &[
                    "type: array",
                    "pattern: '^skill\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
                    "maxItems: 128",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "suggestedPrompts:",
                "systemPrompt:",
                &["type: array", "maxItems: 12", "maxLength: 256"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "temperature:",
                "toolIds:",
                &["type: number", "minimum: 0", "maximum: 2"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "toolIds:",
                "users:",
                &[
                    "type: array",
                    "pattern: '^tool\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
                    "maxItems: 128",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "voiceIds:",
                "welcomeMessage:",
                &[
                    "type: array",
                    "pattern: '^voice\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
                    "maxItems: 16",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "type:",
                "welcomeMessage:",
                &["enum:", "normal", "independent"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "AgentManagementProfile:",
                "users:",
                "welcomeMessage:",
                &["maxLength: 128"],
            );

            if label != "backend" {
                assert_schema_property_block_contains(
                    label,
                    openapi,
                    "AgentRuntimeExecutionRecord:",
                    "agentId:",
                    "executionId:",
                    &["pattern: '^agent\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
                );
            }
        }

        for required in ["provider_binding_changed", "deployment_created"] {
            assert!(
                backend_openapi.contains(required),
                "backend OpenAPI audit action enum must contain {required}"
            );
        }
    }

    #[test]
    fn openapi_specs_expose_knowledge_rag_contracts() {
        let open_openapi = include_str!("../specs/openapi/agent-business-open-openapi-3.1.2.yaml");
        let app_openapi = include_str!("../specs/openapi/agent-business-app-openapi-3.1.2.yaml");
        let backend_openapi =
            include_str!("../specs/openapi/agent-business-backend-openapi-3.1.2.yaml");

        for (label, openapi, prefix) in [
            ("open", open_openapi, AGENT_OPEN_API_PREFIX),
            ("app", app_openapi, AGENT_APP_API_PREFIX),
            ("backend", backend_openapi, AGENT_BACKEND_API_PREFIX),
        ] {
            for required in [
                format!("{prefix}/ai/knowledge_bases:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/restore:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/sources:"),
                format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}:"),
                format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}/restore:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/documents:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/search:"),
                format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}:"),
                format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/restore:"),
                format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/chunks:"),
                format!("{prefix}/ai/knowledge_chunks/{{knowledgeChunkId}}:"),
                format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/indexes:"),
                format!("{prefix}/ai/knowledge_indexes:"),
                format!("{prefix}/ai/knowledge_indexes/{{knowledgeIndexId}}:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/bindings:"),
                format!("{prefix}/ai/knowledge_bindings/{{knowledgeBindingId}}:"),
                format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/sync_jobs:"),
                format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}:"),
                format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/start:"),
                format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/complete:"),
                format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/fail:"),
                format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/cancel:"),
                "operationId: knowledgeBases.list".to_string(),
                "operationId: knowledgeBases.create".to_string(),
                "operationId: knowledgeBases.retrieve".to_string(),
                "operationId: knowledgeBases.update".to_string(),
                "operationId: knowledgeBases.delete".to_string(),
                "operationId: knowledgeBases.restore".to_string(),
                "operationId: knowledgeSources.list".to_string(),
                "operationId: knowledgeSources.create".to_string(),
                "operationId: knowledgeSources.retrieve".to_string(),
                "operationId: knowledgeSources.update".to_string(),
                "operationId: knowledgeSources.delete".to_string(),
                "operationId: knowledgeSources.restore".to_string(),
                "operationId: knowledgeList.list".to_string(),
                "operationId: knowledgeDocuments.create".to_string(),
                "operationId: knowledgeSearch.search".to_string(),
                "operationId: knowledgeRead.read".to_string(),
                "operationId: knowledgeDocuments.update".to_string(),
                "operationId: knowledgeDocuments.delete".to_string(),
                "operationId: knowledgeDocuments.restore".to_string(),
                "operationId: knowledgeChunks.list".to_string(),
                "operationId: knowledgeChunks.create".to_string(),
                "operationId: knowledgeChunks.retrieve".to_string(),
                "operationId: knowledgeIndexes.list".to_string(),
                "operationId: knowledgeIndexes.upsert".to_string(),
                "operationId: knowledgeIndexes.retrieve".to_string(),
                "operationId: knowledgeBindings.list".to_string(),
                "operationId: knowledgeBindings.create".to_string(),
                "operationId: knowledgeBindings.retrieve".to_string(),
                "operationId: knowledgeSyncJobs.list".to_string(),
                "operationId: knowledgeSyncJobs.create".to_string(),
                "operationId: knowledgeSyncJobs.retrieve".to_string(),
                "operationId: knowledgeSyncJobs.start".to_string(),
                "operationId: knowledgeSyncJobs.complete".to_string(),
                "operationId: knowledgeSyncJobs.fail".to_string(),
                "operationId: knowledgeSyncJobs.cancel".to_string(),
                "KnowledgeBaseKind:".to_string(),
                "KnowledgeIndexKind:".to_string(),
                "KnowledgeSourceKind:".to_string(),
                "KnowledgeDocumentKind:".to_string(),
                "KnowledgeBindingScopeKind:".to_string(),
                "KnowledgeSyncJobKind:".to_string(),
                "KnowledgeSyncJobStatus:".to_string(),
                "KnowledgeBaseRecord:".to_string(),
                "KnowledgeSourceRecord:".to_string(),
                "KnowledgeDocumentRecord:".to_string(),
                "KnowledgeChunkRecord:".to_string(),
                "KnowledgeIndexRecord:".to_string(),
                "KnowledgeSearchResult:".to_string(),
                "KnowledgeBindingRecord:".to_string(),
                "KnowledgeSyncJobRecord:".to_string(),
                "CreateKnowledgeBaseRequest:".to_string(),
                "UpdateKnowledgeBaseRequest:".to_string(),
                "CreateKnowledgeSourceRequest:".to_string(),
                "UpdateKnowledgeSourceRequest:".to_string(),
                "CreateKnowledgeDocumentRequest:".to_string(),
                "UpdateKnowledgeDocumentRequest:".to_string(),
                "CreateKnowledgeChunkRequest:".to_string(),
                "SearchKnowledgeRequest:".to_string(),
                "KnowledgeSearchResponse:".to_string(),
                "UpsertKnowledgeIndexRequest:".to_string(),
                "CreateKnowledgeBindingRequest:".to_string(),
                "CreateKnowledgeSyncJobRequest:".to_string(),
                "StartKnowledgeSyncJobRequest:".to_string(),
                "CompleteKnowledgeSyncJobRequest:".to_string(),
                "FailKnowledgeSyncJobRequest:".to_string(),
                "CancelKnowledgeSyncJobRequest:".to_string(),
                "pattern: '^knowledge\\.base\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^knowledge\\.source\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^knowledge\\.document\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^knowledge\\.chunk\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^knowledge\\.index\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^knowledge\\.binding\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^knowledge\\.sync\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "vector knowledge index requires embeddingModelId and vectorDimension".to_string(),
                "x-sdkwork-resource: knowledgeList".to_string(),
                "x-sdkwork-resource: knowledgeRead".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.list".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.read".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.source.retrieve".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.source.update".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.source.delete".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.source.restore".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.document.update".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.chunk.retrieve".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.index.retrieve".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.binding.retrieve".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.sync_job.retrieve".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.sync_job.start".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.sync_job.complete".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.sync_job.fail".to_string(),
                "x-sdkwork-permission: agent.business.knowledge.sync_job.cancel".to_string(),
            ] {
                assert!(
                    openapi.contains(required.as_str()),
                    "{label} OpenAPI must contain {required}"
                );
            }

            for forbidden in [
                "operationId: knowledgeDocuments.list",
                "operationId: knowledgeDocuments.retrieve",
                "operationId: knowledgeChunks.update",
                "operationId: knowledgeChunks.delete",
                "operationId: knowledgeChunks.restore",
                "operationId: knowledgeIndexes.update",
                "operationId: knowledgeIndexes.delete",
                "operationId: knowledgeIndexes.restore",
                "operationId: knowledgeBindings.update",
                "operationId: knowledgeBindings.delete",
                "operationId: knowledgeBindings.restore",
                "x-sdkwork-permission: agent.business.knowledge.document.list",
                "x-sdkwork-permission: agent.business.knowledge.document.retrieve",
                "x-sdkwork-permission: agent.business.knowledge.chunk.update",
                "x-sdkwork-permission: agent.business.knowledge.chunk.delete",
                "x-sdkwork-permission: agent.business.knowledge.chunk.restore",
                "x-sdkwork-permission: agent.business.knowledge.index.update",
                "x-sdkwork-permission: agent.business.knowledge.index.delete",
                "x-sdkwork-permission: agent.business.knowledge.index.restore",
                "x-sdkwork-permission: agent.business.knowledge.binding.update",
                "x-sdkwork-permission: agent.business.knowledge.binding.delete",
                "x-sdkwork-permission: agent.business.knowledge.binding.restore",
            ] {
                assert!(
                    !openapi.contains(forbidden),
                    "{label} OpenAPI must expose provider-neutral knowledge read/list and must not expose unsupported lifecycle contract {forbidden}"
                );
            }

            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeBaseRecord:",
                "retrievalModes:",
                "capabilityIds:",
                &[
                    "$ref: '#/components/schemas/KnowledgeIndexKind'",
                    "minItems: 1",
                    "uniqueItems: true",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeBaseRecord:",
                "documentCount:",
                "status:",
                &["type: integer", "minimum: 0"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeIndexKind:",
                "enum:",
                "KnowledgeSourceKind:",
                &[
                    "exact",
                    "keyword",
                    "full_text",
                    "structured",
                    "graph",
                    "wiki",
                    "rule",
                    "vector",
                    "hybrid",
                    "llm_rerank",
                    "external",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "SearchKnowledgeRequest:",
                "retrievalModes:",
                "includeExternal:",
                &[
                    "$ref: '#/components/schemas/KnowledgeIndexKind'",
                    "uniqueItems: true",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "SearchKnowledgeRequest:",
                "topK:",
                "retrievalModes:",
                &["minimum: 1", "maximum: 100", "default: 10"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeSearchResult:",
                "retrievalMethod:",
                "knowledgeDocumentId:",
                &["$ref: '#/components/schemas/KnowledgeIndexKind'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeSourceRecord:",
                "sourceHash:",
                "syncPolicy:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeDocumentRecord:",
                "contentHash:",
                "summary:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeDocumentRecord:",
                "trustLevel:",
                "redactionClassification:",
                &["maximum: 5"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeDocumentRecord:",
                "documentProfile:",
                "tags:",
                &["$ref: '#/components/schemas/KnowledgeDocumentProfile'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeDocumentRequest:",
                "documentProfile:",
                "tags:",
                &["$ref: '#/components/schemas/KnowledgeDocumentProfile'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpdateKnowledgeDocumentRequest:",
                "documentProfile:",
                "tags:",
                &["$ref: '#/components/schemas/KnowledgeDocumentProfile'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeDocumentProfile:",
                "author:",
                "content:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeDocumentProfile:",
                "parentId:",
                "fileName:",
                &["pattern: '^knowledge\\.document\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeDocumentProfile:",
                "enum:",
                "fileName:",
                &["markdown", "file", "folder"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeChunkRecord:",
                "contentHash:",
                "tokenEstimate:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeChunkRecord:",
                "chunkOrdinal:",
                "heading:",
                &["minimum: 1"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeChunkRecord:",
                "tokenEstimate:",
                "summary:",
                &["minimum: 1"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeIndexRecord:",
                "contentHash:",
                "indexedAt:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeSearchResult:",
                "trustLevel:",
                "redactionClassification:",
                &["maximum: 5"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "KnowledgeBindingRecord:",
                "scopeRef:",
                "active:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeSourceRequest:",
                "sourceHash:",
                "syncPolicy:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpdateKnowledgeSourceRequest:",
                "sourceHash:",
                "syncPolicy:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeDocumentRequest:",
                "contentHash:",
                "summary:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeDocumentRequest:",
                "trustLevel:",
                "redactionClassification:",
                &["maximum: 5"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpdateKnowledgeDocumentRequest:",
                "contentHash:",
                "summary:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpdateKnowledgeDocumentRequest:",
                "trustLevel:",
                "redactionClassification:",
                &["maximum: 5"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeChunkRequest:",
                "contentHash:",
                "tokenEstimate:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeChunkRequest:",
                "chunkOrdinal:",
                "heading:",
                &["minimum: 1"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeChunkRequest:",
                "tokenEstimate:",
                "summary:",
                &["minimum: 1"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpsertKnowledgeIndexRequest:",
                "contentHash:",
                "requestedAt:",
                &["maxLength: 128"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "CreateKnowledgeBindingRequest:",
                "scopeRef:",
                "active:",
                &[
                    "maxLength: 128",
                    "Agent scopes require scopeRef to match agentId; deployment scopes require scopeRef to match deploymentId and include agentId.",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/start:"),
                "post:",
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/complete:"),
                &[
                    "operationId: knowledgeSyncJobs.start",
                    "x-sdkwork-resource: knowledgeSyncJobs",
                    "x-sdkwork-permission: agent.business.knowledge.sync_job.start",
                    "x-sdkwork-audit-event: agent.business.knowledge_sync_job_started",
                    "$ref: '#/components/parameters/KnowledgeSyncJobIdPath'",
                    "$ref: '#/components/schemas/StartKnowledgeSyncJobRequest'",
                    "$ref: '#/components/schemas/KnowledgeSyncJobResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/complete:"),
                "post:",
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/fail:"),
                &[
                    "operationId: knowledgeSyncJobs.complete",
                    "x-sdkwork-resource: knowledgeSyncJobs",
                    "x-sdkwork-permission: agent.business.knowledge.sync_job.complete",
                    "x-sdkwork-audit-event: agent.business.knowledge_sync_job_completed",
                    "$ref: '#/components/parameters/KnowledgeSyncJobIdPath'",
                    "$ref: '#/components/schemas/CompleteKnowledgeSyncJobRequest'",
                    "$ref: '#/components/schemas/KnowledgeSyncJobResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/fail:"),
                "post:",
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/cancel:"),
                &[
                    "operationId: knowledgeSyncJobs.fail",
                    "x-sdkwork-resource: knowledgeSyncJobs",
                    "x-sdkwork-permission: agent.business.knowledge.sync_job.fail",
                    "x-sdkwork-audit-event: agent.business.knowledge_sync_job_failed",
                    "$ref: '#/components/parameters/KnowledgeSyncJobIdPath'",
                    "$ref: '#/components/schemas/FailKnowledgeSyncJobRequest'",
                    "$ref: '#/components/schemas/KnowledgeSyncJobResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sync_jobs/{{syncJobId}}/cancel:"),
                "post:",
                &format!("{prefix}/ai/knowledge_indexes:"),
                &[
                    "operationId: knowledgeSyncJobs.cancel",
                    "x-sdkwork-resource: knowledgeSyncJobs",
                    "x-sdkwork-permission: agent.business.knowledge.sync_job.cancel",
                    "x-sdkwork-audit-event: agent.business.knowledge_sync_job_cancelled",
                    "$ref: '#/components/parameters/KnowledgeSyncJobIdPath'",
                    "$ref: '#/components/schemas/CancelKnowledgeSyncJobRequest'",
                    "$ref: '#/components/schemas/KnowledgeSyncJobResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}:"),
                "get:",
                "patch:",
                &[
                    "operationId: knowledgeSources.retrieve",
                    "x-sdkwork-resource: knowledgeSources",
                    "x-sdkwork-permission: agent.business.knowledge.source.retrieve",
                    "$ref: '#/components/parameters/KnowledgeSourceIdPath'",
                    "$ref: '#/components/schemas/KnowledgeSourceResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}:"),
                "patch:",
                "delete:",
                &[
                    "operationId: knowledgeSources.update",
                    "x-sdkwork-resource: knowledgeSources",
                    "x-sdkwork-permission: agent.business.knowledge.source.update",
                    "$ref: '#/components/schemas/UpdateKnowledgeSourceRequest'",
                    "$ref: '#/components/schemas/KnowledgeSourceResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}:"),
                "delete:",
                &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}/restore:"),
                &[
                    "operationId: knowledgeSources.delete",
                    "x-sdkwork-resource: knowledgeSources",
                    "x-sdkwork-permission: agent.business.knowledge.source.delete",
                    "$ref: '#/components/parameters/ExpectedVersion'",
                    "$ref: '#/components/parameters/RequestedAt'",
                    "$ref: '#/components/schemas/KnowledgeSourceResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_sources/{{knowledgeSourceId}}/restore:"),
                "post:",
                &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/documents:"),
                &[
                    "operationId: knowledgeSources.restore",
                    "x-sdkwork-resource: knowledgeSources",
                    "x-sdkwork-permission: agent.business.knowledge.source.restore",
                    "$ref: '#/components/schemas/RestoreAgentRequest'",
                    "$ref: '#/components/schemas/KnowledgeSourceResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/documents:"),
                "get:",
                "post:",
                &[
                    "operationId: knowledgeList.list",
                    "x-sdkwork-resource: knowledgeList",
                    "x-sdkwork-permission: agent.business.knowledge.list",
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                    "$ref: '#/components/schemas/KnowledgeDocumentListResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_bases/{{knowledgeBaseId}}/search:"),
                "post:",
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}:"),
                &[
                    "operationId: knowledgeSearch.search",
                    "$ref: '#/components/schemas/SearchKnowledgeRequest'",
                    "$ref: '#/components/schemas/KnowledgeSearchResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}:"),
                "patch:",
                "delete:",
                &[
                    "operationId: knowledgeDocuments.update",
                    "x-sdkwork-resource: knowledgeDocuments",
                    "x-sdkwork-permission: agent.business.knowledge.document.update",
                    "$ref: '#/components/schemas/UpdateKnowledgeDocumentRequest'",
                    "$ref: '#/components/schemas/KnowledgeDocumentResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}:"),
                "delete:",
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/restore:"),
                &[
                    "operationId: knowledgeDocuments.delete",
                    "x-sdkwork-resource: knowledgeDocuments",
                    "x-sdkwork-permission: agent.business.knowledge.document.delete",
                    "- $ref: '#/components/parameters/ExpectedVersion'",
                    "- $ref: '#/components/parameters/RequestedAt'",
                    "$ref: '#/components/schemas/KnowledgeDocumentResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
            assert_operation_block_excludes(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}:"),
                "delete:",
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}/restore:"),
                &[
                    "requestBody:",
                    "$ref: '#/components/schemas/DeleteAgentRequest'",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_documents/{{knowledgeDocumentId}}:"),
                "get:",
                "patch:",
                &[
                    "operationId: knowledgeRead.read",
                    "x-sdkwork-resource: knowledgeRead",
                    "x-sdkwork-permission: agent.business.knowledge.read",
                    "$ref: '#/components/schemas/KnowledgeDocumentResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpsertKnowledgeIndexRequest:",
                "embeddingModelId:",
                "vectorDimension:",
                &["pattern: '^model\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_bases:"),
                "get:",
                "post:",
                &[
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/knowledge_indexes:"),
                "post:",
                "components:",
                &["'400':", "'403':", "'404':", "'409':"],
            );
        }
    }

    #[test]
    fn openapi_specs_expose_memory_rag_contracts() {
        let open_openapi = include_str!("../specs/openapi/agent-business-open-openapi-3.1.2.yaml");
        let app_openapi = include_str!("../specs/openapi/agent-business-app-openapi-3.1.2.yaml");
        let backend_openapi =
            include_str!("../specs/openapi/agent-business-backend-openapi-3.1.2.yaml");

        for (label, openapi, prefix) in [
            ("open", open_openapi, AGENT_OPEN_API_PREFIX),
            ("app", app_openapi, AGENT_APP_API_PREFIX),
            ("backend", backend_openapi, AGENT_BACKEND_API_PREFIX),
        ] {
            for required in [
                format!("{prefix}/ai/memory_stores:"),
                format!("{prefix}/ai/memory_stores/{{memoryStoreId}}:"),
                format!("{prefix}/ai/memory_stores/{{memoryStoreId}}/profiles:"),
                format!("{prefix}/ai/memory_profiles/{{memoryProfileId}}:"),
                format!("{prefix}/ai/memory_profiles/{{memoryProfileId}}/bindings:"),
                format!("{prefix}/ai/memory_bindings/{{memoryBindingId}}:"),
                format!("{prefix}/ai/memory_namespaces:"),
                format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}:"),
                format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}/records:"),
                format!("{prefix}/ai/memory_records/{{memoryId}}:"),
                format!("{prefix}/ai/memory_records/{{memoryId}}/restore:"),
                format!("{prefix}/ai/memory_records/{{memoryId}}/sources:"),
                format!("{prefix}/ai/memory_records/{{memoryId}}/relations:"),
                format!("{prefix}/ai/memory_records/{{memoryId}}/retrieval_indexes:"),
                format!("{prefix}/ai/memory_retrieval_indexes:"),
                "operationId: memoryStores.create".to_string(),
                "operationId: memoryStores.retrieve".to_string(),
                "operationId: memoryStores.update".to_string(),
                "operationId: memoryProfiles.create".to_string(),
                "operationId: memoryProfiles.retrieve".to_string(),
                "operationId: memoryBindings.create".to_string(),
                "operationId: memoryBindings.retrieve".to_string(),
                "operationId: memoryNamespaces.create".to_string(),
                "operationId: memoryNamespaces.retrieve".to_string(),
                "operationId: memoryRecords.list".to_string(),
                "operationId: memoryRecords.create".to_string(),
                "operationId: memoryRecords.retrieve".to_string(),
                "operationId: memoryRecords.delete".to_string(),
                "operationId: memoryRecords.restore".to_string(),
                "operationId: memorySources.list".to_string(),
                "operationId: memorySources.create".to_string(),
                "operationId: memoryRelations.list".to_string(),
                "operationId: memoryRelations.create".to_string(),
                "operationId: memoryRetrievalIndexes.list".to_string(),
                "operationId: memoryRetrievalIndexes.upsert".to_string(),
                "MemoryStoreKind:".to_string(),
                "MemoryIndexKind:".to_string(),
                "MemoryBindingScopeKind:".to_string(),
                "MemoryNamespaceKind:".to_string(),
                "MemoryRecordKind:".to_string(),
                "MemorySourceKind:".to_string(),
                "MemoryRelationKind:".to_string(),
                "MemoryStoreRecord:".to_string(),
                "MemoryProfileRecord:".to_string(),
                "MemoryBindingRecord:".to_string(),
                "MemoryNamespaceRecord:".to_string(),
                "MemoryRecord:".to_string(),
                "MemorySourceRecord:".to_string(),
                "MemoryRelationRecord:".to_string(),
                "MemoryRetrievalIndexRecord:".to_string(),
                "CreateMemoryStoreRequest:".to_string(),
                "UpdateMemoryStoreRequest:".to_string(),
                "CreateMemoryProfileRequest:".to_string(),
                "CreateMemoryBindingRequest:".to_string(),
                "CreateMemoryNamespaceRequest:".to_string(),
                "CreateMemoryRecordRequest:".to_string(),
                "CreateMemorySourceRequest:".to_string(),
                "CreateMemoryRelationRequest:".to_string(),
                "UpsertMemoryRetrievalIndexRequest:".to_string(),
                "pattern: '^memory\\.store\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.profile\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.binding\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.namespace\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.record\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.source\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.relation\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "pattern: '^memory\\.index\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'".to_string(),
                "memory retrieval is provider-neutral and is not vector-only".to_string(),
                "vector memory index requires embeddingModelId and vectorDimension".to_string(),
                "x-sdkwork-permission: agent.business.memory.store.create".to_string(),
                "x-sdkwork-permission: agent.business.memory.record.list".to_string(),
                "x-sdkwork-permission: agent.business.memory.retrieval_index.upsert".to_string(),
            ] {
                assert!(
                    openapi.contains(required.as_str()),
                    "{label} OpenAPI must contain {required}"
                );
            }

            for forbidden in [
                "operationId: memoryStores.delete",
                "operationId: memoryStores.restore",
                "operationId: memoryProfiles.update",
                "operationId: memoryProfiles.delete",
                "operationId: memoryProfiles.restore",
                "operationId: memoryBindings.update",
                "operationId: memoryBindings.delete",
                "operationId: memoryBindings.restore",
                "operationId: memoryNamespaces.update",
                "operationId: memoryNamespaces.delete",
                "operationId: memoryNamespaces.restore",
                "operationId: memoryRecords.update",
                "operationId: memorySources.update",
                "operationId: memorySources.delete",
                "operationId: memorySources.restore",
                "operationId: memoryRelations.update",
                "operationId: memoryRelations.delete",
                "operationId: memoryRelations.restore",
                "operationId: memoryRetrievalIndexes.update",
                "operationId: memoryRetrievalIndexes.delete",
                "operationId: memoryRetrievalIndexes.restore",
                "x-sdkwork-permission: agent.business.memory.store.delete",
                "x-sdkwork-permission: agent.business.memory.store.restore",
                "x-sdkwork-permission: agent.business.memory.profile.update",
                "x-sdkwork-permission: agent.business.memory.profile.delete",
                "x-sdkwork-permission: agent.business.memory.profile.restore",
                "x-sdkwork-permission: agent.business.memory.binding.update",
                "x-sdkwork-permission: agent.business.memory.binding.delete",
                "x-sdkwork-permission: agent.business.memory.binding.restore",
                "x-sdkwork-permission: agent.business.memory.namespace.update",
                "x-sdkwork-permission: agent.business.memory.namespace.delete",
                "x-sdkwork-permission: agent.business.memory.namespace.restore",
                "x-sdkwork-permission: agent.business.memory.record.update",
                "x-sdkwork-permission: agent.business.memory.source.update",
                "x-sdkwork-permission: agent.business.memory.source.delete",
                "x-sdkwork-permission: agent.business.memory.source.restore",
                "x-sdkwork-permission: agent.business.memory.relation.update",
                "x-sdkwork-permission: agent.business.memory.relation.delete",
                "x-sdkwork-permission: agent.business.memory.relation.restore",
                "x-sdkwork-permission: agent.business.memory.retrieval_index.update",
                "x-sdkwork-permission: agent.business.memory.retrieval_index.delete",
                "x-sdkwork-permission: agent.business.memory.retrieval_index.restore",
            ] {
                assert!(
                    !openapi.contains(forbidden),
                    "{label} OpenAPI must not expose unsupported memory lifecycle contract {forbidden}"
                );
            }

            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryStoreRecord:",
                "retrievalModes:",
                "capabilityIds:",
                &[
                    "$ref: '#/components/schemas/MemoryIndexKind'",
                    "minItems: 1",
                    "uniqueItems: true",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryIndexKind:",
                "enum:",
                "MemoryBindingScopeKind:",
                &[
                    "keyword", "sparse", "vector", "graph", "wiki", "rule", "hybrid",
                ],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryProfileRecord:",
                "writePolicy:",
                "retrievalPolicy:",
                &["type: object", "additionalProperties: true"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryProfileRecord:",
                "retrievalPolicy:",
                "compactionPolicy:",
                &["type: object", "additionalProperties: true"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryRecord:",
                "content:",
                "summary:",
                &["type: object", "additionalProperties: true"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryRecord:",
                "salienceScore:",
                "confidenceScore:",
                &["minimum: 0", "maximum: 1"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemoryRecord:",
                "sensitivityLevel:",
                "sourceCount:",
                &["minimum: 0", "maximum: 4"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "MemorySourceRecord:",
                "evidence:",
                "capturedAt:",
                &["type: object", "additionalProperties: true"],
            );
            assert_schema_property_block_contains(
                label,
                openapi,
                "UpsertMemoryRetrievalIndexRequest:",
                "embeddingModelId:",
                "vectorDimension:",
                &["pattern: '^model\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'"],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/memory_namespaces/{{memoryNamespaceId}}/records:"),
                "get:",
                "post:",
                &[
                    "operationId: memoryRecords.list",
                    "x-sdkwork-resource: memoryRecords",
                    "x-sdkwork-permission: agent.business.memory.record.list",
                    "- $ref: '#/components/parameters/Page'",
                    "- $ref: '#/components/parameters/PageSize'",
                    "$ref: '#/components/schemas/MemoryRecordListResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                ],
            );
            assert_operation_block_contains(
                label,
                openapi,
                &format!("{prefix}/ai/memory_retrieval_indexes:"),
                "post:",
                "components:",
                &[
                    "operationId: memoryRetrievalIndexes.upsert",
                    "x-sdkwork-resource: memoryRetrievalIndexes",
                    "x-sdkwork-permission: agent.business.memory.retrieval_index.upsert",
                    "$ref: '#/components/schemas/UpsertMemoryRetrievalIndexRequest'",
                    "$ref: '#/components/schemas/MemoryRetrievalIndexResponse'",
                    "'400':",
                    "'403':",
                    "'404':",
                    "'409':",
                ],
            );
        }
    }

    fn assert_operation_block_contains(
        label: &str,
        openapi: &str,
        path: &str,
        operation: &str,
        until: &str,
        required: &[&str],
    ) {
        let block = operation_block(openapi, path, operation, until);
        for item in required {
            assert!(
                block.contains(item),
                "{label} OpenAPI block {path} {operation} must contain {item}"
            );
        }
    }

    fn assert_operation_block_excludes(
        label: &str,
        openapi: &str,
        path: &str,
        operation: &str,
        until: &str,
        forbidden: &[&str],
    ) {
        let block = operation_block(openapi, path, operation, until);
        for item in forbidden {
            assert!(
                !block.contains(item),
                "{label} OpenAPI block {path} {operation} must not contain {item}"
            );
        }
    }

    fn assert_schema_property_block_contains(
        label: &str,
        openapi: &str,
        schema: &str,
        property: &str,
        until: &str,
        required: &[&str],
    ) {
        let schema_start = openapi
            .find(schema)
            .unwrap_or_else(|| panic!("{label} OpenAPI must contain schema {schema}"));
        let after_schema = &openapi[schema_start..];
        let property_start = after_schema
            .find(property)
            .unwrap_or_else(|| panic!("{label} OpenAPI schema {schema} must contain {property}"));
        let after_property = &after_schema[property_start..];
        let end = after_property.find(until).unwrap_or_else(|| {
            panic!("{label} OpenAPI schema {schema} property {property} must end at {until}")
        });
        let block = &after_property[..end];

        for item in required {
            assert!(
                block.contains(item),
                "{label} OpenAPI schema {schema} property {property} must contain {item}"
            );
        }
    }

    fn assert_schema_block_excludes(
        label: &str,
        openapi: &str,
        schema: &str,
        until: &str,
        forbidden: &[&str],
    ) {
        let schema_start = openapi
            .find(schema)
            .unwrap_or_else(|| panic!("{label} OpenAPI must contain schema {schema}"));
        let after_schema = &openapi[schema_start..];
        let end = after_schema
            .find(until)
            .unwrap_or_else(|| panic!("{label} OpenAPI schema {schema} must end at {until}"));
        let block = &after_schema[..end];

        for item in forbidden {
            assert!(
                !block.contains(item),
                "{label} OpenAPI schema {schema} must not contain {item}"
            );
        }
    }

    fn operation_block<'a>(openapi: &'a str, path: &str, operation: &str, until: &str) -> &'a str {
        let path_start = openapi
            .find(path)
            .unwrap_or_else(|| panic!("OpenAPI must contain path {path}"));
        let after_path = &openapi[path_start..];
        let operation_start = after_path
            .find(operation)
            .unwrap_or_else(|| panic!("OpenAPI path {path} must contain operation {operation}"));
        let after_operation = &after_path[operation_start..];
        let end = after_operation
            .find(until)
            .unwrap_or_else(|| panic!("OpenAPI operation {path} {operation} must end at {until}"));
        &after_operation[..end]
    }

    fn deployment_post_block_end(label: &str, prefix: &str) -> String {
        if label == "backend" {
            format!("{prefix}/ai/knowledge_bases:")
        } else {
            format!("{prefix}/ai/agents/{{agentId}}/preview_responses:")
        }
    }

    fn assert_operation(operations: &[ApiOperation], method: &str, path: &str, operation_id: &str) {
        assert!(
            operations.iter().any(|operation| {
                operation.method == method
                    && operation.path == path
                    && operation.operation_id == operation_id
            }),
            "{method} {path} must be registered as {operation_id}"
        );
    }
}
