use sdkwork_agent_business::{
    AgentBusinessService, AgentBusinessStatus, AgentKnowledgeBaseCreateCommand,
    AgentKnowledgeBaseCreateRequestDto, AgentKnowledgeBaseKind, AgentKnowledgeBaseRecordDto,
    AgentKnowledgeBaseUpdateCommand, AgentKnowledgeBaseUpdateRequestDto,
    AgentKnowledgeBindingCreateCommand, AgentKnowledgeBindingCreateRequestDto,
    AgentKnowledgeBindingRecordDto, AgentKnowledgeBindingScopeKind,
    AgentKnowledgeChunkCreateCommand, AgentKnowledgeChunkCreateRequestDto,
    AgentKnowledgeChunkRecordDto, AgentKnowledgeDocumentCreateCommand,
    AgentKnowledgeDocumentCreateRequestDto, AgentKnowledgeDocumentKind,
    AgentKnowledgeDocumentProfileDto, AgentKnowledgeDocumentRecordDto,
    AgentKnowledgeDocumentUpdateCommand, AgentKnowledgeDocumentUpdateRequestDto,
    AgentKnowledgeIndexKind, AgentKnowledgeIndexRecordDto, AgentKnowledgeIndexUpsertCommand,
    AgentKnowledgeIndexUpsertRequestDto, AgentKnowledgeListCommand, AgentKnowledgeReadCommand,
    AgentKnowledgeSearchCommand, AgentKnowledgeSearchRequestDto, AgentKnowledgeSearchResultDto,
    AgentKnowledgeSourceCreateCommand, AgentKnowledgeSourceCreateRequestDto,
    AgentKnowledgeSourceKind, AgentKnowledgeSourceRecordDto, AgentKnowledgeSourceUpdateCommand,
    AgentKnowledgeSourceUpdateRequestDto, AgentKnowledgeSyncJobCancelCommand,
    AgentKnowledgeSyncJobCompleteCommand, AgentKnowledgeSyncJobCreateCommand,
    AgentKnowledgeSyncJobCreateRequestDto, AgentKnowledgeSyncJobFailCommand,
    AgentKnowledgeSyncJobKind, AgentKnowledgeSyncJobRecordDto, AgentKnowledgeSyncJobStartCommand,
    AgentKnowledgeSyncJobStatus, AgentVisibility, AllowAllPolicyProvider,
    DeleteAgentMarketplaceItemCommand, GetAgentMarketplaceItemCommand, InMemoryAgentAuditSink,
    InMemoryAgentRepository, ListAgentKnowledgeBasesRequestDto, RestoreAgentMarketplaceItemCommand,
};
use sdkwork_agent_kernel::{KernelErrorKind, PolicySubject};

fn subject() -> PolicySubject {
    PolicySubject::new("user.knowledge.admin", "tenant.1").with_role("agent.knowledge.admin")
}

fn service(
) -> AgentBusinessService<InMemoryAgentRepository, InMemoryAgentAuditSink, AllowAllPolicyProvider> {
    AgentBusinessService::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("provider.policy.knowledge"),
    )
}

fn knowledge_base_command(base_id: &str, code: &str) -> AgentKnowledgeBaseCreateCommand {
    AgentKnowledgeBaseCreateCommand {
        tenant_id: 1,
        organization_id: 10,
        owner_user_id: 100,
        knowledge_base_id: base_id.to_string(),
        code: code.to_string(),
        display_name: "Kernel Knowledge Base".to_string(),
        description: Some("Hybrid kernel knowledge base".to_string()),
        provider_id: "provider.knowledge.llm-wiki".to_string(),
        base_kind: AgentKnowledgeBaseKind::Hybrid,
        retrieval_modes: vec![
            AgentKnowledgeIndexKind::Keyword,
            AgentKnowledgeIndexKind::Wiki,
            AgentKnowledgeIndexKind::Graph,
            AgentKnowledgeIndexKind::Hybrid,
        ],
        capability_ids: vec![
            "knowledge.search".to_string(),
            "knowledge.read".to_string(),
            "knowledge.list".to_string(),
        ],
        configuration_profile_id: "profile.knowledge.kernel".to_string(),
        visibility: AgentVisibility::Organization,
        requested_by: subject(),
        requested_at: "2026-06-05T01:00:00Z".to_string(),
    }
}

#[test]
fn knowledge_dtos_are_public_agent_business_api_contracts() {
    fn assert_public_contract<T>() {}

    assert_public_contract::<ListAgentKnowledgeBasesRequestDto>();
    assert_public_contract::<AgentKnowledgeBaseCreateRequestDto>();
    assert_public_contract::<AgentKnowledgeBaseUpdateRequestDto>();
    assert_public_contract::<AgentKnowledgeSourceCreateRequestDto>();
    assert_public_contract::<AgentKnowledgeSourceUpdateRequestDto>();
    assert_public_contract::<AgentKnowledgeDocumentCreateRequestDto>();
    assert_public_contract::<AgentKnowledgeDocumentUpdateRequestDto>();
    assert_public_contract::<AgentKnowledgeChunkCreateRequestDto>();
    assert_public_contract::<AgentKnowledgeIndexUpsertRequestDto>();
    assert_public_contract::<AgentKnowledgeSearchRequestDto>();
    assert_public_contract::<AgentKnowledgeBindingCreateRequestDto>();
    assert_public_contract::<AgentKnowledgeSyncJobCreateRequestDto>();
    assert_public_contract::<AgentKnowledgeBaseRecordDto>();
    assert_public_contract::<AgentKnowledgeSourceRecordDto>();
    assert_public_contract::<AgentKnowledgeDocumentProfileDto>();
    assert_public_contract::<AgentKnowledgeDocumentRecordDto>();
    assert_public_contract::<AgentKnowledgeChunkRecordDto>();
    assert_public_contract::<AgentKnowledgeIndexRecordDto>();
    assert_public_contract::<AgentKnowledgeSearchResultDto>();
    assert_public_contract::<AgentKnowledgeBindingRecordDto>();
    assert_public_contract::<AgentKnowledgeSyncJobRecordDto>();
}

#[test]
fn knowledge_business_crud_surface_is_documented_as_current_contract() {
    let database_spec = include_str!("../specs/AGENT_BUSINESS_DATABASE_SPEC.md");
    let normalized_database_spec = database_spec
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for required in [
        "- Knowledge bases: create, update, get, list, search, soft-delete, restore.",
        "- Knowledge sources: create, update, get, list by knowledge base id, soft-delete, restore.",
        "- Knowledge documents: create, update, read/get, list by knowledge base id, soft-delete, restore.",
        "- Knowledge chunks: create, get, list by document id.",
        "- Knowledge indexes: upsert, get, list by document id, search by knowledge base id.",
        "- Knowledge bindings: create, get, list by knowledge base id.",
        "- Knowledge sync jobs: create, get, list by knowledge base id, start, complete, fail, cancel.",
        "Knowledge base, source, document, chunk, index, binding, and sync-job read/list/search paths use active-parent validation and hide soft-deleted parent resources.",
        "Knowledge source update/delete, document update/delete, and sync-job runtime transition paths also require active parent resources; restore paths validate parent resources before restoring soft-deleted children.",
        "Knowledge binding scope consistency is enforced: agent scopes require matching agentId/scopeRef, and deployment scopes require agentId plus matching deploymentId/scopeRef.",
        "Knowledge chunk, index, and binding update/delete/restore operations are not exposed until the schema adds the required lifecycle, version, timestamp, and audit semantics.",
        "RAG search is provider-neutral and may use exact, keyword, full-text, structured, graph, wiki, rule, vector, hybrid, LLM-rerank, or external retrieval; vector metadata is required only for vector indexes.",
        "Agent managementProfile is an API read projection derived from default_code_task_intent_json compatibility constraints and is not a separate physical column in this baseline.",
        "Agent managementProfile compatibility fields include author, avatar, categoryId, color, debugMode, iconName, jsonMode, knowledgeBaseIds, memoryEnabled, model, skillIds, suggestedPrompts, systemPrompt, temperature, toolIds, type, users, voiceIds, and welcomeMessage.",
        "Knowledge document documentProfile is an API read projection derived from metadata_json compatibility keys and is not a separate physical column in this baseline.",
        "If managementProfile or documentProfile fields become filter, sort, authorization, retention, or analytics keys, they must be promoted through an expand-backfill-contract migration with explicit columns and indexes.",
    ] {
        let normalized_required = required.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized_database_spec.contains(normalized_required.as_str()),
            "database spec must document current knowledge CRUD/RAG surface: {required}"
        );
    }
}

