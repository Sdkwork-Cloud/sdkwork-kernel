use sdkwork_agent_business::{
    AgentBusinessIdGenerator, AgentBusinessService, AgentIdGenerator, AgentMcpAuthKind,
    AgentMcpServerCreateCommand, AgentMcpTransportKind, AgentVisibility, AllowAllPolicyProvider,
    InMemoryAgentAuditSink, InMemoryAgentRepository, SQL_INSERT_AGENT_DEPLOYMENT,
    SQL_INSERT_AGENT_KNOWLEDGE_BASE, SQL_INSERT_AGENT_KNOWLEDGE_BINDING,
    SQL_INSERT_AGENT_KNOWLEDGE_CHUNK, SQL_INSERT_AGENT_KNOWLEDGE_DOCUMENT,
    SQL_INSERT_AGENT_KNOWLEDGE_SOURCE, SQL_INSERT_AGENT_KNOWLEDGE_SYNC_JOB,
    SQL_INSERT_AGENT_MEMORY_BINDING, SQL_INSERT_AGENT_MEMORY_RELATION,
    SQL_INSERT_AGENT_MEMORY_SOURCE, SQL_INSERT_AGENT_PROVIDER_BINDING, SQL_INSERT_AUDIT_EVENT,
    SQL_UPSERT_AGENT_KNOWLEDGE_INDEX, SQL_UPSERT_AGENT_MEMORY_RETRIEVAL_INDEX,
};
use sdkwork_agent_kernel::PolicySubject;

fn subject() -> PolicySubject {
    PolicySubject::new("user.id.admin", "tenant.1").with_role("agent.id.admin")
}

#[test]
fn agent_business_id_generator_uses_signed_safe_snowflake_ids() {
    let generator = AgentBusinessIdGenerator::with_node_id(42)
        .expect("valid snowflake node id should initialize");

    let first = generator
        .next_id()
        .expect("first snowflake id should generate");
    let second = generator
        .next_id()
        .expect("second snowflake id should generate");

    assert!(first > 0);
    assert!(second > first);
    assert!(first <= i64::MAX as u64);
    assert_eq!(42, AgentBusinessIdGenerator::decode_node_id(first));
    assert!(AgentBusinessIdGenerator::decode_timestamp_delta_millis(first) > 0);
}

#[test]
fn runtime_services_assign_snowflake_ids_instead_of_local_counters() {
    let mut service = AgentBusinessService::new(
        InMemoryAgentRepository::new(),
        InMemoryAgentAuditSink::default(),
        AllowAllPolicyProvider::allow("provider.policy.id"),
    );

    let created = service
        .create_mcp_server(AgentMcpServerCreateCommand {
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            mcp_server_id: "mcp.server.id.standard".to_string(),
            code: "id-standard-mcp".to_string(),
            display_name: "ID Standard MCP".to_string(),
            description: None,
            protocol_version: "2026-06-05".to_string(),
            transport_kind: AgentMcpTransportKind::Http,
            endpoint_ref: Some("endpoint.mcp.id-standard".to_string()),
            command_ref: None,
            auth_kind: AgentMcpAuthKind::None,
            auth_profile_id: None,
            capability_ids: vec!["mcp.tools".to_string()],
            tool_count: 1,
            resource_count: 0,
            prompt_count: 0,
            capabilities_json: r#"{"tools":true}"#.to_string(),
            categories: vec!["identity".to_string()],
            tags: vec!["snowflake".to_string()],
            security_profile_id: None,
            visibility: AgentVisibility::Tenant,
            requested_by: subject(),
            requested_at: "2026-06-05T00:00:00Z".to_string(),
        })
        .expect("mcp server should be created");

    assert!(created.id > (1_u64 << 22));
    assert_ne!(created.id, 1);
}

#[test]
fn postgres_runtime_insert_sql_binds_ids_explicitly_for_all_business_tables() {
    for (name, sql) in [
        (
            "a_agent_provider_binding",
            SQL_INSERT_AGENT_PROVIDER_BINDING,
        ),
        ("a_agent_deployment", SQL_INSERT_AGENT_DEPLOYMENT),
        ("a_agent_business_audit_event", SQL_INSERT_AUDIT_EVENT),
        ("a_agent_memory_binding", SQL_INSERT_AGENT_MEMORY_BINDING),
        ("a_agent_memory_source", SQL_INSERT_AGENT_MEMORY_SOURCE),
        ("a_agent_memory_relation", SQL_INSERT_AGENT_MEMORY_RELATION),
        ("a_agent_knowledge_base", SQL_INSERT_AGENT_KNOWLEDGE_BASE),
        (
            "a_agent_knowledge_source",
            SQL_INSERT_AGENT_KNOWLEDGE_SOURCE,
        ),
        (
            "a_agent_knowledge_document",
            SQL_INSERT_AGENT_KNOWLEDGE_DOCUMENT,
        ),
        ("a_agent_knowledge_chunk", SQL_INSERT_AGENT_KNOWLEDGE_CHUNK),
        ("a_agent_knowledge_index", SQL_UPSERT_AGENT_KNOWLEDGE_INDEX),
        (
            "a_agent_knowledge_binding",
            SQL_INSERT_AGENT_KNOWLEDGE_BINDING,
        ),
        (
            "a_agent_knowledge_sync_job",
            SQL_INSERT_AGENT_KNOWLEDGE_SYNC_JOB,
        ),
        (
            "a_agent_memory_retrieval_index",
            SQL_UPSERT_AGENT_MEMORY_RETRIEVAL_INDEX,
        ),
    ] {
        let prefix = format!("INSERT INTO {name} (id, ");
        assert!(
            sql.starts_with(&prefix),
            "{name} insert must bind snowflake id explicitly: {sql}"
        );
        assert!(
            !sql.contains("nextval") && !sql.contains("RETURNING id"),
            "{name} insert must not allocate ids through database side effects"
        );
    }
}

#[test]
fn postgres_agent_business_schema_does_not_define_database_generated_ids() {
    let ddl = include_str!("../specs/sql/agent_business_postgres.sql");

    assert!(!ddl.contains("GENERATED BY DEFAULT AS IDENTITY"));
    assert!(!ddl.contains("BIGSERIAL"));
    assert!(!ddl.contains("SERIAL PRIMARY KEY"));
    assert!(!ddl.contains("nextval("));

    for table in [
        "a_agent_business",
        "a_agent_provider_binding",
        "a_agent_deployment",
        "a_agent_business_audit_event",
        "a_agent_mcp_server",
        "a_agent_knowledge_base",
        "a_agent_knowledge_source",
        "a_agent_knowledge_document",
        "a_agent_knowledge_chunk",
        "a_agent_knowledge_index",
        "a_agent_knowledge_binding",
        "a_agent_knowledge_sync_job",
        "a_agent_memory_store",
        "a_agent_memory_profile",
        "a_agent_memory_binding",
        "a_agent_memory_namespace",
        "a_agent_memory_record",
        "a_agent_memory_source",
        "a_agent_memory_relation",
        "a_agent_memory_retrieval_index",
    ] {
        let create_table = format!("CREATE TABLE IF NOT EXISTS {table} (");
        assert!(ddl.contains(&create_table), "missing table DDL for {table}");
    }
}
