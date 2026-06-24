use crate::domain::{
    AgentBusinessRecord, AgentBusinessStatus, AgentDeploymentRecord, AgentKnowledgeBaseRecord,
    AgentKnowledgeBindingRecord, AgentKnowledgeChunkRecord, AgentKnowledgeDocumentRecord,
    AgentKnowledgeIndexRecord, AgentKnowledgeSourceRecord, AgentKnowledgeSyncJobRecord,
    AgentMcpServerRecord, AgentMemoryBindingRecord, AgentMemoryNamespaceRecord,
    AgentMemoryProfileRecord, AgentMemoryRecord, AgentMemoryRelationRecord,
    AgentMemoryRetrievalIndexRecord, AgentMemorySourceRecord, AgentMemoryStoreRecord,
    AgentProviderBindingRecord,
    AgentVisibility,
};
use sdkwork_agent_kernel::{KernelError, KernelEvent, KernelResult};
use crate::validation::optional_non_blank;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentListQuery {
    pub tenant_id: u64,
    pub organization_id: Option<u64>,
    pub owner_user_id: Option<u64>,
    pub include_deleted: bool,
    pub search_query: Option<String>,
}

impl AgentListQuery {
    pub fn for_tenant(tenant_id: u64) -> Self {
        Self {
            tenant_id,
            organization_id: None,
            owner_user_id: None,
            include_deleted: false,
            search_query: None,
        }
    }

    pub fn for_organization(mut self, organization_id: u64) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    pub fn for_owner(mut self, owner_user_id: u64) -> Self {
        self.owner_user_id = Some(owner_user_id);
        self
    }

    pub fn with_deleted(mut self) -> Self {
        self.include_deleted = true;
        self
    }

    pub fn with_search(mut self, query: impl Into<String>) -> Self {
        let query = query.into();
        self.search_query = optional_non_blank(query);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMarketplaceListQuery {
    pub tenant_id: u64,
    pub organization_id: Option<u64>,
    pub owner_user_id: Option<u64>,
    pub status: Option<AgentBusinessStatus>,
    pub visibility: Option<AgentVisibility>,
    pub include_deleted: bool,
    pub search_query: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
}

impl AgentMarketplaceListQuery {
    pub fn for_tenant(tenant_id: u64) -> Self {
        Self {
            tenant_id,
            organization_id: None,
            owner_user_id: None,
            status: None,
            visibility: None,
            include_deleted: false,
            search_query: None,
            category: None,
            tag: None,
        }
    }

    pub fn for_organization(mut self, organization_id: u64) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    pub fn for_owner(mut self, owner_user_id: u64) -> Self {
        self.owner_user_id = Some(owner_user_id);
        self
    }

    pub fn with_status(mut self, status: AgentBusinessStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_visibility(mut self, visibility: AgentVisibility) -> Self {
        self.visibility = Some(visibility);
        self
    }

    pub fn with_deleted(mut self) -> Self {
        self.include_deleted = true;
        self
    }

    pub fn with_search(mut self, query: impl Into<String>) -> Self {
        let query = query.into();
        self.search_query = optional_non_blank(query);
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        let category = category.into();
        self.category = optional_non_blank(category);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let tag = tag.into();
        self.tag = optional_non_blank(tag);
        self
    }
}

pub trait AgentRepository {
    fn next_id(&mut self) -> KernelResult<u64>;

    fn insert(&mut self, record: AgentBusinessRecord) -> KernelResult<()>;

    fn update(&mut self, record: AgentBusinessRecord) -> KernelResult<()>;

    fn get(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRecord>;

    fn list(&self, query: &AgentListQuery) -> Vec<AgentBusinessRecord>;

    fn insert_provider_binding(&mut self, _record: AgentProviderBindingRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.provider_binding".to_string(),
        })
    }

    fn update_provider_binding(&mut self, _record: AgentProviderBindingRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.provider_binding".to_string(),
        })
    }

    fn get_provider_binding(
        &self,
        _tenant_id: u64,
        _agent_id: &str,
        _binding_id: &str,
    ) -> Option<AgentProviderBindingRecord> {
        None
    }

    fn list_provider_bindings(
        &self,
        _tenant_id: u64,
        _agent_id: &str,
    ) -> Vec<AgentProviderBindingRecord> {
        Vec::new()
    }