#[test]
fn knowledge_base_source_document_chunk_index_binding_and_sync_job_form_standard_rag_stack() {
    let mut service = service();

    let base = service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.kernel",
            "kernel-knowledge",
        ))
        .expect("knowledge base should be created");
    assert!(base.id > (1_u64 << 22));
    assert!(base.id <= i64::MAX as u64);
    assert_eq!(base.base_kind, AgentKnowledgeBaseKind::Hybrid);
    assert_eq!(base.status, AgentBusinessStatus::Draft);
    assert!(base
        .retrieval_modes
        .contains(&AgentKnowledgeIndexKind::Wiki));
    assert!(base
        .retrieval_modes
        .contains(&AgentKnowledgeIndexKind::Graph));

    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.kernel.wiki".to_string(),
            knowledge_base_id: "knowledge.base.kernel".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://sdkwork/kernel".to_string(),
            source_hash: "sha256:source123".to_string(),
            sync_policy_json: r#"{"mode":"incremental","schedule":"manual"}"#.to_string(),
            metadata_json: r#"{"owner":"architecture"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:01:00Z".to_string(),
        })
        .expect("knowledge source should be created");
    assert_eq!(source.source_kind, AgentKnowledgeSourceKind::Wiki);

    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.kernel.spi".to_string(),
            knowledge_base_id: "knowledge.base.kernel".to_string(),
            knowledge_source_id: Some("knowledge.source.kernel.wiki".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "Kernel SPI Standard".to_string(),
            content_ref: "knowledge-content://kernel/spi".to_string(),
            content_hash: "sha256:doc123".to_string(),
            summary: Some("Kernel SPI standard for agent capabilities".to_string()),
            metadata_json: r#"{"path":"/kernel/spi"}"#.to_string(),
            tags: vec!["kernel".to_string(), "spi".to_string()],
            categories: vec!["architecture".to_string()],
            trust_level: 4,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:02:00Z".to_string(),
        })
        .expect("knowledge document should be created");
    assert_eq!(document.document_kind, AgentKnowledgeDocumentKind::WikiPage);
    assert_eq!(document.chunk_count, 0);

    let chunk = service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.kernel.spi.intro".to_string(),
            knowledge_document_id: "knowledge.document.kernel.spi".to_string(),
            parent_chunk_id: None,
            chunk_ordinal: 1,
            heading: Some("SPI Boundary".to_string()),
            content_ref: "knowledge-content://kernel/spi#intro".to_string(),
            content_hash: "sha256:chunk123".to_string(),
            token_estimate: 256,
            summary: Some("KnowledgeProvider is the standard RAG boundary".to_string()),
            metadata_json: r#"{"section":"intro"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:03:00Z".to_string(),
        })
        .expect("knowledge chunk should be created");
    assert_eq!(chunk.chunk_ordinal, 1);
    assert_eq!(chunk.token_estimate, 256);

    let wiki_index = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.kernel.spi.wiki".to_string(),
            knowledge_base_id: "knowledge.base.kernel".to_string(),
            knowledge_document_id: Some("knowledge.document.kernel.spi".to_string()),
            knowledge_chunk_id: Some("knowledge.chunk.kernel.spi.intro".to_string()),
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://sdkwork/kernel/spi#intro".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:index123".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:04:00Z".to_string(),
        })
        .expect("wiki knowledge index should be upserted without vector metadata");
    assert_eq!(wiki_index.index_kind, AgentKnowledgeIndexKind::Wiki);
    assert_eq!(wiki_index.embedding_model_id, None);

    let indexes = service
        .list_knowledge_indexes(1, "knowledge.document.kernel.spi", subject())
        .expect("knowledge indexes should list");
    assert_eq!(indexes.len(), 1);
    assert_eq!(
        indexes[0].knowledge_index_id,
        "knowledge.index.kernel.spi.wiki"
    );

    let binding = service
        .create_knowledge_binding(AgentKnowledgeBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_binding_id: "knowledge.binding.agent.kernel.default".to_string(),
            knowledge_base_id: "knowledge.base.kernel".to_string(),
            agent_id: Some("agent.kernel.architect".to_string()),
            deployment_id: None,
            scope_kind: AgentKnowledgeBindingScopeKind::Agent,
            scope_ref: "agent.kernel.architect".to_string(),
            active: true,
            default_binding: true,
            requested_by: subject(),
            requested_at: "2026-06-05T01:05:00Z".to_string(),
        })
        .expect("knowledge binding should be created");
    assert!(binding.active);
    assert!(binding.default_binding);

    let sync_job = service
        .create_knowledge_sync_job(AgentKnowledgeSyncJobCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            sync_job_id: "knowledge.sync.kernel.manual.1".to_string(),
            knowledge_base_id: "knowledge.base.kernel".to_string(),
            knowledge_source_id: Some("knowledge.source.kernel.wiki".to_string()),
            job_kind: AgentKnowledgeSyncJobKind::Reindex,
            input_ref: "job-input://knowledge/kernel/manual/1".to_string(),
            input_json: r#"{"reason":"manual-reindex"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:06:00Z".to_string(),
        })
        .expect("knowledge sync job should be created");
    assert_eq!(sync_job.job_kind, AgentKnowledgeSyncJobKind::Reindex);
    assert_eq!(sync_job.status, AgentKnowledgeSyncJobStatus::Queued);
}

#[test]
fn knowledge_bindings_require_scope_identifier_consistency() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.binding.scope",
            "binding-scope-knowledge",
        ))
        .expect("knowledge base should be created");

    for (binding_id, agent_id, deployment_id, scope_kind, scope_ref, reason) in [
        (
            "knowledge.binding.scope.agent.missing_agent",
            None,
            None,
            AgentKnowledgeBindingScopeKind::Agent,
            "agent.binding.scope",
            "agent scope requires agentId",
        ),
        (
            "knowledge.binding.scope.agent.mismatch",
            Some("agent.binding.scope"),
            None,
            AgentKnowledgeBindingScopeKind::Agent,
            "agent.binding.other",
            "agent scopeRef must match agentId",
        ),
        (
            "knowledge.binding.scope.deployment.missing_deployment",
            Some("agent.binding.scope"),
            None,
            AgentKnowledgeBindingScopeKind::Deployment,
            "deployment.binding.scope",
            "deployment scope requires deploymentId",
        ),
        (
            "knowledge.binding.scope.deployment.mismatch",
            Some("agent.binding.scope"),
            Some("deployment.binding.scope"),
            AgentKnowledgeBindingScopeKind::Deployment,
            "deployment.binding.other",
            "deployment scopeRef must match deploymentId",
        ),
    ] {
        let error = service
            .create_knowledge_binding(AgentKnowledgeBindingCreateCommand {
                tenant_id: 1,
                organization_id: 10,
                knowledge_binding_id: binding_id.to_string(),
                knowledge_base_id: "knowledge.base.binding.scope".to_string(),
                agent_id: agent_id.map(str::to_string),
                deployment_id: deployment_id.map(str::to_string),
                scope_kind,
                scope_ref: scope_ref.to_string(),
                active: true,
                default_binding: false,
                requested_by: subject(),
                requested_at: "2026-06-05T02:06:00Z".to_string(),
            })
            .expect_err(reason);
        assert_eq!(error.kind(), KernelErrorKind::ValidationError, "{reason}");
    }

    let binding = service
        .create_knowledge_binding(AgentKnowledgeBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_binding_id: "knowledge.binding.scope.deployment.valid".to_string(),
            knowledge_base_id: "knowledge.base.binding.scope".to_string(),
            agent_id: Some("agent.binding.scope".to_string()),
            deployment_id: Some("deployment.binding.scope".to_string()),
            scope_kind: AgentKnowledgeBindingScopeKind::Deployment,
            scope_ref: "deployment.binding.scope".to_string(),
            active: true,
            default_binding: true,
            requested_by: subject(),
            requested_at: "2026-06-05T02:07:00Z".to_string(),
        })
        .expect("deployment scoped binding should accept matching identifiers");
    assert_eq!(binding.agent_id.as_deref(), Some("agent.binding.scope"));
    assert_eq!(
        binding.deployment_id.as_deref(),
        Some("deployment.binding.scope")
    );
    assert_eq!(binding.scope_ref, "deployment.binding.scope");
}

