//! Live PostgreSQL contract tests for agent runtime session persistence.

use sdkwork_agent_database::{
    EventRepository, MessageRepository, MessageRow, PermissionRepository, PermissionRow,
    PostgresDatabase, RuntimeSessionWrites, SessionRepository,
};

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
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    })
    .expect("save session");

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
        .load_messages(
            &session_id,
            &sdkwork_agent_database::MessageQuery::default(),
        )
        .expect("messages");
    assert_eq!(messages.len(), 1);

    let _ = db.delete_session_cascade(&session_id);
}

#[test]
fn live_postgres_update_session_preserves_messages_when_uri_configured() {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let session_id = format!("session.runtime.pg.upsert.{}", uuid_like_suffix());

    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.clone(),
        agent_id: "agent.runtime".to_string(),
        kind: "main".to_string(),
        source: "contract-test".to_string(),
        state: "active".to_string(),
        title: None,
        model: None,
        cwd: None,
        provider_id: None,
        bridge_id: None,
        token_usage_json: None,
        message_count: 0,
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    })
    .expect("save session");

    db.save_message(&MessageRow {
        message_id: format!("msg.{session_id}"),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "must survive session update".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    })
    .expect("save message");

    let mut session = db.load_session(&session_id).expect("load").expect("found");
    session.state = "closed".to_string();
    session.message_count = 1;
    session.updated_at = Some("2026-06-23T00:00:02Z".to_string());
    db.update_session(&session).expect("update session");

    let messages = db
        .load_messages(
            &session_id,
            &sdkwork_agent_database::MessageQuery::default(),
        )
        .expect("messages");
    assert_eq!(messages.len(), 1);

    let _ = db.delete_session_cascade(&session_id);
}

#[test]
fn live_postgres_save_message_rejects_duplicate_message_id_with_different_content_when_uri_configured(
) {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let session_id = format!("session.runtime.pg.save-conflict.{}", uuid_like_suffix());

    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.clone(),
        agent_id: "agent.runtime".to_string(),
        kind: "main".to_string(),
        source: "contract-test".to_string(),
        state: "active".to_string(),
        title: None,
        model: None,
        cwd: None,
        provider_id: None,
        bridge_id: None,
        token_usage_json: None,
        message_count: 0,
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    })
    .expect("save session");

    let message = MessageRow {
        message_id: format!("msg.save-conflict.{}", uuid_like_suffix()),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "original payload".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    db.save_message(&message).expect("first save");

    let conflicting = MessageRow {
        content: "changed payload".to_string(),
        ..message.clone()
    };
    let error = db
        .save_message(&conflicting)
        .expect_err("duplicate message id with changed payload must fail");
    assert!(matches!(
        error,
        sdkwork_agent_database::DatabaseError::ConstraintViolation(_)
    ));

    let messages = db
        .load_messages(
            &session_id,
            &sdkwork_agent_database::MessageQuery::default(),
        )
        .expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "original payload");

    let _ = db.delete_session_cascade(&session_id);
}

