//! SQLite contract tests for agent runtime persistence.

use sdkwork_agent_database::{
    DatabaseError, EventQuery, EventRepository, EventRow, MessageQuery, MessageRepository,
    MessageRow, PermissionQuery, PermissionRepository, PermissionRow, RuntimeMaintenance,
    RuntimeSessionWrites, SessionRepository, SessionRow, SqliteDatabase, TaskRepository,
};

fn migrated_sqlite() -> SqliteDatabase {
    SqliteDatabase::memory_migrated().expect("sqlite")
}

fn scoped_session(session_id: &str, tenant: &str, user: &str) -> SessionRow {
    SessionRow {
        session_id: session_id.to_string(),
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
        owner_tenant_id: Some(tenant.to_string()),
        owner_user_ref: Some(user.to_string()),
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
        metadata_json: None,
    }
}

#[test]
fn sqlite_session_message_roundtrip() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.1".to_string();

    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.clone(),
        agent_id: "agent.runtime".to_string(),
        kind: "main".to_string(),
        source: "contract-test".to_string(),
        state: "active".to_string(),
        title: Some("runtime contract".to_string()),
        model: None,
        cwd: None,
        provider_id: Some("rig".to_string()),
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
        content: "runtime sqlite contract".to_string(),
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

    db.delete_session_cascade(&session_id).expect("cascade");
    assert!(db.load_session(&session_id).expect("load").is_none());
}

#[test]
fn sqlite_cascade_delete_removes_permissions() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.perm".to_string();

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

    let permission_id = "perm.runtime.sqlite.1".to_string();
    db.save_permission(&PermissionRow {
        permission_request_id: permission_id.clone(),
        session_id: Some(session_id.clone()),
        category: "host.filesystem.read".to_string(),
        resource: "/workspace/README.md".to_string(),
        side_effect_level: "read_only".to_string(),
        reason: "contract test".to_string(),
        status: "pending".to_string(),
        owner_tenant_id: None,
        owner_user_ref: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
        updated_at: None,
    })
    .expect("save permission");

    db.delete_session_cascade(&session_id).expect("cascade");
    assert!(db
        .load_permission(&permission_id)
        .expect("load permission")
        .is_none());
}

#[test]
fn sqlite_event_replay_after_event_id_is_strict() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.events".to_string();

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

    for (event_id, created_at) in [
        ("evt.1", "2026-06-23T00:00:01Z"),
        ("evt.2", "2026-06-23T00:00:02Z"),
        ("evt.3", "2026-06-23T00:00:03Z"),
    ] {
        db.save_event(&sdkwork_agent_database::EventRow {
            event_id: event_id.to_string(),
            session_id: Some(session_id.clone()),
            event_type: "test.event".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: created_at.to_string(),
        })
        .expect("save event");
    }

    let after_first = db
        .load_events(
            &session_id,
            &EventQuery {
                after_event_id: Some("evt.1".to_string()),
                limit: Some(10),
                ..EventQuery::default()
            },
        )
        .expect("after first");
    assert_eq!(after_first.len(), 2);
    assert_eq!(after_first[0].event_id, "evt.2");

    let missing_cursor = db
        .load_events(
            &session_id,
            &EventQuery {
                after_event_id: Some("evt.missing".to_string()),
                limit: Some(10),
                ..EventQuery::default()
            },
        )
        .expect("missing cursor");
    assert!(missing_cursor.is_empty());

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_message_list_after_message_id_is_strict() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.messages".to_string();

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

    for (message_id, created_at) in [
        ("msg.1", "2026-06-23T00:00:01Z"),
        ("msg.2", "2026-06-23T00:00:02Z"),
        ("msg.3", "2026-06-23T00:00:03Z"),
    ] {
        db.save_message(&MessageRow {
            message_id: message_id.to_string(),
            session_id: session_id.clone(),
            role: "user".to_string(),
            content: message_id.to_string(),
            created_at: created_at.to_string(),
            metadata_json: None,
        })
        .expect("save message");
    }

    let after_first = db
        .load_messages(
            &session_id,
            &sdkwork_agent_database::MessageQuery {
                after_message_id: Some("msg.1".to_string()),
                limit: Some(10),
                ..Default::default()
            },
        )
        .expect("after first");
    assert_eq!(after_first.len(), 2);
    assert_eq!(after_first[0].message_id, "msg.2");

    let missing_cursor = db
        .load_messages(
            &session_id,
            &sdkwork_agent_database::MessageQuery {
                after_message_id: Some("msg.missing".to_string()),
                limit: Some(10),
                ..Default::default()
            },
        )
        .expect("missing cursor");
    assert!(missing_cursor.is_empty());

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_session_list_after_session_id_is_strict() {
    let db = migrated_sqlite();
    for (session_id, created_at) in [
        ("session.a", "2026-06-23T00:00:03Z"),
        ("session.b", "2026-06-23T00:00:02Z"),
        ("session.c", "2026-06-23T00:00:01Z"),
    ] {
        db.save_session(&sdkwork_agent_database::SessionRow {
            session_id: session_id.to_string(),
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
            created_at: created_at.to_string(),
            updated_at: Some(created_at.to_string()),
            metadata_json: None,
        })
        .expect("save session");
    }

    let after_first = db
        .list_sessions(&sdkwork_agent_database::SessionQuery {
            after_session_id: Some("session.a".to_string()),
            limit: Some(10),
            ..Default::default()
        })
        .expect("after first");
    assert_eq!(after_first.len(), 2);
    assert_eq!(after_first[0].session_id, "session.b");

    let missing_cursor = db
        .list_sessions(&sdkwork_agent_database::SessionQuery {
            after_session_id: Some("session.missing".to_string()),
            limit: Some(10),
            ..Default::default()
        })
        .expect("missing cursor");
    assert!(missing_cursor.is_empty());

    for session_id in ["session.a", "session.b", "session.c"] {
        db.delete_session_cascade(session_id).expect("cascade");
    }
}