#[test]
fn knowledge_child_resources_support_provider_neutral_retrieve_contracts() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.retrieve",
            "retrieve-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.retrieve.wiki".to_string(),
            knowledge_base_id: "knowledge.base.retrieve".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://retrieve/kernel".to_string(),
            source_hash: "sha256:retrieve-source".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"owner":"retrieve"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:00:00Z".to_string(),
        })
        .expect("knowledge source should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.retrieve.spi".to_string(),
            knowledge_base_id: "knowledge.base.retrieve".to_string(),
            knowledge_source_id: Some("knowledge.source.retrieve.wiki".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "Retrieve SPI".to_string(),
            content_ref: "knowledge-content://retrieve/spi".to_string(),
            content_hash: "sha256:retrieve-doc".to_string(),
            summary: Some("Retrieve contracts expose managed RAG children".to_string()),
            metadata_json: r#"{"path":"/retrieve/spi"}"#.to_string(),
            tags: vec!["retrieve".to_string()],
            categories: vec!["contracts".to_string()],
            trust_level: 4,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:01:00Z".to_string(),
        })
        .expect("knowledge document should be created");
    service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.retrieve.spi.intro".to_string(),
            knowledge_document_id: "knowledge.document.retrieve.spi".to_string(),
            parent_chunk_id: None,
            chunk_ordinal: 1,
            heading: Some("Retrieve boundary".to_string()),
            content_ref: "knowledge-content://retrieve/spi#intro".to_string(),
            content_hash: "sha256:retrieve-chunk".to_string(),
            token_estimate: 128,
            summary: Some("Child resources must support direct retrieval".to_string()),
            metadata_json: r#"{"section":"intro"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:02:00Z".to_string(),
        })
        .expect("knowledge chunk should be created");
    service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.retrieve.spi.wiki".to_string(),
            knowledge_base_id: "knowledge.base.retrieve".to_string(),
            knowledge_document_id: Some("knowledge.document.retrieve.spi".to_string()),
            knowledge_chunk_id: Some("knowledge.chunk.retrieve.spi.intro".to_string()),
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://retrieve/kernel/spi#intro".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:retrieve-index".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:03:00Z".to_string(),
        })
        .expect("knowledge index should be upserted");
    service
        .create_knowledge_binding(AgentKnowledgeBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_binding_id: "knowledge.binding.retrieve.agent.default".to_string(),
            knowledge_base_id: "knowledge.base.retrieve".to_string(),
            agent_id: Some("agent.retrieve.kernel".to_string()),
            deployment_id: None,
            scope_kind: AgentKnowledgeBindingScopeKind::Agent,
            scope_ref: "agent.retrieve.kernel".to_string(),
            active: true,
            default_binding: true,
            requested_by: subject(),
            requested_at: "2026-06-05T02:04:00Z".to_string(),
        })
        .expect("knowledge binding should be created");
    service
        .create_knowledge_sync_job(AgentKnowledgeSyncJobCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            sync_job_id: "knowledge.sync.retrieve.reindex.1".to_string(),
            knowledge_base_id: "knowledge.base.retrieve".to_string(),
            knowledge_source_id: Some("knowledge.source.retrieve.wiki".to_string()),
            job_kind: AgentKnowledgeSyncJobKind::Reindex,
            input_ref: "job-input://knowledge/retrieve/reindex/1".to_string(),
            input_json: r#"{"reason":"contract"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:05:00Z".to_string(),
        })
        .expect("knowledge sync job should be created");

    let chunk = service
        .get_knowledge_chunk(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.chunk.retrieve.spi.intro".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge chunk should be retrieved");
    assert_eq!(
        chunk.knowledge_document_id,
        "knowledge.document.retrieve.spi"
    );
    assert_eq!(chunk.status, AgentBusinessStatus::Active);

    let index = service
        .get_knowledge_index(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.index.retrieve.spi.wiki".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge index should be retrieved");
    assert_eq!(index.knowledge_base_id, "knowledge.base.retrieve");
    assert_eq!(
        index.knowledge_chunk_id.as_deref(),
        Some("knowledge.chunk.retrieve.spi.intro")
    );

    let binding = service
        .get_knowledge_binding(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.binding.retrieve.agent.default".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge binding should be retrieved");
    assert_eq!(binding.scope_ref, "agent.retrieve.kernel");
    assert!(binding.default_binding);

    let sync_job = service
        .get_knowledge_sync_job(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.sync.retrieve.reindex.1".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge sync job should be retrieved");
    assert_eq!(sync_job.knowledge_base_id, "knowledge.base.retrieve");
    assert_eq!(sync_job.status, AgentKnowledgeSyncJobStatus::Queued);
}

#[test]
fn knowledge_sync_jobs_support_runtime_status_transitions() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.sync.runtime",
            "sync-runtime-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.sync.runtime.wiki".to_string(),
            knowledge_base_id: "knowledge.base.sync.runtime".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://sync/runtime".to_string(),
            source_hash: "sha256:sync-runtime-source".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"owner":"runtime"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:00:00Z".to_string(),
        })
        .expect("knowledge source should be created");

    let sync_job = service
        .create_knowledge_sync_job(AgentKnowledgeSyncJobCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            sync_job_id: "knowledge.sync.runtime.reindex.1".to_string(),
            knowledge_base_id: "knowledge.base.sync.runtime".to_string(),
            knowledge_source_id: Some("knowledge.source.sync.runtime.wiki".to_string()),
            job_kind: AgentKnowledgeSyncJobKind::Reindex,
            input_ref: "job-input://knowledge/sync/runtime/1".to_string(),
            input_json: r#"{"reason":"runtime"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:01:00Z".to_string(),
        })
        .expect("knowledge sync job should be created");
    assert_eq!(sync_job.status, AgentKnowledgeSyncJobStatus::Queued);
    assert_eq!(sync_job.started_at, None);
    assert_eq!(sync_job.completed_at, None);

    let running = service
        .start_knowledge_sync_job(AgentKnowledgeSyncJobStartCommand {
            tenant_id: 1,
            sync_job_id: "knowledge.sync.runtime.reindex.1".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:02:00Z".to_string(),
        })
        .expect("queued knowledge sync job should start");
    assert_eq!(running.status, AgentKnowledgeSyncJobStatus::Running);
    assert_eq!(running.started_at.as_deref(), Some("2026-06-05T03:02:00Z"));
    assert_eq!(running.completed_at, None);

    let completed = service
        .complete_knowledge_sync_job(AgentKnowledgeSyncJobCompleteCommand {
            tenant_id: 1,
            sync_job_id: "knowledge.sync.runtime.reindex.1".to_string(),
            output_json: r#"{"indexedDocuments":1,"indexedChunks":0}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:03:00Z".to_string(),
        })
        .expect("running knowledge sync job should complete");
    assert_eq!(completed.status, AgentKnowledgeSyncJobStatus::Succeeded);
    assert_eq!(
        completed.output_json.as_deref(),
        Some(r#"{"indexedDocuments":1,"indexedChunks":0}"#)
    );
    assert_eq!(
        completed.completed_at.as_deref(),
        Some("2026-06-05T03:03:00Z")
    );
    assert_eq!(completed.error_json, None);

    let complete_twice = service
        .complete_knowledge_sync_job(AgentKnowledgeSyncJobCompleteCommand {
            tenant_id: 1,
            sync_job_id: "knowledge.sync.runtime.reindex.1".to_string(),
            output_json: r#"{"ignored":true}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:04:00Z".to_string(),
        })
        .expect_err("terminal knowledge sync job must not complete twice");
    assert_eq!(complete_twice.kind(), KernelErrorKind::ValidationError);

    service
        .create_knowledge_sync_job(AgentKnowledgeSyncJobCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            sync_job_id: "knowledge.sync.runtime.fail.1".to_string(),
            knowledge_base_id: "knowledge.base.sync.runtime".to_string(),
            knowledge_source_id: Some("knowledge.source.sync.runtime.wiki".to_string()),
            job_kind: AgentKnowledgeSyncJobKind::Refresh,
            input_ref: "job-input://knowledge/sync/fail/1".to_string(),
            input_json: r#"{"reason":"failure-contract"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:05:00Z".to_string(),
        })
        .expect("failing sync job should be created");
    service
        .start_knowledge_sync_job(AgentKnowledgeSyncJobStartCommand {
            tenant_id: 1,
            sync_job_id: "knowledge.sync.runtime.fail.1".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:06:00Z".to_string(),
        })
        .expect("failing sync job should start");
    let failed = service
        .fail_knowledge_sync_job(AgentKnowledgeSyncJobFailCommand {
            tenant_id: 1,
            sync_job_id: "knowledge.sync.runtime.fail.1".to_string(),
            error_json: r#"{"code":"source_unavailable"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:07:00Z".to_string(),
        })
        .expect("running sync job should fail");
    assert_eq!(failed.status, AgentKnowledgeSyncJobStatus::Failed);
    assert_eq!(
        failed.error_json.as_deref(),
        Some(r#"{"code":"source_unavailable"}"#)
    );
    assert_eq!(failed.completed_at.as_deref(), Some("2026-06-05T03:07:00Z"));

    service
        .create_knowledge_sync_job(AgentKnowledgeSyncJobCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            sync_job_id: "knowledge.sync.runtime.cancel.1".to_string(),
            knowledge_base_id: "knowledge.base.sync.runtime".to_string(),
            knowledge_source_id: Some("knowledge.source.sync.runtime.wiki".to_string()),
            job_kind: AgentKnowledgeSyncJobKind::Import,
            input_ref: "job-input://knowledge/sync/cancel/1".to_string(),
            input_json: r#"{"reason":"cancel-contract"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:08:00Z".to_string(),
        })
        .expect("cancelled sync job should be created");
    let cancelled = service
        .cancel_knowledge_sync_job(AgentKnowledgeSyncJobCancelCommand {
            tenant_id: 1,
            sync_job_id: "knowledge.sync.runtime.cancel.1".to_string(),
            cancellation_json: r#"{"reason":"operator_cancelled"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:09:00Z".to_string(),
        })
        .expect("queued sync job should cancel");
    assert_eq!(cancelled.status, AgentKnowledgeSyncJobStatus::Cancelled);
    assert_eq!(cancelled.started_at, None);
    assert_eq!(
        cancelled.completed_at.as_deref(),
        Some("2026-06-05T03:09:00Z")
    );
}

