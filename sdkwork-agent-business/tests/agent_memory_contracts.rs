use sdkwork_agent_business::{
    AgentBusinessService, AgentBusinessStatus, AgentMemoryBindingCreateCommand,
    AgentMemoryBindingCreateRequestDto, AgentMemoryBindingRecordDto, AgentMemoryBindingScopeKind,
    AgentMemoryIndexKind, AgentMemoryNamespaceCreateCommand, AgentMemoryNamespaceCreateRequestDto,
    AgentMemoryNamespaceKind, AgentMemoryNamespaceRecordDto, AgentMemoryProfileCreateCommand,
    AgentMemoryProfileCreateRequestDto, AgentMemoryProfileRecordDto,
    AgentMemoryRecordCreateCommand, AgentMemoryRecordCreateRequestDto, AgentMemoryRecordDto,
    AgentMemoryRecordKind, AgentMemoryRelationCreateCommand, AgentMemoryRelationCreateRequestDto,
    AgentMemoryRelationKind, AgentMemoryRelationRecordDto, AgentMemoryRetrievalIndexRecordDto,
    AgentMemoryRetrievalIndexUpsertCommand, AgentMemoryRetrievalIndexUpsertRequestDto,
    AgentMemorySourceCreateCommand, AgentMemorySourceCreateRequestDto, AgentMemorySourceKind,
    AgentMemorySourceRecordDto, AgentMemoryStoreCreateCommand, AgentMemoryStoreCreateRequestDto,
    AgentMemoryStoreKind, AgentMemoryStoreRecordDto, AgentMemoryStoreUpdateCommand,
    AgentMemoryStoreUpdateRequestDto, AgentVisibility, AllowAllPolicyProvider,
    DeleteAgentMarketplaceItemCommand, GetAgentMarketplaceItemCommand, InMemoryAgentAuditSink,
    InMemoryAgentRepository, RestoreAgentMarketplaceItemCommand,
};
use sdkwork_agent_kernel::{KernelErrorKind, PolicySubject};

fn subject() -> PolicySubject {
    PolicySubject::new("user.memory.admin", "tenant.1").with_role("agent.memory.admin")
}

fn service(
) -> AgentBusinessService<InMemoryAgentRepository, InMemoryAgentAuditSink, AllowAllPolicyProvider> {
    AgentBusinessService::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("provider.policy.memory"),
    )
}

fn memory_store_command(store_id: &str, code: &str) -> AgentMemoryStoreCreateCommand {
    AgentMemoryStoreCreateCommand {
        tenant_id: 1,
        organization_id: 10,
        owner_user_id: 100,
        memory_store_id: store_id.to_string(),
        code: code.to_string(),
        display_name: "Primary Memory Store".to_string(),
        description: Some("Hybrid memory store".to_string()),
        provider_id: "provider.memory.local-postgres".to_string(),
        store_kind: AgentMemoryStoreKind::HybridStore,
        retrieval_modes: vec![
            AgentMemoryIndexKind::Keyword,
            AgentMemoryIndexKind::Graph,
            AgentMemoryIndexKind::Wiki,
        ],
        capability_ids: vec![
            "memory.write".to_string(),
            "memory.retrieve".to_string(),
            "memory.redact".to_string(),
        ],
        configuration_profile_id: "profile.memory.local".to_string(),
        visibility: AgentVisibility::Tenant,
        requested_by: subject(),
        requested_at: "2026-06-04T01:00:00Z".to_string(),
    }
}

#[test]
fn memory_dtos_are_public_agent_business_api_contracts() {
    fn assert_public_contract<T>() {}

    assert_public_contract::<AgentMemoryStoreCreateRequestDto>();
    assert_public_contract::<AgentMemoryStoreUpdateRequestDto>();
    assert_public_contract::<AgentMemoryProfileCreateRequestDto>();
    assert_public_contract::<AgentMemoryBindingCreateRequestDto>();
    assert_public_contract::<AgentMemoryNamespaceCreateRequestDto>();
    assert_public_contract::<AgentMemoryRecordCreateRequestDto>();
    assert_public_contract::<AgentMemorySourceCreateRequestDto>();
    assert_public_contract::<AgentMemoryRelationCreateRequestDto>();
    assert_public_contract::<AgentMemoryRetrievalIndexUpsertRequestDto>();
    assert_public_contract::<AgentMemoryStoreRecordDto>();
    assert_public_contract::<AgentMemoryProfileRecordDto>();
    assert_public_contract::<AgentMemoryBindingRecordDto>();
    assert_public_contract::<AgentMemoryNamespaceRecordDto>();
    assert_public_contract::<AgentMemoryRecordDto>();
    assert_public_contract::<AgentMemorySourceRecordDto>();
    assert_public_contract::<AgentMemoryRelationRecordDto>();
    assert_public_contract::<AgentMemoryRetrievalIndexRecordDto>();
}

