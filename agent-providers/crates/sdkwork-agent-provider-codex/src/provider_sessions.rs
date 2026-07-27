use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    SessionState,
};
use sdkwork_agent_provider_core::{epoch_millis_to_rfc3339, SessionAdapter};
use serde_json::Value;

use crate::{CodexAdapter, CodexSessionMeta};

pub fn discover_codex_provider_sessions() -> KernelResult<Vec<AgentSession>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let state_path = PathBuf::from(home).join(".codex").join("state_5.sqlite");
    if !state_path.is_file() {
        return Ok(Vec::new());
    }
    read_codex_provider_sessions(&state_path)
}

pub fn discover_codex_provider_session_messages(
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let state_path = PathBuf::from(home).join(".codex").join("state_5.sqlite");
    if !state_path.is_file() {
        return Ok(Vec::new());
    }
    read_codex_provider_session_messages(&state_path, session_id)
}

pub fn read_codex_provider_session_messages(
    state_path: &Path,
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let connection = Connection::open_with_flags(
        state_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(codex_inventory_error)?;
    let rollout_path = connection
        .query_row(
            "SELECT rollout_path FROM threads WHERE id = ?1 LIMIT 1",
            [session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(codex_inventory_error)?;
    let Some(rollout_path) = rollout_path else {
        return Ok(Vec::new());
    };
    read_codex_rollout_messages(Path::new(&rollout_path), session_id)
}

fn read_codex_rollout_messages(
    rollout_path: &Path,
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let file = File::open(rollout_path).map_err(codex_transcript_io_error)?;
    let mut messages = Vec::new();
    let mut seen_message_ids = HashSet::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(codex_transcript_io_error)?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        let role = match payload.get("role").and_then(Value::as_str) {
            Some("user") => AgentMessageRole::User,
            Some("assistant") => AgentMessageRole::Agent,
            _ => continue,
        };
        let Some(message_id) = payload
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if !seen_message_ids.insert(message_id.to_string()) {
            continue;
        }
        let content = payload
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                Some("input_text" | "output_text") => part.get("text").and_then(Value::as_str),
                _ => None,
            })
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        if content.is_empty() {
            continue;
        }
        let mut message = AgentMessage::new(
            message_id,
            role,
            vec![AgentPart::text(format!("{message_id}.text"), content)],
        )
        .for_session(session_id);
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            message = message.created_at(timestamp);
        }
        messages.push(message);
    }
    Ok(messages)
}

pub fn read_codex_provider_sessions(path: &Path) -> KernelResult<Vec<AgentSession>> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(codex_inventory_error)?;
    let mut statement = connection
        .prepare(
            "SELECT id, created_at_ms, updated_at_ms, model_provider, cwd, title, archived, \
             archived_at, model, reasoning_effort, agent_nickname, agent_role, approval_mode, \
             preview, tokens_used, name, first_user_message \
             FROM threads ORDER BY updated_at_ms DESC, id DESC",
        )
        .map_err(codex_inventory_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, i64>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, String>(16)?,
            ))
        })
        .map_err(codex_inventory_error)?;
    let adapter = CodexAdapter::new();
    let mut sessions = Vec::new();
    for row in rows {
        let (
            id,
            created_at_ms,
            updated_at_ms,
            model_provider,
            cwd,
            title,
            archived,
            archived_at_ms,
            model,
            reasoning_effort,
            agent_nickname,
            role,
            approval_policy,
            preview,
            tokens_used,
            name,
            first_user_message,
        ) = row.map_err(codex_inventory_error)?;
        let mut session = adapter.to_agent_session(&CodexSessionMeta {
            id,
            forked_from_id: None,
            parent_thread_id: None,
            timestamp: created_at_ms.and_then(epoch_millis_to_rfc3339),
            cwd: Some(cwd),
            originator: None,
            model,
            model_provider: Some(model_provider),
            agent_nickname,
            role,
            reasoning_effort,
            approval_policy: Some(approval_policy),
        })?;
        session.title = codex_provider_session_title(name, title, first_user_message, &preview);
        session.preview = non_empty(preview);
        session.updated_at = updated_at_ms.and_then(epoch_millis_to_rfc3339);
        session.token_usage.total_tokens = tokens_used.max(0) as u64;
        if archived != 0 {
            session.state = SessionState::Archived;
            session.archived_at = archived_at_ms.and_then(epoch_millis_to_rfc3339);
        }
        sessions.push(session);
    }
    Ok(sessions)
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn codex_provider_session_title(
    name: Option<String>,
    legacy_title: String,
    first_user_message: String,
    preview: &str,
) -> Option<String> {
    let name = name.and_then(non_empty);
    let legacy_title = non_empty(legacy_title);
    let first_user_message = non_empty(first_user_message);
    let preview = non_empty(preview.to_string());
    let provider_title = legacy_title
        .as_ref()
        .filter(|title| {
            first_user_message.as_deref() != Some(title.as_str())
                && preview.as_deref() != Some(title.as_str())
        })
        .cloned();

    name.or(provider_title)
        .or(first_user_message)
        .or(preview)
        .or(legacy_title)
}