#[test]
fn knowledge_bases_support_marketplace_crud_and_deleted_bases_are_inactive() {
    let mut service = service();
    let base = service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.crud",
            "crud-knowledge",
        ))
        .expect("knowledge base should be created");

    let retrieved = service
        .get_knowledge_base(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.crud".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge base should be retrieved");
    assert_eq!(retrieved.knowledge_base_id, "knowledge.base.crud");

    let updated = service
        .update_knowledge_base(AgentKnowledgeBaseUpdateCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.crud".to_string(),
            expected_version: Some(base.version),
            display_name: Some("Updated Kernel Knowledge".to_string()),
            description: Some("Updated provider-neutral RAG base".to_string()),
            provider_id: Some("provider.knowledge.hybrid".to_string()),
            base_kind: Some(AgentKnowledgeBaseKind::DocumentRepository),
            retrieval_modes: Some(vec![
                AgentKnowledgeIndexKind::Keyword,
                AgentKnowledgeIndexKind::FullText,
                AgentKnowledgeIndexKind::Hybrid,
            ]),
            capability_ids: Some(vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ]),
            configuration_profile_id: Some("profile.knowledge.crud.updated".to_string()),
            visibility: Some(AgentVisibility::Tenant),
            requested_by: subject(),
            requested_at: "2026-06-05T01:10:00Z".to_string(),
        })
        .expect("knowledge base should be updated");
    assert_eq!(updated.display_name, "Updated Kernel Knowledge");
    assert_eq!(
        updated.base_kind,
        AgentKnowledgeBaseKind::DocumentRepository
    );
    assert_eq!(updated.provider_id, "provider.knowledge.hybrid");
    assert_eq!(updated.visibility, AgentVisibility::Tenant);
    assert_eq!(updated.version, base.version + 1);

    let stale_update = service
        .update_knowledge_base(AgentKnowledgeBaseUpdateCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.crud".to_string(),
            expected_version: Some(base.version),
            display_name: Some("Stale update".to_string()),
            description: None,
            provider_id: None,
            base_kind: None,
            retrieval_modes: None,
            capability_ids: None,
            configuration_profile_id: None,
            visibility: None,
            requested_by: subject(),
            requested_at: "2026-06-05T01:11:00Z".to_string(),
        })
        .expect_err("stale expected version should fail");
    assert_eq!(stale_update.kind(), KernelErrorKind::Conflict);

    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.crud.before_delete".to_string(),
            knowledge_base_id: "knowledge.base.crud".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Runbook,
            title: "CRUD Parent Lifecycle".to_string(),
            content_ref: "knowledge-content://crud/before-delete".to_string(),
            content_hash: "sha256:crud-before-delete".to_string(),
            summary: Some("Deleted parent bases hide existing documents".to_string()),
            metadata_json: r#"{"lifecycle":"parent-delete"}"#.to_string(),
            tags: vec!["crud".to_string()],
            categories: vec!["lifecycle".to_string()],
            trust_level: 3,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:11:30Z".to_string(),
        })
        .expect("document should be created before parent base deletion");

    let deleted = service
        .delete_knowledge_base(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.crud".to_string(),
            expected_version: Some(updated.version),
            requested_by: subject(),
            requested_at: "2026-06-05T01:12:00Z".to_string(),
        })
        .expect("knowledge base should be deleted");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);
    assert!(deleted.deleted_at.is_some());

    let retrieve_deleted_base = service
        .get_knowledge_base(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.crud".to_string(),
            requested_by: subject(),
        })
        .expect_err("knowledge base retrieve should hide deleted bases");
    assert_eq!(
        retrieve_deleted_base.kind(),
        KernelErrorKind::ValidationError
    );

    let read_under_deleted_base = service
        .read_knowledge(AgentKnowledgeReadCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.crud.before_delete".to_string(),
            requested_by: subject(),
        })
        .expect_err("knowledge.read should hide documents under deleted bases");
    assert_eq!(
        read_under_deleted_base.kind(),
        KernelErrorKind::ValidationError
    );

    let source_under_deleted = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.crud.blocked".to_string(),
            knowledge_base_id: "knowledge.base.crud".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://crud/blocked".to_string(),
            source_hash: "sha256:crud-blocked".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"blocked":true}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:13:00Z".to_string(),
        })
        .expect_err("deleted bases should reject child resources");
    assert_eq!(
        source_under_deleted.kind(),
        KernelErrorKind::ValidationError
    );

    let restored = service
        .restore_knowledge_base(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.crud".to_string(),
            expected_version: Some(deleted.version),
            requested_by: subject(),
            requested_at: "2026-06-05T01:14:00Z".to_string(),
        })
        .expect("knowledge base should be restored");
    assert_eq!(restored.status, AgentBusinessStatus::Active);
    assert!(restored.deleted_at.is_none());

    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.crud.allowed".to_string(),
            knowledge_base_id: "knowledge.base.crud".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://crud/allowed".to_string(),
            source_hash: "sha256:crud-allowed".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"blocked":false}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T01:15:00Z".to_string(),
        })
        .expect("restored bases should accept child resources");
    assert_eq!(source.knowledge_base_id, "knowledge.base.crud");
}

#[test]
fn knowledge_sources_support_retrieve_update_delete_and_restore_lifecycle() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.sources.crud",
            "source-crud-knowledge",
        ))
        .expect("knowledge base should be created");
    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.sources.crud".to_string(),
            knowledge_base_id: "knowledge.base.sources.crud".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://sources/crud".to_string(),
            source_hash: "sha256:source-crud".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"version":1}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:00:00Z".to_string(),
        })
        .expect("knowledge source should be created");

    let retrieved = service
        .get_knowledge_source(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.sources.crud".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge source should be retrieved");
    assert_eq!(retrieved.knowledge_base_id, "knowledge.base.sources.crud");

    let updated = service
        .update_knowledge_source(AgentKnowledgeSourceUpdateCommand {
            tenant_id: 1,
            knowledge_source_id: "knowledge.source.sources.crud".to_string(),
            expected_version: Some(source.version),
            source_kind: Some(AgentKnowledgeSourceKind::Web),
            source_ref: Some("https://docs.example.test/sources/crud".to_string()),
            source_hash: Some("sha256:source-crud-v2".to_string()),
            sync_policy_json: Some(r#"{"mode":"incremental"}"#.to_string()),
            metadata_json: Some(r#"{"version":2}"#.to_string()),
            requested_by: subject(),
            requested_at: "2026-06-05T05:01:00Z".to_string(),
        })
        .expect("knowledge source should be updated");
    assert_eq!(updated.source_kind, AgentKnowledgeSourceKind::Web);
    assert_eq!(updated.source_hash, "sha256:source-crud-v2");
    assert_eq!(updated.version, source.version + 1);

    let stale_update = service
        .update_knowledge_source(AgentKnowledgeSourceUpdateCommand {
            tenant_id: 1,
            knowledge_source_id: "knowledge.source.sources.crud".to_string(),
            expected_version: Some(source.version),
            source_kind: None,
            source_ref: Some("https://docs.example.test/stale".to_string()),
            source_hash: None,
            sync_policy_json: None,
            metadata_json: None,
            requested_by: subject(),
            requested_at: "2026-06-05T05:02:00Z".to_string(),
        })
        .expect_err("stale source update should fail");
    assert_eq!(stale_update.kind(), KernelErrorKind::Conflict);

    let deleted = service
        .delete_knowledge_source(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.sources.crud".to_string(),
            expected_version: Some(updated.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:03:00Z".to_string(),
        })
        .expect("knowledge source should be deleted");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);
    assert!(deleted.deleted_at.is_some());
    assert!(service
        .list_knowledge_sources(1, "knowledge.base.sources.crud", subject())
        .expect("source listing should work")
        .is_empty());

    let retrieve_deleted_source = service
        .get_knowledge_source(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.sources.crud".to_string(),
            requested_by: subject(),
        })
        .expect_err("knowledge source retrieve should hide deleted sources");
    assert_eq!(
        retrieve_deleted_source.kind(),
        KernelErrorKind::ValidationError
    );

    let document_under_deleted_source = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.sources.blocked".to_string(),
            knowledge_base_id: "knowledge.base.sources.crud".to_string(),
            knowledge_source_id: Some("knowledge.source.sources.crud".to_string()),
            document_kind: AgentKnowledgeDocumentKind::Article,
            title: "Blocked Source Document".to_string(),
            content_ref: "knowledge-content://sources/blocked".to_string(),
            content_hash: "sha256:sources-blocked".to_string(),
            summary: Some("Deleted sources must not accept documents".to_string()),
            metadata_json: r#"{"blocked":true}"#.to_string(),
            tags: vec!["source".to_string()],
            categories: vec!["lifecycle".to_string()],
            trust_level: 3,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:04:00Z".to_string(),
        })
        .expect_err("deleted source should reject new documents");
    assert_eq!(
        document_under_deleted_source.kind(),
        KernelErrorKind::ValidationError
    );

    let restored = service
        .restore_knowledge_source(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.sources.crud".to_string(),
            expected_version: Some(deleted.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:05:00Z".to_string(),
        })
        .expect("knowledge source should be restored");
    assert_eq!(restored.status, AgentBusinessStatus::Active);
    assert!(restored.deleted_at.is_none());
    assert_eq!(
        service
            .list_knowledge_sources(1, "knowledge.base.sources.crud", subject())
            .expect("restored source should be listed")
            .len(),
        1
    );
}