#[test]
fn sqlite_task_list_after_task_id_is_strict() {
    let db = migrated_sqlite();
    let session_id = "session.tasks";
    db.save_session(&sdkwork_agent_database::SessionRow {
        session_id: session_id.to_string(),
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

    for (task_id, created_at) in [
        ("task.1", "2026-06-23T00:00:01Z"),
        ("task.2", "2026-06-23T00:00:02Z"),
        ("task.3", "2026-06-23T00:00:03Z"),
    ] {
        db.save_task(&sdkwork_agent_database::TaskRow {
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            instruction: task_id.to_string(),
            state: "pending".to_string(),
            created_at: created_at.to_string(),
            updated_at: None,
        })
        .expect("save task");
    }

    let after_first = db
        .load_tasks(
            session_id,
            &sdkwork_agent_database::TaskQuery {
                after_task_id: Some("task.1".to_string()),
                limit: Some(10),
                ..Default::default()
            },
        )
        .expect("after first");
    assert_eq!(after_first.len(), 2);
    assert_eq!(after_first[0].task_id, "task.2");

    let missing_cursor = db
        .load_tasks(
            session_id,
            &sdkwork_agent_database::TaskQuery {
                after_task_id: Some("task.missing".to_string()),
                limit: Some(10),
                ..Default::default()
            },
        )
        .expect("missing cursor");
    assert!(missing_cursor.is_empty());

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_update_session_preserves_messages() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.upsert".to_string();

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
        message_id: "msg.preserve.1".to_string(),
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
    assert_eq!(messages[0].message_id, "msg.preserve.1");

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_save_message_rejects_duplicate_message_id_with_different_content() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.save-conflict".to_string();

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
        message_id: "msg.sqlite.save-conflict.1".to_string(),
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
    assert!(matches!(error, DatabaseError::ConstraintViolation(_)));

    let messages = db
        .load_messages(
            &session_id,
            &sdkwork_agent_database::MessageQuery::default(),
        )
        .expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "original payload");

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_save_message_rejects_duplicate_message_id_for_different_session() {
    let db = migrated_sqlite();
    let first_session_id = "session.runtime.sqlite.save-conflict-a".to_string();
    let second_session_id = "session.runtime.sqlite.save-conflict-b".to_string();

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
        message_id: "msg.sqlite.save-cross-session.1".to_string(),
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
    assert!(matches!(error, DatabaseError::ConstraintViolation(_)));

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

    db.delete_session_cascade(&first_session_id)
        .expect("cascade");
    db.delete_session_cascade(&second_session_id)
        .expect("cascade");
}

#[test]
fn sqlite_append_message_with_event_is_idempotent_for_duplicate_message_id() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.idempotent-append".to_string();

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
        message_id: "msg.idempotent.1".to_string(),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "retry-safe append".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    let event = sdkwork_agent_database::EventRow {
        event_id: "evt.idempotent.1".to_string(),
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
    let session = db.load_session(&session_id).expect("load").expect("found");
    assert_eq!(session.message_count, 1);
    assert_eq!(
        db.load_messages(&session_id, &Default::default())
            .expect("messages")
            .len(),
        1
    );
    assert_eq!(
        db.load_events(&session_id, &EventQuery::default())
            .expect("events")
            .len(),
        1
    );

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_append_message_with_event_does_not_write_event_for_duplicate_message_id() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.duplicate-event".to_string();

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
        message_id: "msg.sqlite.duplicate-event.1".to_string(),
        session_id: session_id.clone(),
        role: "user".to_string(),
        content: "retry-safe append".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    let first_event = sdkwork_agent_database::EventRow {
        event_id: "evt.sqlite.duplicate-event.1".to_string(),
        session_id: Some(session_id.clone()),
        event_type: "message.sent".to_string(),
        severity: "info".to_string(),
        payload: None,
        created_at: "2026-06-23T00:00:01Z".to_string(),
    };
    let retry_event = sdkwork_agent_database::EventRow {
        event_id: "evt.sqlite.duplicate-event.2".to_string(),
        created_at: "2026-06-23T00:00:02Z".to_string(),
        ..first_event.clone()
    };

    db.append_message_with_event(&message, &first_event)
        .expect("first append");
    let retry_count = db
        .append_message_with_event(&message, &retry_event)
        .expect("retry append");

    let events = db
        .load_events(&session_id, &EventQuery::default())
        .expect("events");
    assert_eq!(retry_count, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "evt.sqlite.duplicate-event.1");

    db.delete_session_cascade(&session_id).expect("cascade");
}

#[test]
fn sqlite_append_message_with_event_rejects_duplicate_message_id_for_different_session() {
    let db = migrated_sqlite();
    let first_session_id = "session.runtime.sqlite.conflict-a".to_string();
    let second_session_id = "session.runtime.sqlite.conflict-b".to_string();

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
        message_id: "msg.conflict.1".to_string(),
        session_id: first_session_id.clone(),
        role: "user".to_string(),
        content: "original append".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        metadata_json: None,
    };
    let event = sdkwork_agent_database::EventRow {
        event_id: "evt.conflict.1".to_string(),
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
        event_id: "evt.conflict.2".to_string(),
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
    assert_eq!(
        db.load_messages(&first_session_id, &Default::default())
            .expect("first messages")
            .len(),
        1
    );
    assert!(db
        .load_messages(&second_session_id, &Default::default())
        .expect("second messages")
        .is_empty());
    assert!(db
        .load_events(&second_session_id, &EventQuery::default())
        .expect("second events")
        .is_empty());

    db.delete_session_cascade(&first_session_id)
        .expect("cascade a");
    db.delete_session_cascade(&second_session_id)
        .expect("cascade b");
}

#[test]
fn sqlite_message_cursor_is_stable_for_equal_timestamps() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.equal-time";
    db.save_session(&scoped_session(session_id, "tenant.a", "user.a"))
        .expect("session");

    for message_id in ["msg.z", "msg.a"] {
        db.save_message(&MessageRow {
            message_id: message_id.to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: message_id.to_string(),
            created_at: "2026-06-23T00:00:01Z".to_string(),
            metadata_json: None,
        })
        .expect("message");
    }

    let first = db
        .load_messages(
            session_id,
            &MessageQuery {
                limit: Some(1),
                ..Default::default()
            },
        )
        .expect("first page");
    assert_eq!(first[0].message_id, "msg.a");

    let second = db
        .load_messages(
            session_id,
            &MessageQuery {
                after_message_id: Some(first[0].message_id.clone()),
                limit: Some(1),
                ..Default::default()
            },
        )
        .expect("second page");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].message_id, "msg.z");
}

