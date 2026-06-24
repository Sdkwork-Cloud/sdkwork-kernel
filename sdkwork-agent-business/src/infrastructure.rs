use crate::domain::{
    AgentBusinessRecord, AgentDeploymentRecord, AgentKnowledgeBaseRecord,
    AgentKnowledgeBindingRecord, AgentKnowledgeBindingScopeKind, AgentKnowledgeChunkRecord,
    AgentKnowledgeDocumentRecord, AgentKnowledgeIndexRecord, AgentKnowledgeSourceRecord,
    AgentKnowledgeSyncJobRecord, AgentMcpServerRecord, AgentMemoryBindingRecord,
    AgentMemoryBindingScopeKind, AgentMemoryNamespaceRecord, AgentMemoryProfileRecord,
    AgentMemoryRecord, AgentMemoryRelationRecord, AgentMemoryRetrievalIndexRecord,
    AgentMemorySourceRecord, AgentMemoryStoreRecord, AgentProviderBindingRecord,
};
use crate::id::{AgentBusinessIdGenerator, AgentIdGenerator};
use crate::ports::{AgentAuditSink, AgentListQuery, AgentMarketplaceListQuery, AgentRepository};
use sdkwork_agent_kernel::{
    KernelError, KernelEvent, KernelResult, PolicyDecision, PolicyProvider, PolicyRequest,
};
use std::cmp::Ordering;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct InMemoryAgentRepository {
    id_generator: AgentBusinessIdGenerator,
    records: Vec<AgentBusinessRecord>,
    provider_bindings: Vec<AgentProviderBindingRecord>,
    deployments: Vec<AgentDeploymentRecord>,
    mcp_servers: Vec<AgentMcpServerRecord>,
    knowledge_bases: Vec<AgentKnowledgeBaseRecord>,
    knowledge_sources: Vec<AgentKnowledgeSourceRecord>,
    knowledge_documents: Vec<AgentKnowledgeDocumentRecord>,
    knowledge_chunks: Vec<AgentKnowledgeChunkRecord>,
    knowledge_indexes: Vec<AgentKnowledgeIndexRecord>,
    knowledge_bindings: Vec<AgentKnowledgeBindingRecord>,
    knowledge_sync_jobs: Vec<AgentKnowledgeSyncJobRecord>,
    memory_stores: Vec<AgentMemoryStoreRecord>,
    memory_profiles: Vec<AgentMemoryProfileRecord>,
    memory_bindings: Vec<AgentMemoryBindingRecord>,
    memory_namespaces: Vec<AgentMemoryNamespaceRecord>,
    memory_records: Vec<AgentMemoryRecord>,
    memory_sources: Vec<AgentMemorySourceRecord>,
    memory_relations: Vec<AgentMemoryRelationRecord>,
    memory_retrieval_indexes: Vec<AgentMemoryRetrievalIndexRecord>,
}

impl InMemoryAgentRepository {
    pub fn new() -> Self {
        Self {
            id_generator: AgentBusinessIdGenerator::new_default()
                .expect("default agent business snowflake node id is valid"),
            records: Vec::new(),
            provider_bindings: Vec::new(),
            deployments: Vec::new(),
            mcp_servers: Vec::new(),
            knowledge_bases: Vec::new(),
            knowledge_sources: Vec::new(),
            knowledge_documents: Vec::new(),
            knowledge_chunks: Vec::new(),
            knowledge_indexes: Vec::new(),
            knowledge_bindings: Vec::new(),
            knowledge_sync_jobs: Vec::new(),
            memory_stores: Vec::new(),
            memory_profiles: Vec::new(),
            memory_bindings: Vec::new(),
            memory_namespaces: Vec::new(),
            memory_records: Vec::new(),
            memory_sources: Vec::new(),
            memory_relations: Vec::new(),
            memory_retrieval_indexes: Vec::new(),
        }
    }

    pub fn records(&self) -> &[AgentBusinessRecord] {
        &self.records
    }
}

impl Default for InMemoryAgentRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRepository for InMemoryAgentRepository {
    fn next_id(&mut self) -> KernelResult<u64> {
        self.id_generator.next_id()
    }

