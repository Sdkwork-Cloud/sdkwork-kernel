//! SQLite contract tests for agent runtime persistence.

use sdkwork_agent_database::{
    DatabaseError, EventQuery, EventRepository, MessageRepository, MessageRow,
    PermissionRepository, PermissionRow, RuntimeSessionWrites, SessionRepository, SqliteDatabase,
    TaskRepository,
};

fn migrated_sqlite() -> SqliteDatabase {
    SqliteDatabase::memory_migrated().expect("sqlite")
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