#[test]
fn sqlite_recent_messages_returns_bounded_tail_in_chronological_order() {
    let db = migrated_sqlite();
    let session_id = "session.runtime.sqlite.recent";
    db.save_session(&scoped_session(session_id, "tenant.a", "user.a"))
        .expect("session");
    for index in 1..=6 {
        db.save_message(&MessageRow {
            message_id: format!("msg.{index}"),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: index.to_string(),
            created_at: format!("2026-06-23T00:00:0{index}Z"),
            metadata_json: None,
        })
        .expect("message");
    }

    let recent = db
        .load_recent_messages(session_id, 3)
        .expect("recent messages");
    assert_eq!(
        recent
            .iter()
            .map(|row| row.message_id.as_str())
            .collect::<Vec<_>>(),
        vec!["msg.4", "msg.5", "msg.6"]
    );
    assert!(db.load_recent_messages(session_id, 0).is_err());
    assert!(db.load_recent_messages(session_id, 513).is_err());
}

#[test]
fn sqlite_event_and_permission_queries_enforce_tenant_user_scope() {
    let db = migrated_sqlite();
    for (session_id, tenant, user) in [
        ("session.scope.a", "tenant.a", "user.a"),
        ("session.scope.b", "tenant.b", "user.b"),
    ] {
        db.save_session(&scoped_session(session_id, tenant, user))
            .expect("session");
        db.save_event(&EventRow {
            event_id: format!("evt.{tenant}"),
            session_id: Some(session_id.to_string()),
            event_type: "scope.test".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: "2026-06-23T00:00:01Z".to_string(),
        })
        .expect("event");
        db.save_permission(&PermissionRow {
            permission_request_id: format!("perm.{tenant}"),
            session_id: Some(session_id.to_string()),
            category: "host.filesystem.read".to_string(),
            resource: "/workspace".to_string(),
            side_effect_level: "read_only".to_string(),
            reason: "scope test".to_string(),
            status: "pending".to_string(),
            owner_tenant_id: Some(tenant.to_string()),
            owner_user_ref: Some(user.to_string()),
            created_at: "2026-06-23T00:00:01Z".to_string(),
            updated_at: None,
        })
        .expect("permission");
    }
    db.save_event(&EventRow {
        event_id: "evt.global".to_string(),
        session_id: None,
        event_type: "scope.test".to_string(),
        severity: "info".to_string(),
        payload: None,
        created_at: "2026-06-23T00:00:02Z".to_string(),
    })
    .expect("global event");

    let events = db
        .list_recent_events(&EventQuery {
            owner_tenant_id: Some("tenant.a".to_string()),
            owner_user_ref: Some("user.a".to_string()),
            limit: Some(20),
            ..Default::default()
        })
        .expect("scoped events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "evt.tenant.a");

    let permissions = db
        .list_permissions(&PermissionQuery {
            status: Some("pending".to_string()),
            owner_tenant_id: Some("tenant.a".to_string()),
            owner_user_ref: Some("user.a".to_string()),
            limit: Some(20),
            ..Default::default()
        })
        .expect("scoped permissions");
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions[0].permission_request_id, "perm.tenant.a");
}