    fn insert(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        if self.records.iter().any(|existing| {
            existing.tenant_id == record.tenant_id && existing.agent_id == record.agent_id
        }) {
            return Err(KernelError::conflict("agent already exists"));
        }
        if self
            .records
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.code == record.code)
        {
            return Err(KernelError::conflict("agent code already exists"));
        }
        self.records.push(record);
        Ok(())
    }

    fn update(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        let Some(index) = self.records.iter().position(|existing| {
            existing.tenant_id == record.tenant_id && existing.agent_id == record.agent_id
        }) else {
            return Err(KernelError::validation("agent not found"));
        };
        let expected_version = self.records[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if self.records.iter().enumerate().any(|(current, existing)| {
            current != index
                && existing.tenant_id == record.tenant_id
                && existing.code == record.code
        }) {
            return Err(KernelError::conflict("agent code already exists"));
        }
        self.records[index] = record;
        Ok(())
    }

    fn get(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRecord> {
        self.records
            .iter()
            .find(|record| record.tenant_id == tenant_id && record.agent_id == agent_id)
            .cloned()
    }

    fn list(&self, query: &AgentListQuery) -> Vec<AgentBusinessRecord> {
        self.records
            .iter()
            .filter(|record| record.tenant_id == query.tenant_id)
            .filter(|record| {
                if let Some(organization_id) = query.organization_id {
                    record.organization_id == organization_id
                } else {
                    true
                }
            })
            .filter(|record| {
                if let Some(owner_user_id) = query.owner_user_id {
                    record.owner_user_id == owner_user_id
                } else {
                    true
                }
            })
            .filter(|record| query.include_deleted || !record.is_deleted())
            .filter(|record| {
                let Some(search_query) = query.search_query.as_ref() else {
                    return true;
                };
                let normalized_query = search_query.trim().to_lowercase();
                if normalized_query.is_empty() {
                    return true;
                }

                let description = record.description.as_deref().unwrap_or("");
                record
                    .agent_id
                    .to_lowercase()
                    .contains(normalized_query.as_str())
                    || record
                        .code
                        .to_lowercase()
                        .contains(normalized_query.as_str())
                    || record
                        .display_name
                        .to_lowercase()
                        .contains(normalized_query.as_str())
                    || description
                        .to_lowercase()
                        .contains(normalized_query.as_str())
            })
            .cloned()
            .collect()
    }

    fn insert_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        if self.provider_bindings.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.agent_id == record.agent_id
                && existing.binding_id == record.binding_id
        }) {
            return Err(KernelError::conflict(
                "agent provider binding already exists",
            ));
        }
        if record.active
            && self.provider_bindings.iter().any(|existing| {
                existing.tenant_id == record.tenant_id
                    && existing.agent_id == record.agent_id
                    && existing.active
            })
        {
            return Err(KernelError::conflict(
                "active provider binding already exists",
            ));
        }
        self.provider_bindings.push(record);
        Ok(())
    }

    fn update_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        let Some(index) = self.provider_bindings.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.agent_id == record.agent_id
                && existing.binding_id == record.binding_id
        }) else {
            return Err(KernelError::validation("agent provider binding not found"));
        };
        let expected_version = self.provider_bindings[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "provider binding version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if record.active
            && self
                .provider_bindings
                .iter()
                .enumerate()
                .any(|(current, existing)| {
                    current != index
                        && existing.tenant_id == record.tenant_id
                        && existing.agent_id == record.agent_id
                        && existing.active
                })
        {
            return Err(KernelError::conflict(
                "active provider binding already exists",
            ));
        }
        self.provider_bindings[index] = record;
        Ok(())
    }

    fn get_provider_binding(
        &self,
        tenant_id: u64,
        agent_id: &str,
        binding_id: &str,
    ) -> Option<AgentProviderBindingRecord> {
        self.provider_bindings
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id
                    && record.agent_id == agent_id
                    && record.binding_id == binding_id
            })
            .cloned()
    }

    fn list_provider_bindings(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> Vec<AgentProviderBindingRecord> {
        let mut records: Vec<AgentProviderBindingRecord> = self
            .provider_bindings
            .iter()
            .filter(|record| record.tenant_id == tenant_id && record.agent_id == agent_id)
            .cloned()
            .collect();
        records.sort_by(compare_provider_bindings_standard_order);
        records
    }

    fn insert_deployment(&mut self, record: AgentDeploymentRecord) -> KernelResult<()> {
        if self.deployments.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.agent_id == record.agent_id
                && existing.deployment_id == record.deployment_id
        }) {
            return Err(KernelError::conflict("agent deployment already exists"));
        }
        self.deployments.push(record);
        Ok(())
    }

    fn list_deployments(&self, tenant_id: u64, agent_id: &str) -> Vec<AgentDeploymentRecord> {
        let mut records: Vec<AgentDeploymentRecord> = self
            .deployments
            .iter()
            .filter(|record| record.tenant_id == tenant_id && record.agent_id == agent_id)
            .cloned()
            .collect();
        records.sort_by(compare_deployments_standard_order);
        records
    }

    fn insert_mcp_server(&mut self, record: AgentMcpServerRecord) -> KernelResult<()> {
        if self.mcp_servers.iter().any(|existing| {
            existing.tenant_id == record.tenant_id && existing.mcp_server_id == record.mcp_server_id
        }) {
            return Err(KernelError::conflict("agent mcp server already exists"));
        }
        if self
            .mcp_servers
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.code == record.code)
        {
            return Err(KernelError::conflict(
                "agent mcp server code already exists",
            ));
        }
        self.mcp_servers.push(record);
        Ok(())
    }

    fn update_mcp_server(&mut self, record: AgentMcpServerRecord) -> KernelResult<()> {
        let Some(index) = self.mcp_servers.iter().position(|existing| {
            existing.tenant_id == record.tenant_id && existing.mcp_server_id == record.mcp_server_id
        }) else {
            return Err(KernelError::validation("agent mcp server not found"));
        };
        let expected_version = self.mcp_servers[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent mcp server version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if self
            .mcp_servers
            .iter()
            .enumerate()
            .any(|(current, existing)| {
                current != index
                    && existing.tenant_id == record.tenant_id
                    && existing.code == record.code
            })
        {
            return Err(KernelError::conflict(
                "agent mcp server code already exists",
            ));
        }
        self.mcp_servers[index] = record;
        Ok(())
    }

    fn get_mcp_server(&self, tenant_id: u64, mcp_server_id: &str) -> Option<AgentMcpServerRecord> {
        self.mcp_servers
            .iter()
            .find(|record| record.tenant_id == tenant_id && record.mcp_server_id == mcp_server_id)
            .cloned()
    }

    fn list_mcp_servers(&self, query: &AgentMarketplaceListQuery) -> Vec<AgentMcpServerRecord> {
        let mut records: Vec<AgentMcpServerRecord> = self
            .mcp_servers
            .iter()
            .filter(|record| {
                marketplace_record_matches(
                    query,
                    MarketplaceRecordView {
                        tenant_id: record.tenant_id,
                        organization_id: record.organization_id,
                        owner_user_id: record.owner_user_id,
                        status: record.status,
                        visibility: record.visibility,
                        deleted: record.is_deleted(),
                        id: record.mcp_server_id.as_str(),
                        code: record.code.as_str(),
                        display_name: record.display_name.as_str(),
                        description: record.description.as_deref(),
                        categories: record.categories.as_slice(),
                        tags: record.tags.as_slice(),
                    },
                )
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            compare_marketplace_standard_order(
                left.updated_at.as_str(),
                left.code.as_str(),
                right.updated_at.as_str(),
                right.code.as_str(),
            )
        });
        records
    }

    fn insert_knowledge_base(&mut self, record: AgentKnowledgeBaseRecord) -> KernelResult<()> {
        if self.knowledge_bases.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_base_id == record.knowledge_base_id
        }) {
            return Err(KernelError::conflict("agent knowledge base already exists"));
        }
        if self
            .knowledge_bases
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.code == record.code)
        {
            return Err(KernelError::conflict(
                "agent knowledge base code already exists",
            ));
        }
        self.knowledge_bases.push(record);
        Ok(())
    }

    fn update_knowledge_base(&mut self, record: AgentKnowledgeBaseRecord) -> KernelResult<()> {
        let Some(index) = self.knowledge_bases.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_base_id == record.knowledge_base_id
        }) else {
            return Err(KernelError::validation("agent knowledge base not found"));
        };
        let expected_version = self.knowledge_bases[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent knowledge base version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if self
            .knowledge_bases
            .iter()
            .enumerate()
            .any(|(current, existing)| {
                current != index
                    && existing.tenant_id == record.tenant_id
                    && existing.code == record.code
            })
        {
            return Err(KernelError::conflict(
                "agent knowledge base code already exists",
            ));
        }
        self.knowledge_bases[index] = record;
        Ok(())
    }

    fn get_knowledge_base(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Option<AgentKnowledgeBaseRecord> {
        self.knowledge_bases
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.knowledge_base_id == knowledge_base_id
            })
            .cloned()
    }

    fn list_knowledge_bases(
        &self,
        query: &AgentMarketplaceListQuery,
    ) -> Vec<AgentKnowledgeBaseRecord> {
        let mut records: Vec<AgentKnowledgeBaseRecord> = self
            .knowledge_bases
            .iter()
            .filter(|record| {
                marketplace_record_matches(
                    query,
                    MarketplaceRecordView {
                        tenant_id: record.tenant_id,
                        organization_id: record.organization_id,
                        owner_user_id: record.owner_user_id,
                        status: record.status,
                        visibility: record.visibility,
                        deleted: record.is_deleted(),
                        id: record.knowledge_base_id.as_str(),
                        code: record.code.as_str(),
                        display_name: record.display_name.as_str(),
                        description: record.description.as_deref(),
                        categories: &[],
                        tags: &[],
                    },
                )
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            compare_marketplace_standard_order(
                left.updated_at.as_str(),
                left.code.as_str(),
                right.updated_at.as_str(),
                right.code.as_str(),
            )
        });
        records
    }

    fn insert_knowledge_source(&mut self, record: AgentKnowledgeSourceRecord) -> KernelResult<()> {
        if self.knowledge_sources.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_source_id == record.knowledge_source_id
        }) {
            return Err(KernelError::conflict(
                "agent knowledge source already exists",
            ));
        }
        self.knowledge_sources.push(record);
        Ok(())
    }

    fn update_knowledge_source(&mut self, record: AgentKnowledgeSourceRecord) -> KernelResult<()> {
        let Some(index) = self.knowledge_sources.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_source_id == record.knowledge_source_id
        }) else {
            return Err(KernelError::validation("agent knowledge source not found"));
        };
        let expected_version = self.knowledge_sources[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent knowledge source version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        self.knowledge_sources[index] = record;
        Ok(())
    }

    fn get_knowledge_source(
        &self,
        tenant_id: u64,
        knowledge_source_id: &str,
    ) -> Option<AgentKnowledgeSourceRecord> {
        self.knowledge_sources
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.knowledge_source_id == knowledge_source_id
            })
            .cloned()
    }

    fn list_knowledge_sources(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeSourceRecord> {
        let mut records: Vec<AgentKnowledgeSourceRecord> = self
            .knowledge_sources
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && record.knowledge_base_id == knowledge_base_id
                    && !record.is_deleted()
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.knowledge_source_id.cmp(&right.knowledge_source_id))
        });
        records
    }

    fn insert_knowledge_document(
        &mut self,
        record: AgentKnowledgeDocumentRecord,
    ) -> KernelResult<()> {
        if self.knowledge_documents.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_document_id == record.knowledge_document_id
        }) {
            return Err(KernelError::conflict(
                "agent knowledge document already exists",
            ));
        }
        self.knowledge_documents.push(record);
        Ok(())
    }

    fn update_knowledge_document(
        &mut self,
        record: AgentKnowledgeDocumentRecord,
    ) -> KernelResult<()> {
        let Some(index) = self.knowledge_documents.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_document_id == record.knowledge_document_id
        }) else {
            return Err(KernelError::validation(
                "agent knowledge document not found",
            ));
        };
        let expected_version = self.knowledge_documents[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent knowledge document version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        self.knowledge_documents[index] = record;
        Ok(())
    }

    fn get_knowledge_document(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> Option<AgentKnowledgeDocumentRecord> {
        self.knowledge_documents
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id
                    && record.knowledge_document_id == knowledge_document_id
            })
            .cloned()
    }

    fn list_knowledge_documents(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeDocumentRecord> {
        let mut records: Vec<AgentKnowledgeDocumentRecord> = self
            .knowledge_documents
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && record.knowledge_base_id == knowledge_base_id
                    && !record.is_deleted()
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.knowledge_document_id.cmp(&right.knowledge_document_id))
        });
        records
    }

    fn insert_knowledge_chunk(&mut self, record: AgentKnowledgeChunkRecord) -> KernelResult<()> {
        if self.knowledge_chunks.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_chunk_id == record.knowledge_chunk_id
        }) {
            return Err(KernelError::conflict(
                "agent knowledge chunk already exists",
            ));
        }
        if self.knowledge_chunks.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_document_id == record.knowledge_document_id
                && existing.chunk_ordinal == record.chunk_ordinal
        }) {
            return Err(KernelError::conflict(
                "agent knowledge chunk ordinal already exists",
            ));
        }
        if let Some(index) = self.knowledge_documents.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_document_id == record.knowledge_document_id
        }) {
            self.knowledge_documents[index].chunk_count = self.knowledge_documents[index]
                .chunk_count
                .saturating_add(1);
            self.knowledge_documents[index].version =
                self.knowledge_documents[index].version.saturating_add(1);
        }
        self.knowledge_chunks.push(record);
        Ok(())
    }

    fn get_knowledge_chunk(
        &self,
        tenant_id: u64,
        knowledge_chunk_id: &str,
    ) -> Option<AgentKnowledgeChunkRecord> {
        self.knowledge_chunks
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.knowledge_chunk_id == knowledge_chunk_id
            })
            .cloned()
    }

    fn list_knowledge_chunks(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> Vec<AgentKnowledgeChunkRecord> {
        let mut records: Vec<AgentKnowledgeChunkRecord> = self
            .knowledge_chunks
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && record.knowledge_document_id == knowledge_document_id
                    && record.status != crate::domain::AgentBusinessStatus::Deleted
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            left.chunk_ordinal
                .cmp(&right.chunk_ordinal)
                .then_with(|| left.knowledge_chunk_id.cmp(&right.knowledge_chunk_id))
        });
        records
    }

    fn upsert_knowledge_index(&mut self, record: AgentKnowledgeIndexRecord) -> KernelResult<()> {
        if let Some(index) = self.knowledge_indexes.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_index_id == record.knowledge_index_id
        }) {
            self.knowledge_indexes[index] = record;
        } else {
            self.knowledge_indexes.push(record);
        }
        Ok(())
    }

    fn get_knowledge_index(
        &self,
        tenant_id: u64,
        knowledge_index_id: &str,
    ) -> Option<AgentKnowledgeIndexRecord> {
        self.knowledge_indexes
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.knowledge_index_id == knowledge_index_id
            })
            .cloned()
    }

    fn list_knowledge_indexes(
        &self,
        tenant_id: u64,
        knowledge_document_id: &str,
    ) -> Vec<AgentKnowledgeIndexRecord> {
        let mut records: Vec<AgentKnowledgeIndexRecord> = self
            .knowledge_indexes
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && record.knowledge_document_id.as_deref() == Some(knowledge_document_id)
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .indexed_at
                .cmp(&left.indexed_at)
                .then_with(|| left.knowledge_index_id.cmp(&right.knowledge_index_id))
        });
        records
    }

    fn list_knowledge_indexes_by_base(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeIndexRecord> {
        let mut records: Vec<AgentKnowledgeIndexRecord> = self
            .knowledge_indexes
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && record.knowledge_base_id == knowledge_base_id
                    && record.status != crate::domain::AgentBusinessStatus::Deleted
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .indexed_at
                .cmp(&left.indexed_at)
                .then_with(|| left.knowledge_index_id.cmp(&right.knowledge_index_id))
        });
        records
    }

    fn insert_knowledge_binding(
        &mut self,
        record: AgentKnowledgeBindingRecord,
    ) -> KernelResult<()> {
        validate_knowledge_binding_scope_invariant(&record)?;
        if self.knowledge_bindings.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.knowledge_binding_id == record.knowledge_binding_id
        }) {
            return Err(KernelError::conflict(
                "agent knowledge binding already exists",
            ));
        }
        if record.active
            && record.default_binding
            && self.knowledge_bindings.iter().any(|existing| {
                existing.tenant_id == record.tenant_id
                    && existing.knowledge_base_id == record.knowledge_base_id
                    && existing.scope_kind == record.scope_kind
                    && existing.scope_ref == record.scope_ref
                    && existing.active
                    && existing.default_binding
            })
        {
            return Err(KernelError::conflict(
                "active default knowledge binding already exists",
            ));
        }
        self.knowledge_bindings.push(record);
        Ok(())
    }

    fn get_knowledge_binding(
        &self,
        tenant_id: u64,
        knowledge_binding_id: &str,
    ) -> Option<AgentKnowledgeBindingRecord> {
        self.knowledge_bindings
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.knowledge_binding_id == knowledge_binding_id
            })
            .cloned()
    }

    fn list_knowledge_bindings(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeBindingRecord> {
        let mut records: Vec<AgentKnowledgeBindingRecord> = self
            .knowledge_bindings
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id && record.knowledge_base_id == knowledge_base_id
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .active
                .cmp(&left.active)
                .then_with(|| right.default_binding.cmp(&left.default_binding))
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.knowledge_binding_id.cmp(&right.knowledge_binding_id))
        });
        records
    }

    fn insert_knowledge_sync_job(
        &mut self,
        record: AgentKnowledgeSyncJobRecord,
    ) -> KernelResult<()> {
        if self.knowledge_sync_jobs.iter().any(|existing| {
            existing.tenant_id == record.tenant_id && existing.sync_job_id == record.sync_job_id
        }) {
            return Err(KernelError::conflict(
                "agent knowledge sync job already exists",
            ));
        }
        self.knowledge_sync_jobs.push(record);
        Ok(())
    }

    fn update_knowledge_sync_job(
        &mut self,
        record: AgentKnowledgeSyncJobRecord,
    ) -> KernelResult<()> {
        let Some(index) = self.knowledge_sync_jobs.iter().position(|existing| {
            existing.tenant_id == record.tenant_id && existing.sync_job_id == record.sync_job_id
        }) else {
            return Err(KernelError::validation(
                "agent knowledge sync job not found",
            ));
        };
        self.knowledge_sync_jobs[index] = record;
        Ok(())
    }

    fn get_knowledge_sync_job(
        &self,
        tenant_id: u64,
        sync_job_id: &str,
    ) -> Option<AgentKnowledgeSyncJobRecord> {
        self.knowledge_sync_jobs
            .iter()
            .find(|record| record.tenant_id == tenant_id && record.sync_job_id == sync_job_id)
            .cloned()
    }

    fn list_knowledge_sync_jobs(
        &self,
        tenant_id: u64,
        knowledge_base_id: &str,
    ) -> Vec<AgentKnowledgeSyncJobRecord> {
        let mut records: Vec<AgentKnowledgeSyncJobRecord> = self
            .knowledge_sync_jobs
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id && record.knowledge_base_id == knowledge_base_id
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.sync_job_id.cmp(&right.sync_job_id))
        });
        records
    }

    fn insert_memory_store(&mut self, record: AgentMemoryStoreRecord) -> KernelResult<()> {
        if self.memory_stores.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_store_id == record.memory_store_id
        }) {
            return Err(KernelError::conflict("agent memory store already exists"));
        }
        if self
            .memory_stores
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.code == record.code)
        {
            return Err(KernelError::conflict(
                "agent memory store code already exists",
            ));
        }
        self.memory_stores.push(record);
        Ok(())
    }

    fn update_memory_store(&mut self, record: AgentMemoryStoreRecord) -> KernelResult<()> {
        let Some(index) = self.memory_stores.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_store_id == record.memory_store_id
        }) else {
            return Err(KernelError::validation("agent memory store not found"));
        };
        let expected_version = self.memory_stores[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent memory store version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        self.memory_stores[index] = record;
        Ok(())
    }

    fn get_memory_store(
        &self,
        tenant_id: u64,
        memory_store_id: &str,
    ) -> Option<AgentMemoryStoreRecord> {
        self.memory_stores
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_store_id == memory_store_id
            })
            .cloned()
    }

    fn insert_memory_profile(&mut self, record: AgentMemoryProfileRecord) -> KernelResult<()> {
        if self.memory_profiles.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_profile_id == record.memory_profile_id
        }) {
            return Err(KernelError::conflict("agent memory profile already exists"));
        }
        if self
            .memory_profiles
            .iter()
            .any(|existing| existing.tenant_id == record.tenant_id && existing.code == record.code)
        {
            return Err(KernelError::conflict(
                "agent memory profile code already exists",
            ));
        }
        self.memory_profiles.push(record);
        Ok(())
    }

    fn get_memory_profile(
        &self,
        tenant_id: u64,
        memory_profile_id: &str,
    ) -> Option<AgentMemoryProfileRecord> {
        self.memory_profiles
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_profile_id == memory_profile_id
            })
            .cloned()
    }

    fn insert_memory_binding(&mut self, record: AgentMemoryBindingRecord) -> KernelResult<()> {
        validate_memory_binding_scope_invariant(&record)?;
        if self.memory_bindings.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_binding_id == record.memory_binding_id
        }) {
            return Err(KernelError::conflict("agent memory binding already exists"));
        }
        if record.active
            && record.default_binding
            && self.memory_bindings.iter().any(|existing| {
                existing.tenant_id == record.tenant_id
                    && existing.memory_profile_id == record.memory_profile_id
                    && existing.scope_kind == record.scope_kind
                    && existing.scope_ref == record.scope_ref
                    && existing.active
                    && existing.default_binding
            })
        {
            return Err(KernelError::conflict(
                "active default memory binding already exists",
            ));
        }
        self.memory_bindings.push(record);
        Ok(())
    }

    fn get_memory_binding(
        &self,
        tenant_id: u64,
        memory_binding_id: &str,
    ) -> Option<AgentMemoryBindingRecord> {
        self.memory_bindings
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_binding_id == memory_binding_id
            })
            .cloned()
    }

    fn insert_memory_namespace(&mut self, record: AgentMemoryNamespaceRecord) -> KernelResult<()> {
        if self.memory_namespaces.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_namespace_id == record.memory_namespace_id
        }) {
            return Err(KernelError::conflict(
                "agent memory namespace already exists",
            ));
        }
        self.memory_namespaces.push(record);
        Ok(())
    }

    fn get_memory_namespace(
        &self,
        tenant_id: u64,
        memory_namespace_id: &str,
    ) -> Option<AgentMemoryNamespaceRecord> {
        self.memory_namespaces
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_namespace_id == memory_namespace_id
            })
            .cloned()
    }

    fn insert_memory_record(&mut self, record: AgentMemoryRecord) -> KernelResult<()> {
        if self.memory_records.iter().any(|existing| {
            existing.tenant_id == record.tenant_id && existing.memory_id == record.memory_id
        }) {
            return Err(KernelError::conflict("agent memory record already exists"));
        }
        self.memory_records.push(record);
        Ok(())
    }

    fn update_memory_record(&mut self, record: AgentMemoryRecord) -> KernelResult<()> {
        let Some(index) = self.memory_records.iter().position(|existing| {
            existing.tenant_id == record.tenant_id && existing.memory_id == record.memory_id
        }) else {
            return Err(KernelError::validation("agent memory record not found"));
        };
        let expected_version = self.memory_records[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent memory record version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        self.memory_records[index] = record;
        Ok(())
    }

    fn get_memory_record(&self, tenant_id: u64, memory_id: &str) -> Option<AgentMemoryRecord> {
        self.memory_records
            .iter()
            .find(|record| record.tenant_id == tenant_id && record.memory_id == memory_id)
            .cloned()
    }

    fn list_memory_records(
        &self,
        tenant_id: u64,
        memory_namespace_id: &str,
    ) -> Vec<AgentMemoryRecord> {
        let mut records: Vec<AgentMemoryRecord> = self
            .memory_records
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && record.memory_namespace_id == memory_namespace_id
                    && !record.is_deleted()
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.memory_id.cmp(&right.memory_id))
        });
        records
    }

    fn insert_memory_source(&mut self, record: AgentMemorySourceRecord) -> KernelResult<()> {
        if self.memory_sources.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_source_id == record.memory_source_id
        }) {
            return Err(KernelError::conflict("agent memory source already exists"));
        }
        if let Some(index) = self.memory_records.iter().position(|existing| {
            existing.tenant_id == record.tenant_id && existing.memory_id == record.memory_id
        }) {
            self.memory_records[index].source_count =
                self.memory_records[index].source_count.saturating_add(1);
        }
        self.memory_sources.push(record);
        Ok(())
    }

    fn list_memory_sources(&self, tenant_id: u64, memory_id: &str) -> Vec<AgentMemorySourceRecord> {
        let mut results: Vec<AgentMemorySourceRecord> = self
            .memory_sources
            .iter()
            .filter(|record| record.tenant_id == tenant_id && record.memory_id == memory_id)
            .cloned()
            .collect();
        results.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.memory_source_id.cmp(&b.memory_source_id))
        });
        results
    }

    fn insert_memory_relation(&mut self, record: AgentMemoryRelationRecord) -> KernelResult<()> {
        if self.memory_relations.iter().any(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_relation_id == record.memory_relation_id
        }) {
            return Err(KernelError::conflict(
                "agent memory relation already exists",
            ));
        }
        self.memory_relations.push(record);
        Ok(())
    }

    fn list_memory_relations(
        &self,
        tenant_id: u64,
        memory_id: &str,
    ) -> Vec<AgentMemoryRelationRecord> {
        let mut results: Vec<AgentMemoryRelationRecord> = self
            .memory_relations
            .iter()
            .filter(|record| {
                record.tenant_id == tenant_id
                    && (record.from_memory_id == memory_id || record.to_memory_id == memory_id)
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.memory_relation_id.cmp(&b.memory_relation_id))
        });
        results
    }

    fn upsert_memory_retrieval_index(
        &mut self,
        record: AgentMemoryRetrievalIndexRecord,
    ) -> KernelResult<()> {
        if let Some(index) = self.memory_retrieval_indexes.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_index_id == record.memory_index_id
        }) {
            self.memory_retrieval_indexes[index] = record;
        } else {
            self.memory_retrieval_indexes.push(record);
        }
        Ok(())
    }

    fn list_memory_retrieval_indexes(
        &self,
        tenant_id: u64,
        memory_id: &str,
    ) -> Vec<AgentMemoryRetrievalIndexRecord> {
        self.memory_retrieval_indexes
            .iter()
            .filter(|record| record.tenant_id == tenant_id && record.memory_id == memory_id)
            .cloned()
            .collect()
    }

    fn list_memory_stores(&self, tenant_id: u64) -> Vec<AgentMemoryStoreRecord> {
        let mut records: Vec<AgentMemoryStoreRecord> = self
            .memory_stores
            .iter()
            .filter(|record| record.tenant_id == tenant_id)
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.code.cmp(&right.code))
        });
        records
    }

    fn update_memory_profile(&mut self, record: AgentMemoryProfileRecord) -> KernelResult<()> {
        let Some(index) = self.memory_profiles.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_profile_id == record.memory_profile_id
        }) else {
            return Err(KernelError::validation("agent memory profile not found"));
        };
        let expected_version = self.memory_profiles[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent memory profile version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if self
            .memory_profiles
            .iter()
            .enumerate()
            .any(|(current, existing)| {
                current != index
                    && existing.tenant_id == record.tenant_id
                    && existing.code == record.code
            })
        {
            return Err(KernelError::conflict(
                "agent memory profile code already exists",
            ));
        }
        self.memory_profiles[index] = record;
        Ok(())
    }

    fn update_memory_binding(&mut self, record: AgentMemoryBindingRecord) -> KernelResult<()> {
        validate_memory_binding_scope_invariant(&record)?;
        let Some(index) = self.memory_bindings.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_binding_id == record.memory_binding_id
        }) else {
            return Err(KernelError::validation("agent memory binding not found"));
        };
        let expected_version = self.memory_bindings[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent memory binding version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        if record.active
            && record.default_binding
            && self
                .memory_bindings
                .iter()
                .enumerate()
                .any(|(current, existing)| {
                    current != index
                        && existing.tenant_id == record.tenant_id
                        && existing.memory_profile_id == record.memory_profile_id
                        && existing.scope_kind == record.scope_kind
                        && existing.scope_ref == record.scope_ref
                        && existing.active
                        && existing.default_binding
                })
        {
            return Err(KernelError::conflict(
                "active default memory binding already exists",
            ));
        }
        self.memory_bindings[index] = record;
        Ok(())
    }

    fn update_memory_namespace(&mut self, record: AgentMemoryNamespaceRecord) -> KernelResult<()> {
        let Some(index) = self.memory_namespaces.iter().position(|existing| {
            existing.tenant_id == record.tenant_id
                && existing.memory_namespace_id == record.memory_namespace_id
        }) else {
            return Err(KernelError::validation("agent memory namespace not found"));
        };
        let expected_version = self.memory_namespaces[index].version.saturating_add(1);
        if record.version != expected_version {
            return Err(KernelError::conflict(format!(
                "agent memory namespace version mismatch: expected={expected_version}, actual={}",
                record.version
            )));
        }
        self.memory_namespaces[index] = record;
        Ok(())
    }

    fn get_memory_source(
        &self,
        tenant_id: u64,
        memory_source_id: &str,
    ) -> Option<AgentMemorySourceRecord> {
        self.memory_sources
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_source_id == memory_source_id
            })
            .cloned()
    }

    fn get_memory_relation(
        &self,
        tenant_id: u64,
        memory_relation_id: &str,
    ) -> Option<AgentMemoryRelationRecord> {
        self.memory_relations
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_relation_id == memory_relation_id
            })
            .cloned()
    }

    fn get_memory_retrieval_index(
        &self,
        tenant_id: u64,
        retrieval_index_id: &str,
    ) -> Option<AgentMemoryRetrievalIndexRecord> {
        self.memory_retrieval_indexes
            .iter()
            .find(|record| {
                record.tenant_id == tenant_id && record.memory_index_id == retrieval_index_id
            })
            .cloned()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemoryAgentAuditSink {
    events: Vec<KernelEvent>,
}

impl InMemoryAgentAuditSink {
    pub fn events(&self) -> &[KernelEvent] {
        &self.events
    }
}

impl AgentAuditSink for InMemoryAgentAuditSink {
    fn record(&mut self, event: KernelEvent) -> KernelResult<()> {
        self.events.push(event);
        Ok(())
    }

    fn list_events(&self, tenant_id: u64, agent_id: &str) -> KernelResult<Vec<KernelEvent>> {
        let tenant_pattern = format!("tenant_id={tenant_id};");
        let agent_pattern = format!("agent_id={agent_id};");
        let mut events: Vec<KernelEvent> = self
            .events
            .iter()
            .filter(|event| {
                event.payload.contains(tenant_pattern.as_str())
                    && event.payload.contains(agent_pattern.as_str())
            })
            .cloned()
            .collect();

        events.sort_by(compare_audit_events_desc);
        Ok(events)
    }
}

fn compare_audit_events_desc(left: &KernelEvent, right: &KernelEvent) -> Ordering {
    let left_time = parse_occurred_at(left.occurred_at.as_deref());
    let right_time = parse_occurred_at(right.occurred_at.as_deref());

    right_time
        .cmp(&left_time)
        .then_with(|| right.event_id.cmp(&left.event_id))
}

fn compare_provider_bindings_standard_order(
    left: &AgentProviderBindingRecord,
    right: &AgentProviderBindingRecord,
) -> Ordering {
    right
        .active
        .cmp(&left.active)
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| left.binding_id.cmp(&right.binding_id))
}

