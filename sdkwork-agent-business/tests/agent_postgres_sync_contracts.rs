#![cfg(feature = "postgres-sync")]

use sdkwork_agent_business::{
    AgentMemoryRelationKind, AgentMemoryRelationRecord, AgentRepository, PostgresAgentRepository,
    SyncPostgresAdapter, SQL_LIST_AGENT_MEMORY_STORES, SQL_SELECT_AGENT_MEMORY_BINDING,
    SQL_SELECT_AGENT_MEMORY_NAMESPACE, SQL_SELECT_AGENT_MEMORY_PROFILE, SQL_SELECT_AGENT_MEMORY_STORE,
    SQL_SELECT_AGENT_MEMORY_RELATION, SQL_SELECT_AGENT_MEMORY_RETRIEVAL_INDEX,
    SQL_SELECT_AGENT_MEMORY_SOURCE, SQL_UPDATE_AGENT_MEMORY_BINDING, SQL_UPDATE_AGENT_MEMORY_NAMESPACE,
    SQL_UPDATE_AGENT_MEMORY_PROFILE,
};

fn tenant_scoped_select_sql(sql: &str, table: &str) {
    assert!(
        sql.contains("WHERE tenant_id = $1"),
        "{table} select SQL must filter by tenant_id"
    );
    assert!(
        sql.contains("LIMIT 1"),
        "{table} get-by-id select SQL should be bounded"
    );
}

fn tenant_scoped_list_sql(sql: &str, table: &str) {
    assert!(
        sql.contains("WHERE tenant_id = $1"),
        "{table} list SQL must filter by tenant_id"
    );
}

fn tenant_scoped_update_sql(sql: &str, table: &str) {
    assert!(
        sql.contains("WHERE tenant_id ="),
        "{table} update SQL must filter by tenant_id"
    );
    assert!(
        sql.contains("version ="),
        "{table} update SQL must enforce optimistic concurrency"
    );
}

#[test]
fn postgres_memory_get_select_sql_is_tenant_scoped() {
    tenant_scoped_select_sql(SQL_SELECT_AGENT_MEMORY_SOURCE, "a_agent_memory_source");
    tenant_scoped_select_sql(SQL_SELECT_AGENT_MEMORY_RELATION, "a_agent_memory_relation");
    tenant_scoped_select_sql(
        SQL_SELECT_AGENT_MEMORY_RETRIEVAL_INDEX,
        "a_agent_memory_retrieval_index",
    );
    tenant_scoped_select_sql(SQL_SELECT_AGENT_MEMORY_STORE, "a_agent_memory_store");
    tenant_scoped_select_sql(SQL_SELECT_AGENT_MEMORY_PROFILE, "a_agent_memory_profile");
    tenant_scoped_select_sql(SQL_SELECT_AGENT_MEMORY_BINDING, "a_agent_memory_binding");
    tenant_scoped_select_sql(SQL_SELECT_AGENT_MEMORY_NAMESPACE, "a_agent_memory_namespace");
}

#[test]
fn postgres_memory_list_and_update_sql_is_tenant_scoped() {
    tenant_scoped_list_sql(SQL_LIST_AGENT_MEMORY_STORES, "a_agent_memory_store");
    tenant_scoped_update_sql(SQL_UPDATE_AGENT_MEMORY_PROFILE, "a_agent_memory_profile");
    tenant_scoped_update_sql(SQL_UPDATE_AGENT_MEMORY_BINDING, "a_agent_memory_binding");
    tenant_scoped_update_sql(SQL_UPDATE_AGENT_MEMORY_NAMESPACE, "a_agent_memory_namespace");
}

#[test]
fn live_postgres_memory_relation_get_roundtrip_when_uri_configured() {
    let Some(uri) = std::env::var("SDKWORK_AGENT_BUSINESS_POSTGRES_URI")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!(
            "skipping live postgres sync test: SDKWORK_AGENT_BUSINESS_POSTGRES_URI is not set"
        );
        return;
    };

    let adapter = SyncPostgresAdapter::connect(uri.as_str())
        .expect("postgres adapter should connect when URI is configured");
    adapter
        .apply_business_schema()
        .expect("postgres schema should apply");

    let mut repository = PostgresAgentRepository::new(adapter);
    let id = repository.next_id().expect("snowflake id should generate");
    let memory_relation_id = format!("memory.relation.postgres.sync.{id}");
    let relation = AgentMemoryRelationRecord {
        id,
        tenant_id: 1,
        memory_relation_id: memory_relation_id.clone(),
        from_memory_id: "memory.record.postgres.sync.a".to_string(),
        to_memory_id: "memory.record.postgres.sync.b".to_string(),
        relation_kind: AgentMemoryRelationKind::Supports,
        weight: 0.5,
        valid_from: None,
        valid_until: None,
        created_at: "2026-06-17T00:00:00Z".to_string(),
    };

    repository
        .insert_memory_relation(relation.clone())
        .expect("memory relation insert should succeed");

    let loaded = repository
        .get_memory_relation(1, memory_relation_id.as_str())
        .expect("memory relation should load from postgres");
    assert_eq!(loaded.memory_relation_id, relation.memory_relation_id);
    assert_eq!(loaded.from_memory_id, relation.from_memory_id);
    assert_eq!(loaded.to_memory_id, relation.to_memory_id);

    let other_tenant = repository.get_memory_relation(2, memory_relation_id.as_str());
    assert_eq!(
        other_tenant, None,
        "tenant isolation should hide cross-tenant rows"
    );
}
