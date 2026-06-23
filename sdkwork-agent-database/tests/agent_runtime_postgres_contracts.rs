//! Live PostgreSQL contract tests for agent runtime session persistence.

use sdkwork_agent_database::{MessageRow, PostgresDatabase, SessionRepository};

fn runtime_postgres_uri() -> Option<String> {
    std::env::var("SDKWORK_AGENT_RUNTIME_POSTGRES_URI")
        .or_else(|_| std::env::var("SDKWORK_AGENT_BUSINESS_POSTGRES_URI"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[test]
fn live_postgres_session_message_roundtrip_when_uri_configured() {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let session_id = format!("session.runtime.pg.{}", uuid_like_suffix());

    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.clone(),
        agent_id: "agent.runtime".to_string(),
        kind: "main".to_string(),
        source: "contract-test".to_string(),
        state: "active".to_string(),
        title: Some("runtime contract".to_string()),
        model: None,
        cwd: None,
        provider_id: Some("hermes".to_string()),
        bridge_id: None,
        token_usage_json: None,
        message_count: 0,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    })
    .expect("save session");

    use sdkwork_agent_database::MessageRepository;
    db.save_message(&MessageRow {
        message_id: format!("msg.{session_id}"),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "runtime postgres contract".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    })
    .expect("save message");

    let loaded = db.load_session(&session_id).expect("load").expect("found");
    assert_eq!(loaded.agent_id, "agent.runtime");

    let messages = db
        .load_messages(&session_id, &sdkwork_agent_database::MessageQuery::default())
        .expect("messages");
    assert_eq!(messages.len(), 1);

    let _ = db.delete_session(&session_id);
}

fn uuid_like_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{nanos}")
}