fn compare_deployments_standard_order(
    left: &AgentDeploymentRecord,
    right: &AgentDeploymentRecord,
) -> Ordering {
    right
        .created_at
        .cmp(&left.created_at)
        .then_with(|| left.deployment_id.cmp(&right.deployment_id))
}

struct MarketplaceRecordView<'a> {
    tenant_id: u64,
    organization_id: u64,
    owner_user_id: u64,
    status: crate::domain::AgentBusinessStatus,
    visibility: crate::domain::AgentVisibility,
    deleted: bool,
    id: &'a str,
    code: &'a str,
    display_name: &'a str,
    description: Option<&'a str>,
    categories: &'a [String],
    tags: &'a [String],
}

fn marketplace_record_matches(
    query: &AgentMarketplaceListQuery,
    record: MarketplaceRecordView<'_>,
) -> bool {
    if record.tenant_id != query.tenant_id {
        return false;
    }
    if let Some(organization_id) = query.organization_id {
        if record.organization_id != organization_id {
            return false;
        }
    }
    if let Some(owner_user_id) = query.owner_user_id {
        if record.owner_user_id != owner_user_id {
            return false;
        }
    }
    if let Some(status) = query.status {
        if record.status != status {
            return false;
        }
    }
    if let Some(visibility) = query.visibility {
        if record.visibility != visibility {
            return false;
        }
    }
    if !query.include_deleted && record.deleted {
        return false;
    }
    if let Some(category) = query.category.as_ref() {
        if !record.categories.iter().any(|value| value == category) {
            return false;
        }
    }
    if let Some(tag) = query.tag.as_ref() {
        if !record.tags.iter().any(|value| value == tag) {
            return false;
        }
    }
    if let Some(search_query) = query.search_query.as_ref() {
        let normalized_query = search_query.trim().to_lowercase();
        if normalized_query.is_empty() {
            return true;
        }
        let description = record.description.unwrap_or("");
        return record.id.to_lowercase().contains(normalized_query.as_str())
            || record
                .code
                .to_lowercase()
                .contains(normalized_query.as_str())
            || record
                .display_name
                .to_lowercase()
                .contains(normalized_query.as_str())
            || description
                .to_lowercase()
                .contains(normalized_query.as_str());
    }
    true
}