#[test]
fn knowledge_restore_requires_active_parent_resources() {
    let mut service = service();
    let base = service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.restore.parent",
            "restore-parent-knowledge",
        ))
        .expect("knowledge base should be created");
    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.restore.parent".to_string(),
            knowledge_base_id: "knowledge.base.restore.parent".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://restore/parent".to_string(),
            source_hash: "sha256:restore-parent-source".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"restore":"parent"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:30:00Z".to_string(),
        })
        .expect("knowledge source should be created");
    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.restore.parent".to_string(),
            knowledge_base_id: "knowledge.base.restore.parent".to_string(),
            knowledge_source_id: Some("knowledge.source.restore.parent".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "Restore Parent Document".to_string(),
            content_ref: "knowledge-content://restore/parent".to_string(),
            content_hash: "sha256:restore-parent-document".to_string(),
            summary: Some("Restore requires active parents".to_string()),
            metadata_json: r#"{"restore":"parent"}"#.to_string(),
            tags: vec!["restore".to_string()],
            categories: vec!["lifecycle".to_string()],
            trust_level: 3,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:31:00Z".to_string(),
        })
        .expect("knowledge document should be created");
    let deleted_document = service
        .delete_knowledge_document(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.restore.parent".to_string(),
            expected_version: Some(document.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:32:00Z".to_string(),
        })
        .expect("knowledge document should be deleted");
    let deleted_source_before_base = service
        .delete_knowledge_source(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.restore.parent".to_string(),
            expected_version: Some(source.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:32:30Z".to_string(),
        })
        .expect("knowledge source should be deleted before parent base deletion");
    service
        .delete_knowledge_base(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.restore.parent".to_string(),
            expected_version: Some(base.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:33:00Z".to_string(),
        })
        .expect("knowledge base should be deleted");

    let restore_document_under_deleted_base = service
        .restore_knowledge_document(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.restore.parent".to_string(),
            expected_version: Some(deleted_document.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:34:00Z".to_string(),
        })
        .expect_err("knowledge document restore should reject deleted parent bases");
    assert_eq!(
        restore_document_under_deleted_base.kind(),
        KernelErrorKind::ValidationError
    );

    let restore_source_under_deleted_base = service
        .restore_knowledge_source(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.restore.parent".to_string(),
            expected_version: Some(deleted_source_before_base.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:35:00Z".to_string(),
        })
        .expect_err("knowledge source restore should reject deleted parent bases");
    assert_eq!(
        restore_source_under_deleted_base.kind(),
        KernelErrorKind::ValidationError
    );

    service
        .restore_knowledge_base(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.restore.parent".to_string(),
            expected_version: Some(base.version + 1),
            requested_by: subject(),
            requested_at: "2026-06-05T05:36:00Z".to_string(),
        })
        .expect("knowledge base should restore before child restore");
    let restore_document_under_deleted_source = service
        .restore_knowledge_document(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.restore.parent".to_string(),
            expected_version: Some(deleted_document.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:38:00Z".to_string(),
        })
        .expect_err("knowledge document restore should reject deleted parent sources");
    assert_eq!(
        restore_document_under_deleted_source.kind(),
        KernelErrorKind::ValidationError
    );

    let restored_source = service
        .restore_knowledge_source(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.restore.parent".to_string(),
            expected_version: Some(deleted_source_before_base.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:39:00Z".to_string(),
        })
        .expect("knowledge source should restore after parent base is active");
    assert_eq!(restored_source.status, AgentBusinessStatus::Active);
}

#[test]
fn knowledge_documents_support_update_with_source_scope_validation() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.documents.update",
            "document-update-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.documents.valid".to_string(),
            knowledge_base_id: "knowledge.base.documents.update".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://documents/valid".to_string(),
            source_hash: "sha256:documents-valid".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"scope":"valid"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:10:00Z".to_string(),
        })
        .expect("valid source should be created");
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.documents.other",
            "document-other-knowledge",
        ))
        .expect("other knowledge base should be created");
    service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.documents.other".to_string(),
            knowledge_base_id: "knowledge.base.documents.other".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://documents/other".to_string(),
            source_hash: "sha256:documents-other".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"scope":"other"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:11:00Z".to_string(),
        })
        .expect("other source should be created");
    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.documents.update".to_string(),
            knowledge_base_id: "knowledge.base.documents.update".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Article,
            title: "Original Document".to_string(),
            content_ref: "knowledge-content://documents/original".to_string(),
            content_hash: "sha256:documents-original".to_string(),
            summary: Some("Original summary".to_string()),
            metadata_json: r#"{"version":1}"#.to_string(),
            tags: vec!["original".to_string()],
            categories: vec!["docs".to_string()],
            trust_level: 2,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:12:00Z".to_string(),
        })
        .expect("document should be created");

    let updated = service
        .update_knowledge_document(AgentKnowledgeDocumentUpdateCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.documents.update".to_string(),
            expected_version: Some(document.version),
            knowledge_source_id: Some("knowledge.source.documents.valid".to_string()),
            document_kind: Some(AgentKnowledgeDocumentKind::Spec),
            title: Some("Updated Document".to_string()),
            content_ref: Some("knowledge-content://documents/updated".to_string()),
            content_hash: Some("sha256:documents-updated".to_string()),
            summary: Some("Updated summary".to_string()),
            metadata_json: Some(r#"{"version":2}"#.to_string()),
            tags: Some(vec!["updated".to_string(), "standard".to_string()]),
            categories: Some(vec!["spec".to_string()]),
            trust_level: Some(5),
            redaction_classification: Some("confidential".to_string()),
            requested_by: subject(),
            requested_at: "2026-06-05T05:13:00Z".to_string(),
        })
        .expect("document should be updated");
    assert_eq!(updated.document_kind, AgentKnowledgeDocumentKind::Spec);
    assert_eq!(
        updated.knowledge_source_id.as_deref(),
        Some("knowledge.source.documents.valid")
    );
    assert_eq!(updated.title, "Updated Document");
    assert_eq!(updated.version, document.version + 1);

    let cross_base_source = service
        .update_knowledge_document(AgentKnowledgeDocumentUpdateCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.documents.update".to_string(),
            expected_version: Some(updated.version),
            knowledge_source_id: Some("knowledge.source.documents.other".to_string()),
            document_kind: None,
            title: None,
            content_ref: None,
            content_hash: None,
            summary: None,
            metadata_json: None,
            tags: None,
            categories: None,
            trust_level: None,
            redaction_classification: None,
            requested_by: subject(),
            requested_at: "2026-06-05T05:14:00Z".to_string(),
        })
        .expect_err("document update should reject source from another base");
    assert_eq!(cross_base_source.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn knowledge_document_update_requires_existing_source_to_remain_active() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.documents.source_active",
            "document-source-active-knowledge",
        ))
        .expect("knowledge base should be created");
    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.documents.source_active".to_string(),
            knowledge_base_id: "knowledge.base.documents.source_active".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://documents/source-active".to_string(),
            source_hash: "sha256:documents-source-active".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"scope":"source-active"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:20:00Z".to_string(),
        })
        .expect("source should be created");
    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.documents.source_active".to_string(),
            knowledge_base_id: "knowledge.base.documents.source_active".to_string(),
            knowledge_source_id: Some("knowledge.source.documents.source_active".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "Source Active Document".to_string(),
            content_ref: "knowledge-content://documents/source-active".to_string(),
            content_hash: "sha256:documents-source-active-document".to_string(),
            summary: Some("Document updates require active source parents".to_string()),
            metadata_json: r#"{"version":1}"#.to_string(),
            tags: vec!["source-active".to_string()],
            categories: vec!["docs".to_string()],
            trust_level: 3,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:21:00Z".to_string(),
        })
        .expect("document should be created");
    service
        .delete_knowledge_source(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.documents.source_active".to_string(),
            expected_version: Some(source.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:22:00Z".to_string(),
        })
        .expect("source should be soft deleted");

    let update_under_deleted_source = service
        .update_knowledge_document(AgentKnowledgeDocumentUpdateCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.documents.source_active".to_string(),
            expected_version: Some(document.version),
            knowledge_source_id: None,
            document_kind: None,
            title: Some("Updated Under Deleted Source".to_string()),
            content_ref: None,
            content_hash: None,
            summary: None,
            metadata_json: None,
            tags: None,
            categories: None,
            trust_level: None,
            redaction_classification: None,
            requested_by: subject(),
            requested_at: "2026-06-05T05:23:00Z".to_string(),
        })
        .expect_err("document update should reject deleted existing source parents");
    assert_eq!(
        update_under_deleted_source.kind(),
        KernelErrorKind::ValidationError
    );
}

#[test]
fn knowledge_delete_requires_active_parent_base_resources() {
    let mut service = service();
    let base = service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.delete.parent",
            "delete-parent-knowledge",
        ))
        .expect("knowledge base should be created");
    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.delete.parent".to_string(),
            knowledge_base_id: "knowledge.base.delete.parent".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://delete/parent".to_string(),
            source_hash: "sha256:delete-parent-source".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"delete":"parent"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:24:00Z".to_string(),
        })
        .expect("source should be created");
    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.delete.parent".to_string(),
            knowledge_base_id: "knowledge.base.delete.parent".to_string(),
            knowledge_source_id: Some("knowledge.source.delete.parent".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "Delete Parent Document".to_string(),
            content_ref: "knowledge-content://delete/parent".to_string(),
            content_hash: "sha256:delete-parent-document".to_string(),
            summary: Some("Delete transitions require active parent resources".to_string()),
            metadata_json: r#"{"delete":"parent"}"#.to_string(),
            tags: vec!["delete".to_string()],
            categories: vec!["lifecycle".to_string()],
            trust_level: 3,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:25:00Z".to_string(),
        })
        .expect("document should be created");
    service
        .delete_knowledge_base(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.base.delete.parent".to_string(),
            expected_version: Some(base.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:26:00Z".to_string(),
        })
        .expect("knowledge base should be soft deleted");

    let delete_source_under_deleted_base = service
        .delete_knowledge_source(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.delete.parent".to_string(),
            expected_version: Some(source.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:27:00Z".to_string(),
        })
        .expect_err("source delete should reject deleted parent bases");
    assert_eq!(
        delete_source_under_deleted_base.kind(),
        KernelErrorKind::ValidationError
    );

    let delete_document_under_deleted_base = service
        .delete_knowledge_document(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.delete.parent".to_string(),
            expected_version: Some(document.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:28:00Z".to_string(),
        })
        .expect_err("document delete should reject deleted parent bases");
    assert_eq!(
        delete_document_under_deleted_base.kind(),
        KernelErrorKind::ValidationError
    );
}