#[test]
fn memory_business_crud_surface_is_documented_as_current_contract() {
    let database_spec = include_str!("../specs/AGENT_BUSINESS_DATABASE_SPEC.md");
    let normalized_database_spec = database_spec
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for required in [
        "- Memory stores: create, update, get.",
        "- Memory profiles: create, get.",
        "- Memory bindings: create, get.",
        "- Memory namespaces: create, get.",
        "- Memory records: create, get, list by namespace, soft-delete, restore.",
        "- Memory sources: create, list by memory id.",
        "- Memory relations: create, list by memory id.",
        "- Memory retrieval indexes: upsert, list by memory id.",
        "Memory binding scope consistency is enforced: agent scopes require matching agentId/scopeRef, and deployment scopes require agentId plus matching deploymentId/scopeRef.",
        "Memory profile, binding, record, source, relation, and retrieval-index create/list/restore paths use active-parent validation and hide soft-deleted parent resources.",
        "Memory retrieval index list queries are memory-record scoped and support non-vector RAG patterns such as wiki, rule, graph, keyword, and hybrid retrieval.",
    ] {
        let normalized_required = required.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized_database_spec.contains(normalized_required.as_str()),
            "database spec must document current memory CRUD/RAG surface: {required}"
        );
    }
}

#[test]
fn memory_store_profile_binding_namespace_and_record_crud_form_standard_memory_stack() {
    let mut service = service();

    let store = service
        .create_memory_store(memory_store_command(
            "memory.store.primary",
            "primary-memory",
        ))
        .expect("memory store should be created");
    assert!(store.id > (1_u64 << 22));
    assert!(store.id <= i64::MAX as u64);
    assert_eq!(store.store_kind, AgentMemoryStoreKind::HybridStore);
    assert_eq!(store.status, AgentBusinessStatus::Draft);
    assert_eq!(store.version, 1);

    let updated_store = service
        .update_memory_store(AgentMemoryStoreUpdateCommand {
            tenant_id: 1,
            memory_store_id: "memory.store.primary".to_string(),
            expected_version: Some(store.version),
            display_name: Some("Primary Hybrid Memory".to_string()),
            description: Some("Updated hybrid memory store".to_string()),
            provider_id: Some("provider.memory.local-postgres".to_string()),
            store_kind: Some(AgentMemoryStoreKind::GraphStore),
            retrieval_modes: Some(vec![
                AgentMemoryIndexKind::Keyword,
                AgentMemoryIndexKind::Graph,
            ]),
            capability_ids: Some(vec![
                "memory.retrieve".to_string(),
                "memory.graph".to_string(),
            ]),
            configuration_profile_id: Some("profile.memory.graph".to_string()),
            visibility: Some(AgentVisibility::Organization),
            requested_by: subject(),
            requested_at: "2026-06-04T01:01:00Z".to_string(),
        })
        .expect("memory store should update");
    assert_eq!(updated_store.store_kind, AgentMemoryStoreKind::GraphStore);
    assert_eq!(updated_store.version, 2);

    let fetched_store = service
        .get_memory_store(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.store.primary".to_string(),
            requested_by: subject(),
        })
        .expect("memory store should be retrievable");
    assert_eq!(fetched_store.memory_store_id, "memory.store.primary");
    assert_eq!(fetched_store.version, 2);

    let profile = service
        .create_memory_profile(AgentMemoryProfileCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            memory_profile_id: "memory.profile.default".to_string(),
            memory_store_id: "memory.store.primary".to_string(),
            code: "default-memory-profile".to_string(),
            display_name: "Default Memory Profile".to_string(),
            description: Some("Standard memory policy".to_string()),
            write_policy_json: r#"{"mode":"curated","promotion":"confidence"}"#.to_string(),
            retrieval_policy_json: r#"{"topK":8,"modes":["keyword","graph"]}"#.to_string(),
            compaction_policy_json: r#"{"summaryAfterTurns":20}"#.to_string(),
            retention_policy_json: r#"{"defaultTtlDays":365}"#.to_string(),
            privacy_policy_json: r#"{"pii":"redact","subjectRequest":"supported"}"#.to_string(),
            visibility: AgentVisibility::Tenant,
            requested_by: subject(),
            requested_at: "2026-06-04T01:02:00Z".to_string(),
        })
        .expect("memory profile should be created");
    assert_eq!(profile.memory_store_id, "memory.store.primary");

    let fetched_profile = service
        .get_memory_profile(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.profile.default".to_string(),
            requested_by: subject(),
        })
        .expect("memory profile should be retrievable");
    assert_eq!(fetched_profile.memory_profile_id, "memory.profile.default");
    assert_eq!(fetched_profile.memory_store_id, "memory.store.primary");

    let binding = service
        .create_memory_binding(AgentMemoryBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_binding_id: "memory.binding.agent.default".to_string(),
            memory_profile_id: "memory.profile.default".to_string(),
            agent_id: Some("agent.research.alpha".to_string()),
            deployment_id: None,
            scope_kind: AgentMemoryBindingScopeKind::Agent,
            scope_ref: "agent.research.alpha".to_string(),
            active: true,
            default_binding: true,
            requested_by: subject(),
            requested_at: "2026-06-04T01:03:00Z".to_string(),
        })
        .expect("memory binding should be created");
    assert!(binding.active);
    assert!(binding.default_binding);

    let fetched_binding = service
        .get_memory_binding(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.binding.agent.default".to_string(),
            requested_by: subject(),
        })
        .expect("memory binding should be retrievable");
    assert_eq!(fetched_binding.memory_profile_id, "memory.profile.default");
    assert_eq!(
        fetched_binding.scope_ref,
        fetched_binding.agent_id.as_deref().unwrap()
    );

    let namespace = service
        .create_memory_namespace(AgentMemoryNamespaceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_namespace_id: "memory.namespace.agent.alpha.user.1".to_string(),
            agent_id: Some("agent.research.alpha".to_string()),
            user_ref: Some("user.1".to_string()),
            session_ref: Some("session.1".to_string()),
            thread_ref: Some("thread.1".to_string()),
            namespace_kind: AgentMemoryNamespaceKind::User,
            visibility: AgentVisibility::Private,
            requested_by: subject(),
            requested_at: "2026-06-04T01:04:00Z".to_string(),
        })
        .expect("memory namespace should be created");
    assert_eq!(namespace.namespace_kind, AgentMemoryNamespaceKind::User);

    let fetched_namespace = service
        .get_memory_namespace(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.namespace.agent.alpha.user.1".to_string(),
            requested_by: subject(),
        })
        .expect("memory namespace should be retrievable");
    assert_eq!(
        fetched_namespace.memory_namespace_id,
        "memory.namespace.agent.alpha.user.1"
    );

    let record = service
        .create_memory_record(AgentMemoryRecordCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_id: "memory.record.preference.locale".to_string(),
            memory_namespace_id: "memory.namespace.agent.alpha.user.1".to_string(),
            agent_id: Some("agent.research.alpha".to_string()),
            memory_kind: AgentMemoryRecordKind::Preference,
            content_format: "application/json".to_string(),
            content_json: r#"{"preference":"answer-language","value":"zh-CN"}"#.to_string(),
            summary: Some("User prefers Chinese answers".to_string()),
            salience_score: 0.90,
            confidence_score: 0.95,
            freshness_score: 1.0,
            sensitivity_level: 1,
            effective_at: Some("2026-06-04T01:04:00Z".to_string()),
            expires_at: None,
            requested_by: subject(),
            requested_at: "2026-06-04T01:05:00Z".to_string(),
        })
        .expect("memory record should be created");
    assert_eq!(record.memory_kind, AgentMemoryRecordKind::Preference);
    assert_eq!(record.source_count, 0);

    let fetched_record = service
        .get_memory_record(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.record.preference.locale".to_string(),
            requested_by: subject(),
        })
        .expect("memory record should be retrievable");
    assert_eq!(
        fetched_record.memory_namespace_id,
        "memory.namespace.agent.alpha.user.1"
    );

    let listed = service
        .list_memory_records(1, "memory.namespace.agent.alpha.user.1", subject())
        .expect("memory records should list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].memory_id, "memory.record.preference.locale");

    let deleted = service
        .delete_memory_record(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.record.preference.locale".to_string(),
            expected_version: Some(record.version),
            requested_by: subject(),
            requested_at: "2026-06-04T01:06:00Z".to_string(),
        })
        .expect("memory record should be deleted");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);

    let hidden_deleted_record = service.get_memory_record(GetAgentMarketplaceItemCommand {
        tenant_id: 1,
        item_id: "memory.record.preference.locale".to_string(),
        requested_by: subject(),
    });
    assert!(hidden_deleted_record.is_err());

    let restored = service
        .restore_memory_record(RestoreAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.record.preference.locale".to_string(),
            expected_version: Some(deleted.version),
            requested_by: subject(),
            requested_at: "2026-06-04T01:07:00Z".to_string(),
        })
        .expect("memory record should be restored");
    assert_eq!(restored.status, AgentBusinessStatus::Active);

    let fetched_restored_record = service
        .get_memory_record(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.record.preference.locale".to_string(),
            requested_by: subject(),
        })
        .expect("restored memory record should be retrievable");
    assert_eq!(fetched_restored_record.status, AgentBusinessStatus::Active);
}