#[test]
fn sqlite_session_and_task_event_writes_roll_back_together() {
    let db = migrated_sqlite();
    let session = scoped_session("session.atomic", "tenant.a", "user.a");
    let invalid_event = EventRow {
        event_id: "evt.atomic.invalid".to_string(),
        session_id: Some("session.missing".to_string()),
        event_type: "session.created".to_string(),
        severity: "info".to_string(),
        payload: None,
        created_at: "2026-06-23T00:00:00Z".to_string(),
    };
    assert!(db
        .save_session_with_event(&session, &invalid_event)
        .is_err());
    assert!(db
        .load_session(&session.session_id)
        .expect("session lookup")
        .is_none());

    db.save_session(&session).expect("parent session");
    let task = sdkwork_agent_database::TaskRow {
        task_id: "task.atomic".to_string(),
        session_id: session.session_id.clone(),
        instruction: "verify transaction".to_string(),
        state: "created".to_string(),
        created_at: "2026-06-23T00:00:01Z".to_string(),
        updated_at: None,
    };
    assert!(db.save_task_with_event(&task, &invalid_event).is_err());
    assert!(db.load_task(&task.task_id).expect("task lookup").is_none());
}

#[test]
fn sqlite_concurrent_startup_migration_is_idempotent() {
    let path = std::env::temp_dir().join(format!(
        "sdkwork-agent-runtime-migration-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let path_text = path.to_string_lossy().into_owned();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let barrier = barrier.clone();
        let path_text = path_text.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            SqliteDatabase::open_migrated(&path_text)
        }));
    }
    for handle in handles {
        handle
            .join()
            .expect("migration thread")
            .expect("concurrent migration");
    }
    SqliteDatabase::open_migrated(&path_text).expect("repeat migration");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