#[test]
fn knowledge_document_delete_requires_existing_source_to_remain_active() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.delete.source_active",
            "delete-source-active-knowledge",
        ))
        .expect("knowledge base should be created");
    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.delete.source_active".to_string(),
            knowledge_base_id: "knowledge.base.delete.source_active".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://delete/source-active".to_string(),
            source_hash: "sha256:delete-source-active".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"scope":"delete-source-active"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:29:00Z".to_string(),
        })
        .expect("source should be created");
    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.delete.source_active".to_string(),
            knowledge_base_id: "knowledge.base.delete.source_active".to_string(),
            knowledge_source_id: Some("knowledge.source.delete.source_active".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "Delete Source Active Document".to_string(),
            content_ref: "knowledge-content://delete/source-active".to_string(),
            content_hash: "sha256:delete-source-active-document".to_string(),
            summary: Some("Document delete requires active source parents".to_string()),
            metadata_json: r#"{"version":1}"#.to_string(),
            tags: vec!["source-active".to_string()],
            categories: vec!["lifecycle".to_string()],
            trust_level: 3,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T05:30:00Z".to_string(),
        })
        .expect("document should be created");
    service
        .delete_knowledge_source(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.source.delete.source_active".to_string(),
            expected_version: Some(source.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:31:00Z".to_string(),
        })
        .expect("source should be soft deleted");

    let delete_under_deleted_source = service
        .delete_knowledge_document(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.delete.source_active".to_string(),
            expected_version: Some(document.version),
            requested_by: subject(),
            requested_at: "2026-06-05T05:32:00Z".to_string(),
        })
        .expect_err("document delete should reject deleted existing source parents");
    assert_eq!(
        delete_under_deleted_source.kind(),
        KernelErrorKind::ValidationError
    );
}

#[test]
fn knowledge_indexes_support_non_vector_rag_and_require_vector_metadata_only_for_vector_indexes() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.retrieval",
            "retrieval-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.retrieval.policy".to_string(),
            knowledge_base_id: "knowledge.base.retrieval".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Runbook,
            title: "Retrieval Policy".to_string(),
            content_ref: "knowledge-content://retrieval/policy".to_string(),
            content_hash: "sha256:policy".to_string(),
            summary: Some("RAG retrieval is not vector-only".to_string()),
            metadata_json: r#"{"kind":"policy"}"#.to_string(),
            tags: vec!["retrieval".to_string()],
            categories: vec!["policy".to_string()],
            trust_level: 5,
            redaction_classification: "public".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:00:00Z".to_string(),
        })
        .expect("knowledge document should be created");

    for (index_id, kind) in [
        (
            "knowledge.index.retrieval.keyword",
            AgentKnowledgeIndexKind::Keyword,
        ),
        (
            "knowledge.index.retrieval.full_text",
            AgentKnowledgeIndexKind::FullText,
        ),
        (
            "knowledge.index.retrieval.graph",
            AgentKnowledgeIndexKind::Graph,
        ),
        (
            "knowledge.index.retrieval.wiki",
            AgentKnowledgeIndexKind::Wiki,
        ),
        (
            "knowledge.index.retrieval.hybrid",
            AgentKnowledgeIndexKind::Hybrid,
        ),
        (
            "knowledge.index.retrieval.llm_rerank",
            AgentKnowledgeIndexKind::LlmRerank,
        ),
    ] {
        let index = service
            .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
                tenant_id: 1,
                knowledge_index_id: index_id.to_string(),
                knowledge_base_id: "knowledge.base.retrieval".to_string(),
                knowledge_document_id: Some("knowledge.document.retrieval.policy".to_string()),
                knowledge_chunk_id: None,
                index_kind: kind,
                index_provider_id: "provider.knowledge.hybrid".to_string(),
                external_ref: format!("retrieval://policy/{}", kind.as_str()),
                embedding_model_id: None,
                vector_dimension: None,
                content_hash: format!("sha256:{}", kind.as_str()),
                requested_by: subject(),
                requested_at: "2026-06-05T02:01:00Z".to_string(),
            })
            .expect("non-vector knowledge index should not require embedding metadata");
        assert_eq!(index.index_kind, kind);
    }

    let vector_error = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.retrieval.vector".to_string(),
            knowledge_base_id: "knowledge.base.retrieval".to_string(),
            knowledge_document_id: Some("knowledge.document.retrieval.policy".to_string()),
            knowledge_chunk_id: None,
            index_kind: AgentKnowledgeIndexKind::Vector,
            index_provider_id: "provider.knowledge.vector".to_string(),
            external_ref: "vector://policy".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:vector".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:02:00Z".to_string(),
        })
        .expect_err("vector knowledge index should require embedding metadata");
    assert_eq!(vector_error.kind(), KernelErrorKind::ValidationError);

    let vector_index = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.retrieval.vector".to_string(),
            knowledge_base_id: "knowledge.base.retrieval".to_string(),
            knowledge_document_id: Some("knowledge.document.retrieval.policy".to_string()),
            knowledge_chunk_id: None,
            index_kind: AgentKnowledgeIndexKind::Vector,
            index_provider_id: "provider.knowledge.vector".to_string(),
            external_ref: "vector://policy".to_string(),
            embedding_model_id: Some("model.embedding.text".to_string()),
            vector_dimension: Some(1536),
            content_hash: "sha256:vector".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:03:00Z".to_string(),
        })
        .expect("vector knowledge index should accept complete embedding metadata");
    assert_eq!(vector_index.vector_dimension, Some(1536));
}