fn codex_inventory_error(error: rusqlite::Error) -> KernelError {
    KernelError::provider_error("codex_provider_session_inventory", error.to_string())
}

fn codex_transcript_io_error(error: std::io::Error) -> KernelError {
    KernelError::provider_error("codex_provider_session_transcript", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_complete_inventory_beyond_two_hundred_rows() {
        let path = std::env::temp_dir().join(format!(
            "sdkwork-codex-provider-sessions-{}.sqlite",
            std::process::id()
        ));
        let mut connection = Connection::open(&path).expect("fixture database");
        connection
            .execute_batch(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, created_at_ms INTEGER, \
                 updated_at_ms INTEGER, model_provider TEXT NOT NULL, cwd TEXT NOT NULL, \
                 title TEXT NOT NULL, archived INTEGER NOT NULL, archived_at INTEGER, \
                 model TEXT, reasoning_effort TEXT, agent_nickname TEXT, agent_role TEXT, \
                 approval_mode TEXT NOT NULL, preview TEXT NOT NULL, tokens_used INTEGER NOT NULL, \
                 name TEXT, first_user_message TEXT NOT NULL);",
            )
            .expect("fixture schema");
        let transaction = connection.transaction().expect("fixture transaction");
        for index in 0..225 {
            transaction
                .execute(
                    "INSERT INTO threads VALUES (?1, 0, ?2, 'provider.model.codex', \
                     '\\\\?\\E:\\Work\\BirdCoder', ?3, 0, NULL, 'gpt-5', NULL, NULL, \
                     NULL, 'on-request', ?4, 0, ?5, ?6)",
                    rusqlite::params![
                        format!("thread-{index}"),
                        index,
                        if index == 223 {
                            String::new()
                        } else {
                            format!("Session {index}")
                        },
                        format!("Prompt {index}"),
                        (index == 224).then_some("Renamed provider thread"),
                        format!("Prompt {index}"),
                    ],
                )
                .expect("fixture row");
        }
        transaction.commit().expect("fixture commit");
        drop(connection);

        let sessions = read_codex_provider_sessions(&path).expect("provider sessions");
        assert_eq!(sessions.len(), 225);
        assert_eq!(sessions[0].cwd.as_deref(), Some(r"\\?\E:\Work\BirdCoder"));
        assert_eq!(
            sessions[0].title.as_deref(),
            Some("Renamed provider thread")
        );
        assert_eq!(sessions[1].title.as_deref(), Some("Prompt 223"));
        assert!(sessions.iter().all(|session| {
            session.activity.freshness
                == sdkwork_agent_kernel::SessionActivityFreshness::Unsupported
        }));
        std::fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn reads_visible_user_and_assistant_messages_from_rollout() {
        let root = std::env::temp_dir().join(format!(
            "sdkwork-codex-provider-session-transcript-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("fixture directory");
        let rollout_path = root.join("session-1.jsonl");
        std::fs::write(
            &rollout_path,
            concat!(
                "{\"timestamp\":\"2026-07-01T00:00:00Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"message-user\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"reasoning\",\"id\":\"reasoning-1\"}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:02Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"message-assistant\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"world\"}]}}\n"
            ),
        )
        .expect("fixture rollout");
        let state_path = root.join("state.sqlite");
        let connection = Connection::open(&state_path).expect("fixture database");
        connection
            .execute(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, rollout_path TEXT NOT NULL)",
                [],
            )
            .expect("fixture schema");
        connection
            .execute(
                "INSERT INTO threads VALUES (?1, ?2)",
                rusqlite::params!["session-1", rollout_path.to_string_lossy().as_ref()],
            )
            .expect("fixture thread");
        drop(connection);

        let messages = read_codex_provider_session_messages(&state_path, "session-1")
            .expect("provider transcript");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, AgentMessageRole::User);
        assert_eq!(messages[0].parts[0].text.as_deref(), Some("hello"));
        assert_eq!(messages[1].role, AgentMessageRole::Agent);
        assert_eq!(messages[1].parts[0].text.as_deref(), Some("world"));
        std::fs::remove_dir_all(root).expect("remove fixture");
    }
}