#[test]
fn memory_bindings_require_scope_identifier_consistency() {
    let mut service = service();
    service
        .create_memory_store(memory_store_command(
            "memory.store.binding.scope",
            "binding-scope-memory",
        ))
        .expect("memory store should be created");
    service
        .create_memory_profile(AgentMemoryProfileCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            memory_profile_id: "memory.profile.binding.scope".to_string(),
            memory_store_id: "memory.store.binding.scope".to_string(),
            code: "binding-scope-profile".to_string(),
            display_name: "Binding Scope Profile".to_string(),
            description: Some("Memory profile used to verify scope consistency".to_string()),
            write_policy_json: r#"{"mode":"curated"}"#.to_string(),
            retrieval_policy_json: r#"{"topK":8,"modes":["keyword"]}"#.to_string(),
            compaction_policy_json: r#"{"summaryAfterTurns":20}"#.to_string(),
            retention_policy_json: r#"{"defaultTtlDays":365}"#.to_string(),
            privacy_policy_json: r#"{"pii":"redact"}"#.to_string(),
            visibility: AgentVisibility::Tenant,
            requested_by: subject(),
            requested_at: "2026-06-04T04:00:00Z".to_string(),
        })
        .expect("memory profile should be created");

    for (binding_id, agent_id, deployment_id, scope_kind, scope_ref, reason) in [
        (
            "memory.binding.scope.agent.missing_agent",
            None,
            None,
            AgentMemoryBindingScopeKind::Agent,
            "agent.memory.scope",
            "agent memory scope requires agentId",
        ),
        (
            "memory.binding.scope.agent.mismatch",
            Some("agent.memory.scope"),
            None,
            AgentMemoryBindingScopeKind::Agent,
            "agent.memory.other",
            "agent memory scopeRef must match agentId",
        ),
        (
            "memory.binding.scope.deployment.missing_deployment",
            Some("agent.memory.scope"),
            None,
            AgentMemoryBindingScopeKind::Deployment,
            "deployment.memory.scope",
            "deployment memory scope requires deploymentId",
        ),
        (
            "memory.binding.scope.deployment.mismatch",
            Some("agent.memory.scope"),
            Some("deployment.memory.scope"),
            AgentMemoryBindingScopeKind::Deployment,
            "deployment.memory.other",
            "deployment memory scopeRef must match deploymentId",
        ),
    ] {
        let error = service
            .create_memory_binding(AgentMemoryBindingCreateCommand {
                tenant_id: 1,
                organization_id: 10,
                memory_binding_id: binding_id.to_string(),
                memory_profile_id: "memory.profile.binding.scope".to_string(),
                agent_id: agent_id.map(str::to_string),
                deployment_id: deployment_id.map(str::to_string),
                scope_kind,
                scope_ref: scope_ref.to_string(),
                active: true,
                default_binding: false,
                requested_by: subject(),
                requested_at: "2026-06-04T04:01:00Z".to_string(),
            })
            .expect_err(reason);
        assert_eq!(error.kind(), KernelErrorKind::ValidationError, "{reason}");
    }

    let binding = service
        .create_memory_binding(AgentMemoryBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_binding_id: "memory.binding.scope.deployment.valid".to_string(),
            memory_profile_id: "memory.profile.binding.scope".to_string(),
            agent_id: Some("agent.memory.scope".to_string()),
            deployment_id: Some("deployment.memory.scope".to_string()),
            scope_kind: AgentMemoryBindingScopeKind::Deployment,
            scope_ref: "deployment.memory.scope".to_string(),
            active: true,
            default_binding: true,
            requested_by: subject(),
            requested_at: "2026-06-04T04:02:00Z".to_string(),
        })
        .expect("deployment scoped memory binding should accept matching identifiers");
    assert_eq!(binding.agent_id.as_deref(), Some("agent.memory.scope"));
    assert_eq!(
        binding.deployment_id.as_deref(),
        Some("deployment.memory.scope")
    );
    assert_eq!(binding.scope_ref, "deployment.memory.scope");
}