#[test]
fn knowledge_search_returns_provider_neutral_rag_candidates_with_provenance() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.search",
            "search-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.search.wiki".to_string(),
            knowledge_base_id: "knowledge.base.search".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: "wiki://sdkwork/kernel".to_string(),
            source_hash: "sha256:search-source".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"space":"kernel"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:10:00Z".to_string(),
        })
        .expect("knowledge source should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.search.rag".to_string(),
            knowledge_base_id: "knowledge.base.search".to_string(),
            knowledge_source_id: Some("knowledge.source.search.wiki".to_string()),
            document_kind: AgentKnowledgeDocumentKind::WikiPage,
            title: "RAG Boundary".to_string(),
            content_ref: "knowledge-content://search/rag".to_string(),
            content_hash: "sha256:search-doc".to_string(),
            summary: Some("Provider-neutral RAG search candidates".to_string()),
            metadata_json: r#"{"topic":"rag"}"#.to_string(),
            tags: vec!["rag".to_string(), "kernel".to_string()],
            categories: vec!["architecture".to_string()],
            trust_level: 4,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:11:00Z".to_string(),
        })
        .expect("knowledge document should be created");
    service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.search.rag.boundary".to_string(),
            knowledge_document_id: "knowledge.document.search.rag".to_string(),
            parent_chunk_id: None,
            chunk_ordinal: 1,
            heading: Some("KnowledgeProvider boundary".to_string()),
            content_ref: "knowledge-content://search/rag#boundary".to_string(),
            content_hash: "sha256:search-chunk".to_string(),
            token_estimate: 96,
            summary: Some("Context assembly consumes retrieval candidates".to_string()),
            metadata_json: r#"{"section":"boundary"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:12:00Z".to_string(),
        })
        .expect("knowledge chunk should be created");
    service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.search.rag.wiki".to_string(),
            knowledge_base_id: "knowledge.base.search".to_string(),
            knowledge_document_id: Some("knowledge.document.search.rag".to_string()),
            knowledge_chunk_id: Some("knowledge.chunk.search.rag.boundary".to_string()),
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://sdkwork/kernel/rag#boundary".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:search-index-wiki".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:13:00Z".to_string(),
        })
        .expect("wiki knowledge index should be upserted");
    service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.search.rag.graph".to_string(),
            knowledge_base_id: "knowledge.base.search".to_string(),
            knowledge_document_id: None,
            knowledge_chunk_id: None,
            index_kind: AgentKnowledgeIndexKind::Graph,
            index_provider_id: "provider.knowledge.graph".to_string(),
            external_ref: "graph://sdkwork/kernel/rag-boundary".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:search-index-graph".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:14:00Z".to_string(),
        })
        .expect("base scoped graph index should be upserted");

    let results = service
        .search_knowledge(AgentKnowledgeSearchCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.search".to_string(),
            query: "rag".to_string(),
            top_k: 5,
            retrieval_modes: vec![
                AgentKnowledgeIndexKind::Wiki,
                AgentKnowledgeIndexKind::Graph,
            ],
            include_external: false,
            requested_by: subject(),
        })
        .expect("knowledge search should return provider-neutral candidates");

    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].knowledge_index_id,
        "knowledge.index.search.rag.wiki"
    );
    assert_eq!(
        results[0].knowledge_document_id.as_deref(),
        Some("knowledge.document.search.rag")
    );
    assert_eq!(
        results[0].knowledge_chunk_id.as_deref(),
        Some("knowledge.chunk.search.rag.boundary")
    );
    assert_eq!(results[0].retrieval_method, AgentKnowledgeIndexKind::Wiki);
    assert_eq!(results[0].title, "RAG Boundary");
    assert_eq!(
        results[0].source_ref.as_deref(),
        Some("wiki://sdkwork/kernel")
    );
    assert_eq!(
        results[0].content_ref.as_deref(),
        Some("knowledge-content://search/rag#boundary")
    );
    assert_eq!(
        results[0].external_ref.as_deref(),
        Some("wiki://sdkwork/kernel/rag#boundary")
    );
    assert_eq!(results[0].trust_level, 4);
    assert_eq!(results[0].redaction_classification, "internal");

    let unsupported = service
        .search_knowledge(AgentKnowledgeSearchCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.search".to_string(),
            query: "rag".to_string(),
            top_k: 5,
            retrieval_modes: vec![AgentKnowledgeIndexKind::Vector],
            include_external: false,
            requested_by: subject(),
        })
        .expect_err("search should reject retrieval modes not supported by the knowledge base");
    assert_eq!(unsupported.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn knowledge_search_enforces_openapi_request_limits() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.search.limits",
            "search-limits-knowledge",
        ))
        .expect("knowledge base should be created");

    let top_k_too_large = service
        .search_knowledge(AgentKnowledgeSearchCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.search.limits".to_string(),
            query: "kernel".to_string(),
            top_k: 101,
            retrieval_modes: vec![AgentKnowledgeIndexKind::Wiki],
            include_external: false,
            requested_by: subject(),
        })
        .expect_err("topK above the OpenAPI maximum must be rejected");
    assert_eq!(top_k_too_large.kind(), KernelErrorKind::ValidationError);

    let query_too_long = service
        .search_knowledge(AgentKnowledgeSearchCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.search.limits".to_string(),
            query: "k".repeat(4097),
            top_k: 10,
            retrieval_modes: vec![AgentKnowledgeIndexKind::Wiki],
            include_external: false,
            requested_by: subject(),
        })
        .expect_err("query above the OpenAPI maximum must be rejected");
    assert_eq!(query_too_long.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn knowledge_reference_fields_follow_openapi_and_database_length_bounds() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.reference.bounds",
            "reference-bounds-knowledge",
        ))
        .expect("knowledge base should be created");

    let long_ref = "r".repeat(1024);
    let source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.reference.bounds".to_string(),
            knowledge_base_id: "knowledge.base.reference.bounds".to_string(),
            source_kind: AgentKnowledgeSourceKind::Wiki,
            source_ref: long_ref.clone(),
            source_hash: "h".repeat(128),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"owner":"bounds"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:10:00Z".to_string(),
        })
        .expect("1024-character sourceRef should match OpenAPI and DB TEXT bounds");
    assert_eq!(source.source_ref, long_ref);

    let oversized_hash = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.reference.oversized_hash".to_string(),
            knowledge_base_id: "knowledge.base.reference.bounds".to_string(),
            knowledge_source_id: Some("knowledge.source.reference.bounds".to_string()),
            document_kind: AgentKnowledgeDocumentKind::Spec,
            title: "Reference Bounds".to_string(),
            content_ref: "knowledge-content://reference/bounds".to_string(),
            content_hash: "h".repeat(129),
            summary: Some("Hash values must honor database length bounds".to_string()),
            metadata_json: r#"{"owner":"bounds"}"#.to_string(),
            tags: vec!["bounds".to_string()],
            categories: vec!["standard".to_string()],
            trust_level: 4,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:11:00Z".to_string(),
        })
        .expect_err("129-character contentHash must be rejected before persistence");
    assert_eq!(oversized_hash.kind(), KernelErrorKind::ValidationError);

    let oversized_scope_ref = service
        .create_knowledge_binding(AgentKnowledgeBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_binding_id: "knowledge.binding.reference.oversized_scope".to_string(),
            knowledge_base_id: "knowledge.base.reference.bounds".to_string(),
            agent_id: Some("agent.reference.bounds".to_string()),
            deployment_id: None,
            scope_kind: AgentKnowledgeBindingScopeKind::Agent,
            scope_ref: "s".repeat(129),
            active: true,
            default_binding: false,
            requested_by: subject(),
            requested_at: "2026-06-05T03:12:00Z".to_string(),
        })
        .expect_err("129-character scopeRef must be rejected before persistence");
    assert_eq!(oversized_scope_ref.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn knowledge_list_and_read_are_provider_neutral_spi_entry_points() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.spi",
            "spi-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.spi.standard".to_string(),
            knowledge_base_id: "knowledge.base.spi".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Spec,
            title: "KnowledgeProvider SPI".to_string(),
            content_ref: "knowledge-content://spi/standard".to_string(),
            content_hash: "sha256:spi-standard".to_string(),
            summary: Some("KnowledgeProvider exposes search, read, and list".to_string()),
            metadata_json: r#"{"standard":"knowledge-provider"}"#.to_string(),
            tags: vec!["spi".to_string()],
            categories: vec!["standard".to_string()],
            trust_level: 5,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T02:20:00Z".to_string(),
        })
        .expect("knowledge document should be created");

    let listed = service
        .list_knowledge(AgentKnowledgeListCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.spi".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge.list should list provider-neutral documents");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].knowledge_document_id,
        "knowledge.document.spi.standard"
    );

    let read = service
        .read_knowledge(AgentKnowledgeReadCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.spi.standard".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge.read should read a provider-neutral document");
    assert_eq!(read.title, "KnowledgeProvider SPI");
    assert_eq!(
        read.summary.as_deref(),
        Some("KnowledgeProvider exposes search, read, and list")
    );
    assert_eq!(read.redaction_classification, "internal");
}

#[test]
fn knowledge_records_reject_invalid_ids_plaintext_secrets_and_restore_soft_deleted_documents() {
    let mut service = service();

    let invalid_base = service
        .create_knowledge_base(knowledge_base_command("knowledge.bad", "bad-knowledge"))
        .expect_err("knowledge base id must use knowledge.base prefix");
    assert_eq!(invalid_base.kind(), KernelErrorKind::ValidationError);

    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.secure",
            "secure-knowledge",
        ))
        .expect("knowledge base should be created");

    let secret_source = service
        .create_knowledge_source(AgentKnowledgeSourceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_source_id: "knowledge.source.secure.secret".to_string(),
            knowledge_base_id: "knowledge.base.secure".to_string(),
            source_kind: AgentKnowledgeSourceKind::Api,
            source_ref: "https://example.test/docs?api_key=secret".to_string(),
            source_hash: "sha256:secret".to_string(),
            sync_policy_json: r#"{"mode":"manual"}"#.to_string(),
            metadata_json: r#"{"kind":"api"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:00:00Z".to_string(),
        })
        .expect_err("source ref must not contain plaintext secrets");
    assert_eq!(secret_source.kind(), KernelErrorKind::ValidationError);

    let document = service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.secure.policy".to_string(),
            knowledge_base_id: "knowledge.base.secure".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Policy,
            title: "Secure Knowledge Policy".to_string(),
            content_ref: "knowledge-content://secure/policy".to_string(),
            content_hash: "sha256:secure-policy".to_string(),
            summary: Some("Knowledge content references must be safe".to_string()),
            metadata_json: r#"{"classification":"internal"}"#.to_string(),
            tags: vec!["secure".to_string()],
            categories: vec!["policy".to_string()],
            trust_level: 5,
            redaction_classification: "confidential".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T03:01:00Z".to_string(),
        })
        .expect("knowledge document should be created");

    let deleted = service
        .delete_knowledge_document(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.secure.policy".to_string(),
            expected_version: Some(document.version),
            requested_by: subject(),
            requested_at: "2026-06-05T03:02:00Z".to_string(),
        })
        .expect("knowledge document should be deleted");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);

    let restored = service
        .restore_knowledge_document(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.secure.policy".to_string(),
            expected_version: Some(deleted.version),
            requested_by: subject(),
            requested_at: "2026-06-05T03:03:00Z".to_string(),
        })
        .expect("knowledge document should be restored");
    assert_eq!(restored.status, AgentBusinessStatus::Active);
    assert!(restored.deleted_at.is_none());
}