#[test]
fn live_postgres_save_message_rejects_duplicate_message_id_for_different_session_when_uri_configured(
) {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let first_session_id = format!("session.runtime.pg.save-conflict-a.{}", uuid_like_suffix());
    let second_session_id = format!("session.runtime.pg.save-conflict-b.{}", uuid_like_suffix());

    for session_id in [&first_session_id, &second_session_id] {
        db.save_session(&sdkwork_agent_database::SessionRow {
            session_id: session_id.clone(),
            agent_id: "agent.runtime".to_string(),
            kind: "main".to_string(),
            source: "contract-test".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-06-23T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        })
        .expect("save session");
    }

    let message = MessageRow {
        message_id: format!("msg.save-cross-session.{}", uuid_like_suffix()),
        session_id: first_session_id.clone(),
        role: "user".to_string(),
        content: "original payload".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    db.save_message(&message).expect("first save");

    let conflicting = MessageRow {
        session_id: second_session_id.clone(),
        ..message
    };
    let error = db
        .save_message(&conflicting)
        .expect_err("duplicate message id must not move across sessions");
    assert!(matches!(
        error,
        sdkwork_agent_database::DatabaseError::ConstraintViolation(_)
    ));

    assert_eq!(
        db.load_messages(
            &first_session_id,
            &sdkwork_agent_database::MessageQuery::default()
        )
        .expect("first messages")
        .len(),
        1
    );
    assert!(db
        .load_messages(
            &second_session_id,
            &sdkwork_agent_database::MessageQuery::default()
        )
        .expect("second messages")
        .is_empty());

    let _ = db.delete_session_cascade(&first_session_id);
    let _ = db.delete_session_cascade(&second_session_id);
}

#[test]
fn live_postgres_permissions_roundtrip_when_uri_configured() {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let permission_id = format!("perm.runtime.pg.{}", uuid_like_suffix());

    db.save_permission(&PermissionRow {
        permission_request_id: permission_id.clone(),
        session_id: Some("session.runtime.pg".to_string()),
        category: "host.filesystem.read".to_string(),
        resource: "/workspace/README.md".to_string(),
        side_effect_level: "read_only".to_string(),
        reason: "contract test".to_string(),
        status: "pending".to_string(),
        owner_tenant_id: Some("tenant.contract".to_string()),
        owner_user_ref: Some("user.contract".to_string()),
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
    })
    .expect("save permission");

    let loaded = db
        .load_permission(&permission_id)
        .expect("load")
        .expect("found");
    assert_eq!(loaded.status, "pending");

    let listed = db
        .list_permissions(&sdkwork_agent_database::PermissionQuery {
            status: Some("pending".to_string()),
            limit: Some(20),
            offset: None,
            ..Default::default()
        })
        .expect("list");
    assert!(listed
        .iter()
        .any(|row| row.permission_request_id == permission_id));

    let _ = db.update_permission_status(&permission_id, "approved");
}

#[test]
fn live_postgres_append_message_with_event_is_idempotent_when_uri_configured() {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let session_id = format!("session.runtime.pg.idempotent.{}", uuid_like_suffix());

    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.clone(),
        agent_id: "agent.runtime".to_string(),
        kind: "main".to_string(),
        source: "contract-test".to_string(),
        state: "active".to_string(),
        title: None,
        model: None,
        cwd: None,
        provider_id: None,
        bridge_id: None,
        token_usage_json: None,
        message_count: 0,
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    })
    .expect("save session");

    let message = MessageRow {
        message_id: format!("msg.{session_id}"),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "retry-safe append".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    let event = sdkwork_agent_database::EventRow {
        event_id: format!("evt.{session_id}"),
        session_id: Some(session_id.clone()),
        event_type: "message.sent".to_string(),
        severity: "info".to_string(),
        payload: None,
        created_at: "2026-06-23T00:00:01Z".to_string(),
    };

    let first_count = db
        .append_message_with_event(&message, &event)
        .expect("first append");
    let retry_count = db
        .append_message_with_event(&message, &event)
        .expect("retry append");

    assert_eq!(first_count, 1);
    assert_eq!(retry_count, 1);
    assert_eq!(db.message_count(&session_id).expect("message count"), 1);
    assert_eq!(
        db.load_session(&session_id)
            .expect("load")
            .expect("found")
            .message_count,
        1
    );
    assert_eq!(
        db.load_events(&session_id, &Default::default())
            .expect("events")
            .len(),
        1
    );

    let _ = db.delete_session_cascade(&session_id);
}