    fn insert_deployment(&mut self, _record: AgentDeploymentRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.deployment".to_string(),
        })
    }

    fn list_deployments(&self, _tenant_id: u64, _agent_id: &str) -> Vec<AgentDeploymentRecord> {
        Vec::new()
    }

    fn insert_mcp_server(&mut self, _record: AgentMcpServerRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.mcp".to_string(),
        })
    }

    fn update_mcp_server(&mut self, _record: AgentMcpServerRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.mcp".to_string(),
        })
    }

    fn get_mcp_server(
        &self,
        _tenant_id: u64,
        _mcp_server_id: &str,
    ) -> Option<AgentMcpServerRecord> {
        None
    }

    fn list_mcp_servers(&self, _query: &AgentMarketplaceListQuery) -> Vec<AgentMcpServerRecord> {
        Vec::new()
    }

    fn insert_knowledge_base(&mut self, _record: AgentKnowledgeBaseRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.base".to_string(),
        })
    }

    fn update_knowledge_base(&mut self, _record: AgentKnowledgeBaseRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.base".to_string(),
        })
    }

    fn get_knowledge_base(
        &self,
        _tenant_id: u64,
        _knowledge_base_id: &str,
    ) -> Option<AgentKnowledgeBaseRecord> {
        None
    }

    fn list_knowledge_bases(
        &self,
        _query: &AgentMarketplaceListQuery,
    ) -> Vec<AgentKnowledgeBaseRecord> {
        Vec::new()
    }

    fn insert_knowledge_source(&mut self, _record: AgentKnowledgeSourceRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.source".to_string(),
        })
    }

    fn update_knowledge_source(&mut self, _record: AgentKnowledgeSourceRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.source".to_string(),
        })
    }

    fn get_knowledge_source(
        &self,
        _tenant_id: u64,
        _knowledge_source_id: &str,
    ) -> Option<AgentKnowledgeSourceRecord> {
        None
    }

    fn list_knowledge_sources(
        &self,
        _tenant_id: u64,
        _knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeSourceRecord> {
        Vec::new()
    }

    fn insert_knowledge_document(
        &mut self,
        _record: AgentKnowledgeDocumentRecord,
    ) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.document".to_string(),
        })
    }

    fn update_knowledge_document(
        &mut self,
        _record: AgentKnowledgeDocumentRecord,
    ) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.document".to_string(),
        })
    }

    fn get_knowledge_document(
        &self,
        _tenant_id: u64,
        _knowledge_document_id: &str,
    ) -> Option<AgentKnowledgeDocumentRecord> {
        None
    }

    fn list_knowledge_documents(
        &self,
        _tenant_id: u64,
        _knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeDocumentRecord> {
        Vec::new()
    }

    fn insert_knowledge_chunk(&mut self, _record: AgentKnowledgeChunkRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.chunk".to_string(),
        })
    }

    fn get_knowledge_chunk(
        &self,
        _tenant_id: u64,
        _knowledge_chunk_id: &str,
    ) -> Option<AgentKnowledgeChunkRecord> {
        None
    }

    fn list_knowledge_chunks(
        &self,
        _tenant_id: u64,
        _knowledge_document_id: &str,
    ) -> Vec<AgentKnowledgeChunkRecord> {
        Vec::new()
    }

    fn upsert_knowledge_index(&mut self, _record: AgentKnowledgeIndexRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.index".to_string(),
        })
    }

    fn get_knowledge_index(
        &self,
        _tenant_id: u64,
        _knowledge_index_id: &str,
    ) -> Option<AgentKnowledgeIndexRecord> {
        None
    }

    fn list_knowledge_indexes(
        &self,
        _tenant_id: u64,
        _knowledge_document_id: &str,
    ) -> Vec<AgentKnowledgeIndexRecord> {
        Vec::new()
    }

    fn list_knowledge_indexes_by_base(
        &self,
        _tenant_id: u64,
        _knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeIndexRecord> {
        Vec::new()
    }

    fn insert_knowledge_binding(
        &mut self,
        _record: AgentKnowledgeBindingRecord,
    ) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.binding".to_string(),
        })
    }

    fn get_knowledge_binding(
        &self,
        _tenant_id: u64,
        _knowledge_binding_id: &str,
    ) -> Option<AgentKnowledgeBindingRecord> {
        None
    }

    fn list_knowledge_bindings(
        &self,
        _tenant_id: u64,
        _knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeBindingRecord> {
        Vec::new()
    }

    fn insert_knowledge_sync_job(
        &mut self,
        _record: AgentKnowledgeSyncJobRecord,
    ) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.sync_job".to_string(),
        })
    }

    fn update_knowledge_sync_job(
        &mut self,
        _record: AgentKnowledgeSyncJobRecord,
    ) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.knowledge.sync_job".to_string(),
        })
    }

    fn get_knowledge_sync_job(
        &self,
        _tenant_id: u64,
        _sync_job_id: &str,
    ) -> Option<AgentKnowledgeSyncJobRecord> {
        None
    }

    fn list_knowledge_sync_jobs(
        &self,
        _tenant_id: u64,
        _knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeSyncJobRecord> {
        Vec::new()
    }

    fn insert_memory_store(&mut self, _record: AgentMemoryStoreRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.store".to_string(),
        })
    }

    fn update_memory_store(&mut self, _record: AgentMemoryStoreRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.store".to_string(),
        })
    }

    fn get_memory_store(
        &self,
        _tenant_id: u64,
        _memory_store_id: &str,
    ) -> Option<AgentMemoryStoreRecord> {
        None
    }

    fn insert_memory_profile(&mut self, _record: AgentMemoryProfileRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.profile".to_string(),
        })
    }

    fn get_memory_profile(
        &self,
        _tenant_id: u64,
        _memory_profile_id: &str,
    ) -> Option<AgentMemoryProfileRecord> {
        None
    }

    fn insert_memory_binding(&mut self, _record: AgentMemoryBindingRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.binding".to_string(),
        })
    }

    fn get_memory_binding(
        &self,
        _tenant_id: u64,
        _memory_binding_id: &str,
    ) -> Option<AgentMemoryBindingRecord> {
        None
    }

    fn insert_memory_namespace(&mut self, _record: AgentMemoryNamespaceRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.namespace".to_string(),
        })
    }

    fn get_memory_namespace(
        &self,
        _tenant_id: u64,
        _memory_namespace_id: &str,
    ) -> Option<AgentMemoryNamespaceRecord> {
        None
    }

    fn insert_memory_record(&mut self, _record: AgentMemoryRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.record".to_string(),
        })
    }

    fn update_memory_record(&mut self, _record: AgentMemoryRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.record".to_string(),
        })
    }

    fn get_memory_record(&self, _tenant_id: u64, _memory_id: &str) -> Option<AgentMemoryRecord> {
        None
    }

    fn list_memory_records(
        &self,
        _tenant_id: u64,
        _memory_namespace_id: &str,
    ) -> Vec<AgentMemoryRecord> {
        Vec::new()
    }

    fn insert_memory_source(&mut self, _record: AgentMemorySourceRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.source".to_string(),
        })
    }

    fn list_memory_sources(
        &self,
        _tenant_id: u64,
        _memory_id: &str,
    ) -> Vec<AgentMemorySourceRecord> {
        Vec::new()
    }

    fn insert_memory_relation(&mut self, _record: AgentMemoryRelationRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.relation".to_string(),
        })
    }

    fn list_memory_relations(
        &self,
        _tenant_id: u64,
        _memory_id: &str,
    ) -> Vec<AgentMemoryRelationRecord> {
        Vec::new()
    }

    fn upsert_memory_retrieval_index(
        &mut self,
        _record: AgentMemoryRetrievalIndexRecord,
    ) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.retrieval_index".to_string(),
        })
    }

    fn list_memory_retrieval_indexes(
        &self,
        _tenant_id: u64,
        _memory_id: &str,
    ) -> Vec<AgentMemoryRetrievalIndexRecord> {
        Vec::new()
    }

    fn list_memory_stores(&self, _tenant_id: u64) -> Vec<AgentMemoryStoreRecord> {
        Vec::new()
    }

    fn update_memory_profile(&mut self, _record: AgentMemoryProfileRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.profile".to_string(),
        })
    }

    fn update_memory_binding(&mut self, _record: AgentMemoryBindingRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.binding".to_string(),
        })
    }

    fn update_memory_namespace(&mut self, _record: AgentMemoryNamespaceRecord) -> KernelResult<()> {
        Err(KernelError::CapabilityMissing {
            capability_id: "agent.business.memory.namespace".to_string(),
        })
    }

    fn get_memory_source(
        &self,
        _tenant_id: u64,
        _memory_source_id: &str,
    ) -> Option<AgentMemorySourceRecord> {
        None
    }

    fn get_memory_relation(
        &self,
        _tenant_id: u64,
        _memory_relation_id: &str,
    ) -> Option<AgentMemoryRelationRecord> {
        None
    }

    fn get_memory_retrieval_index(
        &self,
        _tenant_id: u64,
        _retrieval_index_id: &str,
    ) -> Option<AgentMemoryRetrievalIndexRecord> {
        None
    }
}

pub trait AgentAuditSink {
    fn record(&mut self, event: KernelEvent) -> KernelResult<()>;

    fn list_events(&self, _tenant_id: u64, _agent_id: &str) -> KernelResult<Vec<KernelEvent>> {
        Ok(Vec::new())
    }
}