fn compare_marketplace_standard_order(
    left_updated_at: &str,
    left_code: &str,
    right_updated_at: &str,
    right_code: &str,
) -> Ordering {
    right_updated_at
        .cmp(left_updated_at)
        .then_with(|| left_code.cmp(right_code))
}

fn parse_occurred_at(value: Option<&str>) -> Option<OffsetDateTime> {
    let value = value?;
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

fn validate_knowledge_binding_scope_invariant(
    record: &AgentKnowledgeBindingRecord,
) -> KernelResult<()> {
    match record.scope_kind {
        AgentKnowledgeBindingScopeKind::Agent => {
            let Some(agent_id) = record.agent_id.as_deref() else {
                return Err(KernelError::validation(
                    "agentId is required for agent knowledge binding scope",
                ));
            };
            if record.scope_ref != agent_id {
                return Err(KernelError::validation(
                    "scopeRef must match agentId for agent knowledge binding scope",
                ));
            }
        }
        AgentKnowledgeBindingScopeKind::Deployment => {
            if record.agent_id.is_none() {
                return Err(KernelError::validation(
                    "agentId is required for deployment knowledge binding scope",
                ));
            }
            let Some(deployment_id) = record.deployment_id.as_deref() else {
                return Err(KernelError::validation(
                    "deploymentId is required for deployment knowledge binding scope",
                ));
            };
            if record.scope_ref != deployment_id {
                return Err(KernelError::validation(
                    "scopeRef must match deploymentId for deployment knowledge binding scope",
                ));
            }
        }
        AgentKnowledgeBindingScopeKind::User
        | AgentKnowledgeBindingScopeKind::Session
        | AgentKnowledgeBindingScopeKind::Organization
        | AgentKnowledgeBindingScopeKind::Tenant => {}
    }
    Ok(())
}

fn validate_memory_binding_scope_invariant(record: &AgentMemoryBindingRecord) -> KernelResult<()> {
    match record.scope_kind {
        AgentMemoryBindingScopeKind::Agent => {
            let Some(agent_id) = record.agent_id.as_deref() else {
                return Err(KernelError::validation(
                    "agentId is required for agent memory binding scope",
                ));
            };
            if record.scope_ref != agent_id {
                return Err(KernelError::validation(
                    "scopeRef must match agentId for agent memory binding scope",
                ));
            }
        }
        AgentMemoryBindingScopeKind::Deployment => {
            if record.agent_id.is_none() {
                return Err(KernelError::validation(
                    "agentId is required for deployment memory binding scope",
                ));
            }
            let Some(deployment_id) = record.deployment_id.as_deref() else {
                return Err(KernelError::validation(
                    "deploymentId is required for deployment memory binding scope",
                ));
            };
            if record.scope_ref != deployment_id {
                return Err(KernelError::validation(
                    "scopeRef must match deploymentId for deployment memory binding scope",
                ));
            }
        }
        AgentMemoryBindingScopeKind::User
        | AgentMemoryBindingScopeKind::Session
        | AgentMemoryBindingScopeKind::Organization
        | AgentMemoryBindingScopeKind::Tenant => {}
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyMode {
    Allow,
    Deny(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowAllPolicyProvider {
    pub provider_id: String,
    pub mode: PolicyMode,
}

impl AllowAllPolicyProvider {
    pub fn allow(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            mode: PolicyMode::Allow,
        }
    }

    pub fn deny(provider_id: impl Into<String>, reason_code: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            mode: PolicyMode::Deny(reason_code.into()),
        }
    }
}

impl PolicyProvider for AllowAllPolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        let decision_id = format!("decision_{}", request.policy_request_id);
        match &self.mode {
            PolicyMode::Allow => Ok(PolicyDecision::allow(
                decision_id,
                request.policy_request_id,
                self.provider_id.clone(),
            )),
            PolicyMode::Deny(reason) => Ok(PolicyDecision::deny(
                decision_id,
                request.policy_request_id,
                self.provider_id.clone(),
                reason.clone(),
            )),
        }
    }

    fn health(&self) -> sdkwork_agent_kernel::ProviderHealth {
        sdkwork_agent_kernel::ProviderHealth::available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AgentBusinessStatus, AgentImplementationKind, AgentImplementationType,
        AgentMemoryBindingRecord, AgentMemoryBindingScopeKind, AgentProviderBindingRecord,
        AgentVisibility,
    };
    use sdkwork_agent_kernel::AgentManifest;
    use sdkwork_agent_kernel::KernelErrorKind;

    fn sample_manifest(agent_id: &str) -> AgentManifest {
        AgentManifest {
            schema_version: "1.0.0".to_string(),
            manifest_type: "agent".to_string(),
            agent_id: agent_id.to_string(),
            name: "sample-agent".to_string(),
            display_name: "Sample Agent".to_string(),
            description: "sample".to_string(),
            version: "0.1.0".to_string(),
            domain: "intelligence".to_string(),
            required_capabilities: vec!["model.chat".to_string()],
            optional_capabilities: vec!["tool.invoke".to_string()],
            required_capability_requirements: vec![],
            optional_capability_requirements: vec![],
            event_families: vec!["agent.lifecycle".to_string()],
            owner_name: "sdkwork".to_string(),
            status: "active".to_string(),
        }
    }

    #[test]
    fn in_memory_repository_rejects_stale_record_version_update() {
        let mut repository = InMemoryAgentRepository::new();
        let record = AgentBusinessRecord {
            id: 1,
            agent_id: "agent.alpha".to_string(),
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: None,
            manifest: sample_manifest("agent.alpha"),
            default_code_task_intent: None,
            implementation_provider_id: None,
            implementation_kind: None,
            implementation_type: AgentImplementationType::SdkworkNative,
            status: AgentBusinessStatus::Draft,
            visibility: AgentVisibility::Organization,
            tags: vec!["starter".to_string()],
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
        };
        repository
            .insert(record.clone())
            .expect("initial insert should succeed");

        let mut stale = record.clone();
        stale.display_name = "Alpha stale".to_string();
        let error = repository
            .update(stale)
            .expect_err("stale version should fail");
        match error {
            KernelError::Structured { info } => {
                assert_eq!(info.kind.as_str(), "conflict");
                assert!(info.message.contains("version mismatch"));
            }
            _ => panic!("expected structured conflict"),
        }
    }

    #[test]
    fn in_memory_repository_rejects_stale_provider_binding_update() {
        let mut repository = InMemoryAgentRepository::new();
        let record = AgentProviderBindingRecord {
            id: 101,
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            active: true,
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        };
        repository
            .insert_provider_binding(record.clone())
            .expect("initial binding insert should succeed");

        let mut stale = record.clone();
        stale.provider_id = "provider.model.rig-alt".to_string();
        let error = repository
            .update_provider_binding(stale)
            .expect_err("stale binding version should fail");

        assert_eq!(error.kind(), KernelErrorKind::Conflict);
        assert!(error
            .message()
            .contains("provider binding version mismatch"));
    }

    #[test]
    fn in_memory_repository_rejects_second_active_provider_binding() {
        let mut repository = InMemoryAgentRepository::new();
        repository
            .insert_provider_binding(AgentProviderBindingRecord {
                id: 102,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.local".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:00:00Z".to_string(),
            })
            .expect("first active binding insert should succeed");

        let error = repository
            .insert_provider_binding(AgentProviderBindingRecord {
                id: 103,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alt".to_string(),
                provider_id: "provider.model.rig-alt".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alt".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:01:00Z".to_string(),
            })
            .expect_err("second active binding should fail");

        assert_eq!(error.kind(), KernelErrorKind::Conflict);
        assert!(error
            .message()
            .contains("active provider binding already exists"));
    }

    #[test]
    fn in_memory_repository_rejects_update_that_creates_second_active_provider_binding() {
        let mut repository = InMemoryAgentRepository::new();
        repository
            .insert_provider_binding(AgentProviderBindingRecord {
                id: 104,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.local".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:00:00Z".to_string(),
            })
            .expect("first active binding insert should succeed");
        repository
            .insert_provider_binding(AgentProviderBindingRecord {
                id: 105,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alt".to_string(),
                provider_id: "provider.model.rig-alt".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alt".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: false,
                version: 1,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:01:00Z".to_string(),
            })
            .expect("inactive binding insert should succeed");

        let error = repository
            .update_provider_binding(AgentProviderBindingRecord {
                id: 105,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alt".to_string(),
                provider_id: "provider.model.rig-alt".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alt".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 2,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            })
            .expect_err("update cannot create a second active binding");

        assert_eq!(error.kind(), KernelErrorKind::Conflict);
        assert!(error
            .message()
            .contains("active provider binding already exists"));
    }

    #[test]
    fn in_memory_repository_lists_provider_bindings_in_standard_order() {
        let mut repository = InMemoryAgentRepository::new();
        for record in [
            AgentProviderBindingRecord {
                id: 106,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.beta".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.beta".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: false,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
            AgentProviderBindingRecord {
                id: 107,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.default".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: true,
                version: 1,
                created_at: "2026-06-01T00:01:00Z".to_string(),
                updated_at: "2026-06-01T00:01:00Z".to_string(),
            },
            AgentProviderBindingRecord {
                id: 108,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                binding_id: "binding.rig.alpha".to_string(),
                provider_id: "provider.model.rig-rust".to_string(),
                implementation_kind: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id: "profile.rig.alpha".to_string(),
                capabilities: vec!["model.chat".to_string()],
                active: false,
                version: 1,
                created_at: "2026-06-01T00:00:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
        ] {
            repository
                .insert_provider_binding(record)
                .expect("binding insert should succeed");
        }

        let binding_ids: Vec<String> = repository
            .list_provider_bindings(1, "agent.alpha")
            .into_iter()
            .map(|record| record.binding_id)
            .collect();

        assert_eq!(
            binding_ids,
            vec![
                "binding.rig.default".to_string(),
                "binding.rig.alpha".to_string(),
                "binding.rig.beta".to_string()
            ]
        );
    }

    fn sample_knowledge_binding(
        knowledge_binding_id: &str,
        scope_ref: &str,
    ) -> AgentKnowledgeBindingRecord {
        AgentKnowledgeBindingRecord {
            id: 251,
            tenant_id: 1,
            organization_id: 10,
            knowledge_binding_id: knowledge_binding_id.to_string(),
            knowledge_base_id: "knowledge.base.default".to_string(),
            agent_id: Some("agent.alpha".to_string()),
            deployment_id: None,
            scope_kind: AgentKnowledgeBindingScopeKind::Agent,
            scope_ref: scope_ref.to_string(),
            active: true,
            default_binding: true,
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn in_memory_repository_enforces_knowledge_binding_storage_invariants() {
        let mut repository = InMemoryAgentRepository::new();

        let invalid_scope = repository
            .insert_knowledge_binding(sample_knowledge_binding(
                "knowledge.binding.agent.invalid_scope",
                "agent.beta",
            ))
            .expect_err("knowledge binding scopeRef must match agentId");
        assert_eq!(invalid_scope.kind(), KernelErrorKind::ValidationError);
        assert!(invalid_scope.message().contains("scopeRef"));

        repository
            .insert_knowledge_binding(sample_knowledge_binding(
                "knowledge.binding.agent.default",
                "agent.alpha",
            ))
            .expect("first default knowledge binding should insert");

        let duplicate_default = repository
            .insert_knowledge_binding(AgentKnowledgeBindingRecord {
                id: 252,
                knowledge_binding_id: "knowledge.binding.agent.duplicate_default".to_string(),
                ..sample_knowledge_binding(
                    "knowledge.binding.agent.duplicate_default",
                    "agent.alpha",
                )
            })
            .expect_err("second active default knowledge binding should fail");
        assert_eq!(duplicate_default.kind(), KernelErrorKind::Conflict);
        assert!(duplicate_default
            .message()
            .contains("active default knowledge binding already exists"));
    }

    fn sample_memory_binding(memory_binding_id: &str, scope_ref: &str) -> AgentMemoryBindingRecord {
        AgentMemoryBindingRecord {
            id: 301,
            tenant_id: 1,
            organization_id: 10,
            memory_binding_id: memory_binding_id.to_string(),
            memory_profile_id: "memory.profile.default".to_string(),
            agent_id: Some("agent.alpha".to_string()),
            deployment_id: None,
            scope_kind: AgentMemoryBindingScopeKind::Agent,
            scope_ref: scope_ref.to_string(),
            active: true,
            default_binding: true,
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn in_memory_repository_enforces_memory_binding_storage_invariants() {
        let mut repository = InMemoryAgentRepository::new();

        let invalid_scope = repository
            .insert_memory_binding(sample_memory_binding(
                "memory.binding.agent.invalid_scope",
                "agent.beta",
            ))
            .expect_err("memory binding scopeRef must match agentId");
        assert_eq!(invalid_scope.kind(), KernelErrorKind::ValidationError);
        assert!(invalid_scope.message().contains("scopeRef"));

        repository
            .insert_memory_binding(sample_memory_binding(
                "memory.binding.agent.default",
                "agent.alpha",
            ))
            .expect("first default memory binding should insert");

        let duplicate_default = repository
            .insert_memory_binding(AgentMemoryBindingRecord {
                id: 302,
                memory_binding_id: "memory.binding.agent.duplicate_default".to_string(),
                ..sample_memory_binding("memory.binding.agent.duplicate_default", "agent.alpha")
            })
            .expect_err("second active default memory binding should fail");
        assert_eq!(duplicate_default.kind(), KernelErrorKind::Conflict);
        assert!(duplicate_default
            .message()
            .contains("active default memory binding already exists"));
    }

    #[test]
    fn in_memory_repository_lists_deployments_in_standard_order() {
        let mut repository = InMemoryAgentRepository::new();
        for record in [
            AgentDeploymentRecord {
                id: 201,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                deployment_id: "deployment.rig.beta".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id_snapshot: "provider.model.rig-rust".to_string(),
                implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id_snapshot: "profile.rig.local".to_string(),
                capabilities_snapshot: vec!["model.chat".to_string()],
                status: crate::domain::AgentDeploymentStatus::Created,
                version: 1,
                created_at: "2026-06-01T00:02:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
            AgentDeploymentRecord {
                id: 202,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                deployment_id: "deployment.rig.latest".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id_snapshot: "provider.model.rig-rust".to_string(),
                implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id_snapshot: "profile.rig.local".to_string(),
                capabilities_snapshot: vec!["model.chat".to_string()],
                status: crate::domain::AgentDeploymentStatus::Created,
                version: 1,
                created_at: "2026-06-01T00:03:00Z".to_string(),
                updated_at: "2026-06-01T00:03:00Z".to_string(),
            },
            AgentDeploymentRecord {
                id: 203,
                tenant_id: 1,
                agent_id: "agent.alpha".to_string(),
                deployment_id: "deployment.rig.alpha".to_string(),
                binding_id: "binding.rig.default".to_string(),
                provider_id_snapshot: "provider.model.rig-rust".to_string(),
                implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
                configuration_profile_id_snapshot: "profile.rig.local".to_string(),
                capabilities_snapshot: vec!["model.chat".to_string()],
                status: crate::domain::AgentDeploymentStatus::Created,
                version: 1,
                created_at: "2026-06-01T00:02:00Z".to_string(),
                updated_at: "2026-06-01T00:02:00Z".to_string(),
            },
        ] {
            repository
                .insert_deployment(record)
                .expect("deployment insert should succeed");
        }

        let deployment_ids: Vec<String> = repository
            .list_deployments(1, "agent.alpha")
            .into_iter()
            .map(|record| record.deployment_id)
            .collect();

        assert_eq!(
            deployment_ids,
            vec![
                "deployment.rig.latest".to_string(),
                "deployment.rig.alpha".to_string(),
                "deployment.rig.beta".to_string()
            ]
        );
    }
}