#[test]
fn memory_binding_scope_ref_rejects_unsafe_values_before_persistence() {
    let mut service = service();
    service
        .create_memory_store(memory_store_command(
            "memory.store.scope.bounds",
            "scope-bounds-memory",
        ))
        .expect("memory store should be created");
    service
        .create_memory_profile(AgentMemoryProfileCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            memory_profile_id: "memory.profile.scope.bounds".to_string(),
            memory_store_id: "memory.store.scope.bounds".to_string(),
            code: "scope-bounds-profile".to_string(),
            display_name: "Scope Bounds Profile".to_string(),
            description: Some("Memory profile used to verify safe scope refs".to_string()),
            write_policy_json: r#"{"mode":"curated"}"#.to_string(),
            retrieval_policy_json: r#"{"topK":8,"modes":["keyword"]}"#.to_string(),
            compaction_policy_json: r#"{"summaryAfterTurns":20}"#.to_string(),
            retention_policy_json: r#"{"defaultTtlDays":365}"#.to_string(),
            privacy_policy_json: r#"{"pii":"redact"}"#.to_string(),
            visibility: AgentVisibility::Tenant,
            requested_by: subject(),
            requested_at: "2026-06-04T04:10:00Z".to_string(),
        })
        .expect("memory profile should be created");

    let oversized_scope_ref = service
        .create_memory_binding(AgentMemoryBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_binding_id: "memory.binding.scope.oversized".to_string(),
            memory_profile_id: "memory.profile.scope.bounds".to_string(),
            agent_id: None,
            deployment_id: None,
            scope_kind: AgentMemoryBindingScopeKind::Tenant,
            scope_ref: "s".repeat(129),
            active: true,
            default_binding: false,
            requested_by: subject(),
            requested_at: "2026-06-04T04:11:00Z".to_string(),
        })
        .expect_err("129-character memory binding scopeRef must be rejected");
    assert_eq!(oversized_scope_ref.kind(), KernelErrorKind::ValidationError);

    let secret_scope_ref = service
        .create_memory_binding(AgentMemoryBindingCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_binding_id: "memory.binding.scope.secret".to_string(),
            memory_profile_id: "memory.profile.scope.bounds".to_string(),
            agent_id: None,
            deployment_id: None,
            scope_kind: AgentMemoryBindingScopeKind::Tenant,
            scope_ref: "tenant.memory.scope?api_key=secret".to_string(),
            active: true,
            default_binding: false,
            requested_by: subject(),
            requested_at: "2026-06-04T04:12:00Z".to_string(),
        })
        .expect_err("plaintext secret material in memory binding scopeRef must be rejected");
    assert_eq!(secret_scope_ref.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn memory_source_relation_and_retrieval_index_support_non_vector_rag_and_provenance() {
    let mut service = service();
    service
        .create_memory_store(memory_store_command(
            "memory.store.primary",
            "primary-memory",
        ))
        .expect("store should be created");
    service
        .create_memory_namespace(AgentMemoryNamespaceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_namespace_id: "memory.namespace.agent.alpha".to_string(),
            agent_id: Some("agent.research.alpha".to_string()),
            user_ref: None,
            session_ref: None,
            thread_ref: None,
            namespace_kind: AgentMemoryNamespaceKind::Agent,
            visibility: AgentVisibility::Organization,
            requested_by: subject(),
            requested_at: "2026-06-04T02:00:00Z".to_string(),
        })
        .expect("namespace should be created");
    for (memory_id, content, summary) in [
        (
            "memory.record.fact.project",
            r#"{"fact":"Project kernel requires a_ table prefix"}"#,
            "Project requires a_ table prefix",
        ),
        (
            "memory.record.fact.marketplace",
            r#"{"fact":"Skill, MCP, and prompt marketplaces are business entities"}"#,
            "Marketplaces are business entities",
        ),
    ] {
        service
            .create_memory_record(AgentMemoryRecordCreateCommand {
                tenant_id: 1,
                organization_id: 10,
                memory_id: memory_id.to_string(),
                memory_namespace_id: "memory.namespace.agent.alpha".to_string(),
                agent_id: Some("agent.research.alpha".to_string()),
                memory_kind: AgentMemoryRecordKind::Semantic,
                content_format: "application/json".to_string(),
                content_json: content.to_string(),
                summary: Some(summary.to_string()),
                salience_score: 0.8,
                confidence_score: 0.9,
                freshness_score: 1.0,
                sensitivity_level: 0,
                effective_at: None,
                expires_at: None,
                requested_by: subject(),
                requested_at: "2026-06-04T02:01:00Z".to_string(),
            })
            .expect("memory record should be created");
    }

    let source = service
        .create_memory_source(AgentMemorySourceCreateCommand {
            tenant_id: 1,
            memory_source_id: "memory.source.project.doc".to_string(),
            memory_id: "memory.record.fact.project".to_string(),
            source_kind: AgentMemorySourceKind::Document,
            source_ref: "document.kernel.standard".to_string(),
            source_hash: "sha256:abc123".to_string(),
            evidence_json: r#"{"quote":"tables use a_ prefix"}"#.to_string(),
            captured_at: "2026-06-04T02:02:00Z".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T02:02:00Z".to_string(),
        })
        .expect("memory source should be created");
    assert_eq!(source.source_kind, AgentMemorySourceKind::Document);
    let sources = service
        .list_memory_sources(1, "memory.record.fact.project", subject())
        .expect("memory sources should list by memory id");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].memory_source_id, "memory.source.project.doc");

    let relation = service
        .create_memory_relation(AgentMemoryRelationCreateCommand {
            tenant_id: 1,
            memory_relation_id: "memory.relation.project.supports.marketplace".to_string(),
            from_memory_id: "memory.record.fact.project".to_string(),
            to_memory_id: "memory.record.fact.marketplace".to_string(),
            relation_kind: AgentMemoryRelationKind::Supports,
            weight: 0.75,
            valid_from: Some("2026-06-04T02:03:00Z".to_string()),
            valid_until: None,
            requested_by: subject(),
            requested_at: "2026-06-04T02:03:00Z".to_string(),
        })
        .expect("memory relation should be created");
    assert_eq!(relation.relation_kind, AgentMemoryRelationKind::Supports);
    let relations = service
        .list_memory_relations(1, "memory.record.fact.project", subject())
        .expect("memory relations should list by memory id");
    assert_eq!(relations.len(), 1);
    assert_eq!(
        relations[0].memory_relation_id,
        "memory.relation.project.supports.marketplace"
    );

    let index = service
        .upsert_memory_retrieval_index(AgentMemoryRetrievalIndexUpsertCommand {
            tenant_id: 1,
            memory_index_id: "memory.index.project.wiki".to_string(),
            memory_id: "memory.record.fact.project".to_string(),
            index_kind: AgentMemoryIndexKind::Wiki,
            index_provider_id: "provider.memory.llm-wiki".to_string(),
            external_ref: "wiki://kernel/project-standard#memory.record.fact.project".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:def456".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T02:04:00Z".to_string(),
        })
        .expect("wiki retrieval index should be upserted");
    assert_eq!(index.index_kind, AgentMemoryIndexKind::Wiki);
    assert_eq!(index.embedding_model_id, None);
    assert_eq!(index.vector_dimension, None);

    let indexes = service
        .list_memory_retrieval_indexes(1, "memory.record.fact.project", subject())
        .expect("indexes should list");
    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].index_kind, AgentMemoryIndexKind::Wiki);
}