#[test]
fn sqlite_retention_is_bounded_transactional_and_preserves_live_work() {
    let db = migrated_sqlite();
    let mut expired_session = scoped_session("session.retention.closed", "tenant.a", "user.a");
    expired_session.state = "closed".to_string();
    expired_session.created_at = "2020-01-01T00:00:00Z".to_string();
    expired_session.updated_at = Some("2020-01-02T00:00:00Z".to_string());
    expired_session.message_count = 1;
    db.save_session(&expired_session).expect("expired session");

    let mut live_session = scoped_session("session.retention.live", "tenant.a", "user.a");
    live_session.created_at = "2020-01-01T00:00:00Z".to_string();
    live_session.updated_at = Some("2026-01-01T00:00:00Z".to_string());
    live_session.message_count = 1;
    db.save_session(&live_session).expect("live session");

    for (message_id, session_id) in [
        (
            "message.retention.closed",
            expired_session.session_id.as_str(),
        ),
        ("message.retention.live", live_session.session_id.as_str()),
    ] {
        db.save_message(&MessageRow {
            message_id: message_id.to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: "expired".to_string(),
            created_at: "2020-01-01T00:00:01Z".to_string(),
            metadata_json: None,
        })
        .expect("message");
        db.save_event(&EventRow {
            event_id: format!("event.{message_id}"),
            session_id: Some(session_id.to_string()),
            event_type: "retention.test".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: "2020-01-01T00:00:01Z".to_string(),
        })
        .expect("event");
    }

    for (task_id, session_id, state) in [
        (
            "task.retention.closed",
            expired_session.session_id.as_str(),
            "completed",
        ),
        (
            "task.retention.live",
            live_session.session_id.as_str(),
            "completed",
        ),
        (
            "task.retention.pending",
            live_session.session_id.as_str(),
            "pending",
        ),
    ] {
        db.save_task(&sdkwork_agent_database::TaskRow {
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            instruction: "retention".to_string(),
            state: state.to_string(),
            created_at: "2020-01-01T00:00:01Z".to_string(),
            updated_at: None,
        })
        .expect("task");
    }

    for (permission_id, session_id, status) in [
        (
            "permission.retention.closed",
            expired_session.session_id.as_str(),
            "pending",
        ),
        (
            "permission.retention.live",
            live_session.session_id.as_str(),
            "denied",
        ),
        (
            "permission.retention.pending",
            live_session.session_id.as_str(),
            "pending",
        ),
    ] {
        db.save_permission(&PermissionRow {
            permission_request_id: permission_id.to_string(),
            session_id: Some(session_id.to_string()),
            category: "host.filesystem.read".to_string(),
            resource: "/workspace".to_string(),
            side_effect_level: "read_only".to_string(),
            reason: "retention".to_string(),
            status: status.to_string(),
            owner_tenant_id: Some("tenant.a".to_string()),
            owner_user_ref: Some("user.a".to_string()),
            created_at: "2020-01-01T00:00:01Z".to_string(),
            updated_at: None,
        })
        .expect("permission");
    }
    db.save_permission(&PermissionRow {
        permission_request_id: "permission.retention.global.pending".to_string(),
        session_id: None,
        category: "host.filesystem.read".to_string(),
        resource: "/workspace".to_string(),
        side_effect_level: "read_only".to_string(),
        reason: "retention".to_string(),
        status: "pending".to_string(),
        owner_tenant_id: Some("tenant.a".to_string()),
        owner_user_ref: Some("user.a".to_string()),
        created_at: "2020-01-01T00:00:01Z".to_string(),
        updated_at: None,
    })
    .expect("global pending permission");

    let counts = db
        .purge_expired("2021-01-01T00:00:00Z", 100)
        .expect("retention purge");
    assert_eq!(counts.sessions, 1);
    assert_eq!(counts.messages, 2);
    assert_eq!(counts.tasks, 2);
    assert_eq!(counts.events, 2);
    assert_eq!(counts.permissions, 2);
    assert!(db
        .load_session(&expired_session.session_id)
        .expect("expired lookup")
        .is_none());
    assert_eq!(
        db.load_session(&live_session.session_id)
            .expect("live lookup")
            .expect("live session")
            .message_count,
        0
    );
    assert!(db
        .load_task("task.retention.pending")
        .expect("pending task")
        .is_some());
    assert!(db
        .load_permission("permission.retention.pending")
        .expect("pending permission")
        .is_some());
    assert!(db
        .load_permission("permission.retention.global.pending")
        .expect("global pending permission")
        .is_some());

    let schema = db.schema_status().expect("schema status");
    assert!(schema.drift_free);
    assert_eq!(schema.version, schema.expected_version);
    db.run_maintenance().expect("sqlite maintenance");
}