#[test]
fn knowledge_child_operations_treat_soft_deleted_documents_as_not_found() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.lifecycle",
            "lifecycle-knowledge",
        ))
        .expect("knowledge base should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.lifecycle.deleted".to_string(),
            knowledge_base_id: "knowledge.base.lifecycle".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Runbook,
            title: "Deleted Lifecycle Document".to_string(),
            content_ref: "knowledge-content://lifecycle/deleted".to_string(),
            content_hash: "sha256:lifecycle-deleted".to_string(),
            summary: Some("Deleted documents must not accept child operations".to_string()),
            metadata_json: r#"{"lifecycle":"deleted"}"#.to_string(),
            tags: vec!["lifecycle".to_string()],
            categories: vec!["policy".to_string()],
            trust_level: 4,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:00:00Z".to_string(),
        })
        .expect("knowledge document should be created");
    service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.lifecycle.deleted.before".to_string(),
            knowledge_document_id: "knowledge.document.lifecycle.deleted".to_string(),
            parent_chunk_id: None,
            chunk_ordinal: 1,
            heading: Some("Active before delete".to_string()),
            content_ref: "knowledge-content://lifecycle/deleted#before".to_string(),
            content_hash: "sha256:lifecycle-before".to_string(),
            token_estimate: 64,
            summary: Some("Existing chunks are hidden once document is deleted".to_string()),
            metadata_json: r#"{"section":"before"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:01:00Z".to_string(),
        })
        .expect("knowledge chunk should be created");
    service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.lifecycle.deleted.before".to_string(),
            knowledge_base_id: "knowledge.base.lifecycle".to_string(),
            knowledge_document_id: Some("knowledge.document.lifecycle.deleted".to_string()),
            knowledge_chunk_id: Some("knowledge.chunk.lifecycle.deleted.before".to_string()),
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://lifecycle/deleted#before".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:lifecycle-index-before".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:02:00Z".to_string(),
        })
        .expect("knowledge index should be upserted while document is active");

    let current = service
        .read_knowledge(AgentKnowledgeReadCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.lifecycle.deleted".to_string(),
            requested_by: subject(),
        })
        .expect("active document should be readable before deletion");
    service
        .delete_knowledge_document(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "knowledge.document.lifecycle.deleted".to_string(),
            expected_version: Some(current.version),
            requested_by: subject(),
            requested_at: "2026-06-05T04:03:00Z".to_string(),
        })
        .expect("knowledge document should be deleted");

    let read_deleted = service
        .read_knowledge(AgentKnowledgeReadCommand {
            tenant_id: 1,
            knowledge_document_id: "knowledge.document.lifecycle.deleted".to_string(),
            requested_by: subject(),
        })
        .expect_err("knowledge.read should hide deleted documents");
    assert_eq!(read_deleted.kind(), KernelErrorKind::ValidationError);
    let listed = service
        .list_knowledge(AgentKnowledgeListCommand {
            tenant_id: 1,
            knowledge_base_id: "knowledge.base.lifecycle".to_string(),
            requested_by: subject(),
        })
        .expect("knowledge.list should remain available");
    assert!(listed.is_empty());

    let chunk_after_delete = service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.lifecycle.deleted.after".to_string(),
            knowledge_document_id: "knowledge.document.lifecycle.deleted".to_string(),
            parent_chunk_id: None,
            chunk_ordinal: 2,
            heading: Some("After delete".to_string()),
            content_ref: "knowledge-content://lifecycle/deleted#after".to_string(),
            content_hash: "sha256:lifecycle-after".to_string(),
            token_estimate: 64,
            summary: Some("Deleted documents reject new chunks".to_string()),
            metadata_json: r#"{"section":"after"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:04:00Z".to_string(),
        })
        .expect_err("deleted documents should reject new chunks");
    assert_eq!(chunk_after_delete.kind(), KernelErrorKind::ValidationError);

    let chunks_after_delete = service
        .list_knowledge_chunks(1, "knowledge.document.lifecycle.deleted", subject())
        .expect_err("deleted documents should reject chunk listing");
    assert_eq!(chunks_after_delete.kind(), KernelErrorKind::ValidationError);

    let index_after_delete = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.lifecycle.deleted.after".to_string(),
            knowledge_base_id: "knowledge.base.lifecycle".to_string(),
            knowledge_document_id: Some("knowledge.document.lifecycle.deleted".to_string()),
            knowledge_chunk_id: None,
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://lifecycle/deleted#after".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:lifecycle-index-after".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:05:00Z".to_string(),
        })
        .expect_err("deleted documents should reject new indexes");
    assert_eq!(index_after_delete.kind(), KernelErrorKind::ValidationError);

    let indexes_after_delete = service
        .list_knowledge_indexes(1, "knowledge.document.lifecycle.deleted", subject())
        .expect_err("deleted documents should reject index listing");
    assert_eq!(
        indexes_after_delete.kind(),
        KernelErrorKind::ValidationError
    );
}

#[test]
fn knowledge_indexes_require_base_document_and_chunk_scope_consistency() {
    let mut service = service();
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.scope.a",
            "scope-knowledge-a",
        ))
        .expect("first knowledge base should be created");
    service
        .create_knowledge_base(knowledge_base_command(
            "knowledge.base.scope.b",
            "scope-knowledge-b",
        ))
        .expect("second knowledge base should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.scope.a".to_string(),
            knowledge_base_id: "knowledge.base.scope.a".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Spec,
            title: "Scope A Document".to_string(),
            content_ref: "knowledge-content://scope/a".to_string(),
            content_hash: "sha256:scope-a".to_string(),
            summary: Some("Document belongs to scope A".to_string()),
            metadata_json: r#"{"scope":"a"}"#.to_string(),
            tags: vec!["scope".to_string()],
            categories: vec!["standard".to_string()],
            trust_level: 5,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:10:00Z".to_string(),
        })
        .expect("scope A document should be created");
    service
        .create_knowledge_document(AgentKnowledgeDocumentCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_document_id: "knowledge.document.scope.b".to_string(),
            knowledge_base_id: "knowledge.base.scope.b".to_string(),
            knowledge_source_id: None,
            document_kind: AgentKnowledgeDocumentKind::Spec,
            title: "Scope B Document".to_string(),
            content_ref: "knowledge-content://scope/b".to_string(),
            content_hash: "sha256:scope-b".to_string(),
            summary: Some("Document belongs to scope B".to_string()),
            metadata_json: r#"{"scope":"b"}"#.to_string(),
            tags: vec!["scope".to_string()],
            categories: vec!["standard".to_string()],
            trust_level: 5,
            redaction_classification: "internal".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:11:00Z".to_string(),
        })
        .expect("scope B document should be created");
    service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.scope.a".to_string(),
            knowledge_document_id: "knowledge.document.scope.a".to_string(),
            parent_chunk_id: None,
            chunk_ordinal: 1,
            heading: Some("Scope A".to_string()),
            content_ref: "knowledge-content://scope/a#1".to_string(),
            content_hash: "sha256:scope-a-chunk".to_string(),
            token_estimate: 128,
            summary: Some("Chunk belongs to scope A document".to_string()),
            metadata_json: r#"{"scope":"a"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:12:00Z".to_string(),
        })
        .expect("scope A chunk should be created");

    let parent_chunk_cross_document = service
        .create_knowledge_chunk(AgentKnowledgeChunkCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            knowledge_chunk_id: "knowledge.chunk.scope.b.child".to_string(),
            knowledge_document_id: "knowledge.document.scope.b".to_string(),
            parent_chunk_id: Some("knowledge.chunk.scope.a".to_string()),
            chunk_ordinal: 1,
            heading: Some("Invalid parent".to_string()),
            content_ref: "knowledge-content://scope/b#child".to_string(),
            content_hash: "sha256:scope-b-child".to_string(),
            token_estimate: 128,
            summary: Some("Parent chunk must belong to the same document".to_string()),
            metadata_json: r#"{"scope":"b"}"#.to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:12:30Z".to_string(),
        })
        .expect_err("child chunks must use a parent from the same document");
    assert_eq!(
        parent_chunk_cross_document.kind(),
        KernelErrorKind::ValidationError
    );

    let cross_base = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.scope.cross_base".to_string(),
            knowledge_base_id: "knowledge.base.scope.b".to_string(),
            knowledge_document_id: Some("knowledge.document.scope.a".to_string()),
            knowledge_chunk_id: None,
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://scope/cross-base".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:cross-base".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:13:00Z".to_string(),
        })
        .expect_err("index document must belong to the target knowledge base");
    assert_eq!(cross_base.kind(), KernelErrorKind::ValidationError);

    let cross_document_chunk = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.scope.cross_document_chunk".to_string(),
            knowledge_base_id: "knowledge.base.scope.b".to_string(),
            knowledge_document_id: Some("knowledge.document.scope.b".to_string()),
            knowledge_chunk_id: Some("knowledge.chunk.scope.a".to_string()),
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://scope/cross-document-chunk".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:cross-document-chunk".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:14:00Z".to_string(),
        })
        .expect_err("index chunk must belong to the referenced document");
    assert_eq!(
        cross_document_chunk.kind(),
        KernelErrorKind::ValidationError
    );

    let chunk_without_document = service
        .upsert_knowledge_index(AgentKnowledgeIndexUpsertCommand {
            tenant_id: 1,
            knowledge_index_id: "knowledge.index.scope.chunk_without_document".to_string(),
            knowledge_base_id: "knowledge.base.scope.a".to_string(),
            knowledge_document_id: None,
            knowledge_chunk_id: Some("knowledge.chunk.scope.a".to_string()),
            index_kind: AgentKnowledgeIndexKind::Wiki,
            index_provider_id: "provider.knowledge.llm-wiki".to_string(),
            external_ref: "wiki://scope/chunk-without-document".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:chunk-without-document".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-05T04:15:00Z".to_string(),
        })
        .expect_err("chunk-scoped indexes must also reference the owning document");
    assert_eq!(
        chunk_without_document.kind(),
        KernelErrorKind::ValidationError
    );
}