#[test]
fn memory_child_resources_require_active_parent_records() {
    let mut service = service();
    service
        .create_memory_namespace(AgentMemoryNamespaceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_namespace_id: "memory.namespace.deleted.parent".to_string(),
            agent_id: Some("agent.memory.parent".to_string()),
            user_ref: None,
            session_ref: None,
            thread_ref: None,
            namespace_kind: AgentMemoryNamespaceKind::Agent,
            visibility: AgentVisibility::Organization,
            requested_by: subject(),
            requested_at: "2026-06-04T05:00:00Z".to_string(),
        })
        .expect("namespace should be created");
    for memory_id in [
        "memory.record.deleted.parent",
        "memory.record.deleted.parent.peer",
    ] {
        service
            .create_memory_record(AgentMemoryRecordCreateCommand {
                tenant_id: 1,
                organization_id: 10,
                memory_id: memory_id.to_string(),
                memory_namespace_id: "memory.namespace.deleted.parent".to_string(),
                agent_id: Some("agent.memory.parent".to_string()),
                memory_kind: AgentMemoryRecordKind::Semantic,
                content_format: "application/json".to_string(),
                content_json: format!(r#"{{"fact":"{memory_id}"}}"#),
                summary: Some("Memory parent validation".to_string()),
                salience_score: 0.8,
                confidence_score: 0.9,
                freshness_score: 1.0,
                sensitivity_level: 0,
                effective_at: None,
                expires_at: None,
                requested_by: subject(),
                requested_at: "2026-06-04T05:01:00Z".to_string(),
            })
            .expect("memory record should be created");
    }
    let parent_record = service
        .get_memory_record(GetAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.record.deleted.parent".to_string(),
            requested_by: subject(),
        })
        .expect("memory record should exist");

    let index = service
        .upsert_memory_retrieval_index(AgentMemoryRetrievalIndexUpsertCommand {
            tenant_id: 1,
            memory_index_id: "memory.index.deleted.parent.before_delete".to_string(),
            memory_id: "memory.record.deleted.parent".to_string(),
            index_kind: AgentMemoryIndexKind::Wiki,
            index_provider_id: "provider.memory.llm-wiki".to_string(),
            external_ref: "wiki://memory/deleted-parent".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:deleted-parent".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T05:02:00Z".to_string(),
        })
        .expect("memory retrieval index should be created before deletion");
    assert_eq!(index.memory_id, "memory.record.deleted.parent");

    let deleted = service
        .delete_memory_record(DeleteAgentMarketplaceItemCommand {
            tenant_id: 1,
            item_id: "memory.record.deleted.parent".to_string(),
            expected_version: Some(parent_record.version),
            requested_by: subject(),
            requested_at: "2026-06-04T05:03:00Z".to_string(),
        })
        .expect("memory record should be deleted");
    assert_eq!(deleted.status, AgentBusinessStatus::Deleted);

    let source_error = service
        .create_memory_source(AgentMemorySourceCreateCommand {
            tenant_id: 1,
            memory_source_id: "memory.source.deleted.parent".to_string(),
            memory_id: "memory.record.deleted.parent".to_string(),
            source_kind: AgentMemorySourceKind::Document,
            source_ref: "document.deleted.parent".to_string(),
            source_hash: "sha256:deleted-parent-source".to_string(),
            evidence_json: r#"{"quote":"deleted parent"}"#.to_string(),
            captured_at: "2026-06-04T05:04:00Z".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T05:04:00Z".to_string(),
        })
        .expect_err("deleted memory record cannot receive provenance sources");
    assert_eq!(source_error.kind(), KernelErrorKind::ValidationError);

    let relation_error = service
        .create_memory_relation(AgentMemoryRelationCreateCommand {
            tenant_id: 1,
            memory_relation_id: "memory.relation.deleted.parent".to_string(),
            from_memory_id: "memory.record.deleted.parent".to_string(),
            to_memory_id: "memory.record.deleted.parent.peer".to_string(),
            relation_kind: AgentMemoryRelationKind::Supports,
            weight: 0.5,
            valid_from: None,
            valid_until: None,
            requested_by: subject(),
            requested_at: "2026-06-04T05:05:00Z".to_string(),
        })
        .expect_err("deleted memory record cannot be a relation endpoint");
    assert_eq!(relation_error.kind(), KernelErrorKind::ValidationError);

    let index_error = service
        .upsert_memory_retrieval_index(AgentMemoryRetrievalIndexUpsertCommand {
            tenant_id: 1,
            memory_index_id: "memory.index.deleted.parent.after_delete".to_string(),
            memory_id: "memory.record.deleted.parent".to_string(),
            index_kind: AgentMemoryIndexKind::Wiki,
            index_provider_id: "provider.memory.llm-wiki".to_string(),
            external_ref: "wiki://memory/deleted-parent-after-delete".to_string(),
            embedding_model_id: None,
            vector_dimension: None,
            content_hash: "sha256:deleted-parent-after-delete".to_string(),
            requested_by: subject(),
            requested_at: "2026-06-04T05:06:00Z".to_string(),
        })
        .expect_err("deleted memory record cannot receive retrieval indexes");
    assert_eq!(index_error.kind(), KernelErrorKind::ValidationError);

    let list_error = service
        .list_memory_retrieval_indexes(1, "memory.record.deleted.parent", subject())
        .expect_err("deleted memory record indexes should not be listed");
    assert_eq!(list_error.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn memory_records_reject_invalid_ids_plaintext_secrets_and_invalid_scores() {
    let mut service = service();

    let invalid_store = service
        .create_memory_store(memory_store_command("memory.bad", "bad-memory"))
        .expect_err("memory store id must use memory.store prefix");
    assert_eq!(invalid_store.kind(), KernelErrorKind::ValidationError);

    service
        .create_memory_namespace(AgentMemoryNamespaceCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_namespace_id: "memory.namespace.secure".to_string(),
            agent_id: None,
            user_ref: None,
            session_ref: None,
            thread_ref: None,
            namespace_kind: AgentMemoryNamespaceKind::Tenant,
            visibility: AgentVisibility::Tenant,
            requested_by: subject(),
            requested_at: "2026-06-04T03:00:00Z".to_string(),
        })
        .expect("namespace should be created");

    let secret_record = service
        .create_memory_record(AgentMemoryRecordCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            memory_id: "memory.record.secret".to_string(),
            memory_namespace_id: "memory.namespace.secure".to_string(),
            agent_id: None,
            memory_kind: AgentMemoryRecordKind::Semantic,
            content_format: "application/json".to_string(),
            content_json: r#"{"token":"sk-secret"}"#.to_string(),
            summary: Some("secret".to_string()),
            salience_score: 0.5,
            confidence_score: 0.5,
            freshness_score: 0.5,
            sensitivity_level: 3,
            effective_at: None,
            expires_at: None,
            requested_by: subject(),
            requested_at: "2026-06-04T03:01:00Z".to_string(),
        })
        .expect_err("plaintext secret memory content should fail");
    assert_eq!(secret_record.kind(), KernelErrorKind::ValidationError);

    let invalid_score = service
        .create_memory_record(AgentMemoryRecordCreateCommand {
            salience_score: 1.5,
            content_json: r#"{"fact":"valid"}"#.to_string(),
            ..AgentMemoryRecordCreateCommand {
                tenant_id: 1,
                organization_id: 10,
                memory_id: "memory.record.badscore".to_string(),
                memory_namespace_id: "memory.namespace.secure".to_string(),
                agent_id: None,
                memory_kind: AgentMemoryRecordKind::Semantic,
                content_format: "application/json".to_string(),
                content_json: r#"{"fact":"valid"}"#.to_string(),
                summary: Some("valid".to_string()),
                salience_score: 0.5,
                confidence_score: 0.5,
                freshness_score: 0.5,
                sensitivity_level: 0,
                effective_at: None,
                expires_at: None,
                requested_by: subject(),
                requested_at: "2026-06-04T03:02:00Z".to_string(),
            }
        })
        .expect_err("score outside 0..1 should fail");
    assert_eq!(invalid_score.kind(), KernelErrorKind::ValidationError);
}