#[test]
fn live_postgres_append_message_with_event_does_not_write_event_for_duplicate_message_id_when_uri_configured(
) {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let session_id = format!("session.runtime.pg.duplicate-event.{}", uuid_like_suffix());

    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.clone(),
        agent_id: "agent.runtime".to_string(),
        kind: "main".to_string(),
        source: "contract-test".to_string(),
        state: "active".to_string(),
        title: None,
        model: None,
        cwd: None,
        provider_id: None,
        bridge_id: None,
        token_usage_json: None,
        message_count: 0,
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    })
    .expect("save session");

    let message = MessageRow {
        message_id: format!("msg.duplicate-event.{}", uuid_like_suffix()),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "retry-safe append".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    let first_event = sdkwork_agent_database::EventRow {
        event_id: format!("evt.duplicate-event-a.{}", uuid_like_suffix()),
        session_id: Some(session_id.clone()),
        event_type: "message.sent".to_string(),
        severity: "info".to_string(),
        payload: None,
        created_at: "2026-06-23T00:00:01Z".to_string(),
    };
    let retry_event = sdkwork_agent_database::EventRow {
        event_id: format!("evt.duplicate-event-b.{}", uuid_like_suffix()),
        created_at: "2026-06-23T00:00:02Z".to_string(),
        ..first_event.clone()
    };

    db.append_message_with_event(&message, &first_event)
        .expect("first append");
    let retry_count = db
        .append_message_with_event(&message, &retry_event)
        .expect("retry append");

    let events = db
        .load_events(&session_id, &Default::default())
        .expect("events");
    assert_eq!(retry_count, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, first_event.event_id);

    let _ = db.delete_session_cascade(&session_id);
}

#[test]
fn live_postgres_append_message_with_event_rejects_cross_session_duplicate_when_uri_configured() {
    let Some(uri) = runtime_postgres_uri() else {
        return;
    };

    let db = PostgresDatabase::connect_migrated(&uri).expect("postgres");
    let first_session_id = format!("session.runtime.pg.conflict-a.{}", uuid_like_suffix());
    let second_session_id = format!("session.runtime.pg.conflict-b.{}", uuid_like_suffix());

    for session_id in [&first_session_id, &second_session_id] {
        db.save_session(&sdkwork_agent_database::SessionRow {
            session_id: session_id.clone(),
            agent_id: "agent.runtime".to_string(),
            kind: "main".to_string(),
            source: "contract-test".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-06-23T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        })
        .expect("save session");
    }

    let message = MessageRow {
        message_id: format!("msg.conflict.{}", uuid_like_suffix()),
        session_id: first_session_id.clone(),
        role: "user".to_string(),
        content: "original append".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    let event = sdkwork_agent_database::EventRow {
        event_id: format!("evt.conflict.{}", uuid_like_suffix()),
        session_id: Some(first_session_id.clone()),
        event_type: "message.sent".to_string(),
        severity: "info".to_string(),
        payload: None,
        created_at: "2026-06-23T00:00:01Z".to_string(),
    };
    db.append_message_with_event(&message, &event)
        .expect("first append");

    let conflicting_message = MessageRow {
        session_id: second_session_id.clone(),
        content: "conflicting append".to_string(),
        ..message
    };
    let conflicting_event = sdkwork_agent_database::EventRow {
        event_id: format!("evt.conflict.{}", uuid_like_suffix()),
        session_id: Some(second_session_id.clone()),
        ..event
    };

    let error = db
        .append_message_with_event(&conflicting_message, &conflicting_event)
        .expect_err("duplicate message id must not move across sessions");
    assert!(matches!(
        error,
        sdkwork_agent_database::DatabaseError::ConstraintViolation(_)
    ));
    assert_eq!(db.message_count(&first_session_id).expect("first count"), 1);
    assert_eq!(
        db.message_count(&second_session_id).expect("second count"),
        0
    );
    assert!(db
        .load_events(&second_session_id, &Default::default())
        .expect("events")
        .is_empty());

    let _ = db.delete_session_cascade(&first_session_id);
    let _ = db.delete_session_cascade(&second_session_id);
}

fn uuid_like_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{nanos}")
}
