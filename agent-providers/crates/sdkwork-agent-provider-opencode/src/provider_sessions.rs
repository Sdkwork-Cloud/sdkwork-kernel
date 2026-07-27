use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    SessionState,
};
use sdkwork_agent_provider_core::{epoch_millis_to_rfc3339, SessionAdapter};
use serde_json::Value;

use crate::{OpenCodeAdapter, OpenCodeSession};

pub fn discover_opencode_provider_sessions() -> KernelResult<Vec<AgentSession>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let database_path = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    if !database_path.is_file() {
        return Ok(Vec::new());
    }
    read_opencode_provider_sessions(&database_path)
}

pub fn discover_opencode_provider_session_messages(
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let database_path = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    if !database_path.is_file() {
        return Ok(Vec::new());
    }
    read_opencode_provider_session_messages(&database_path, session_id)
}

pub fn read_opencode_provider_session_messages(
    database_path: &Path,
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(opencode_inventory_error)?;
    let mut statement = connection
        .prepare(
            "SELECT m.id, m.time_created, m.data, p.id, p.data \
             FROM message m JOIN part p ON p.message_id = m.id \
             WHERE m.session_id = ?1 \
             ORDER BY m.time_created, m.id, p.time_created, p.id",
        )
        .map_err(opencode_inventory_error)?;
    let rows = statement
        .query_map([session_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(opencode_inventory_error)?;
    let mut grouped = Vec::<(String, i64, Value, Vec<(String, Value)>)>::new();
    for row in rows {
        let (message_id, created_at, message_json, part_id, part_json) =
            row.map_err(opencode_inventory_error)?;
        let (Ok(message_data), Ok(part_data)) = (
            serde_json::from_str::<Value>(&message_json),
            serde_json::from_str::<Value>(&part_json),
        ) else {
            continue;
        };
        if let Some((_, _, _, parts)) = grouped
            .last_mut()
            .filter(|(current_message_id, _, _, _)| current_message_id == &message_id)
        {
            parts.push((part_id, part_data));
        } else {
            grouped.push((
                message_id,
                created_at,
                message_data,
                vec![(part_id, part_data)],
            ));
        }
    }
    Ok(grouped
        .into_iter()
        .filter_map(|(message_id, created_at, message_data, parts)| {
            let role = match message_data.get("role").and_then(Value::as_str) {
                Some("user") => AgentMessageRole::User,
                Some("assistant") => AgentMessageRole::Agent,
                Some("system") => AgentMessageRole::System,
                Some("tool") => AgentMessageRole::Tool,
                _ => return None,
            };
            let parts = parts
                .into_iter()
                .filter_map(|(part_id, part)| opencode_message_part(part_id, part))
                .collect::<Vec<_>>();
            if parts.is_empty() {
                return None;
            }
            let mut message = AgentMessage::new(message_id, role, parts).for_session(session_id);
            if let Some(timestamp) = epoch_millis_to_rfc3339(created_at) {
                message = message.created_at(timestamp);
            }
            Some(message)
        })
        .collect())
}

fn opencode_message_part(part_id: String, part: Value) -> Option<AgentPart> {
    let content_type = part.get("type").and_then(Value::as_str)?;
    match content_type {
        "text" | "reasoning" => {
            let text = part.get("text").and_then(Value::as_str)?.trim();
            (!text.is_empty()).then(|| {
                AgentPart::text(part_id, text)
                    .from_provider("opencode")
                    .with_metadata("opencode.content_type", content_type)
            })
        }
        "tool" => {
            let tool_call_id = part
                .get("callID")
                .or_else(|| part.get("callId"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())?;
            let tool_name = part
                .get("tool")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("tool");
            let status = part
                .pointer("/state/status")
                .and_then(Value::as_str)
                .unwrap_or("pending");
            let has_result = matches!(status, "completed" | "failed" | "error" | "cancelled")
                || part.pointer("/state/output").is_some()
                || part.pointer("/state/error").is_some();
            let mut agent_part = AgentPart::tool_call_ref(part_id, tool_call_id)
                .with_name(tool_name)
                .from_provider("opencode")
                .with_metadata("opencode.content_type", content_type)
                .with_metadata("opencode.status", status)
                .with_metadata(
                    "opencode.has_result",
                    if has_result { "true" } else { "false" },
                );
            agent_part.json = Some(part.to_string());
            Some(agent_part)
        }
        "patch" | "subtask" => {
            let tool_call_id = part
                .get("callID")
                .or_else(|| part.get("callId"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| part_id.clone());
            let tool_name = if content_type == "patch" {
                "apply_patch"
            } else {
                "task"
            };
            let mut agent_part = AgentPart::tool_call_ref(part_id, tool_call_id)
                .with_name(tool_name)
                .from_provider("opencode")
                .with_metadata("opencode.content_type", content_type)
                .with_metadata("opencode.status", "completed")
                .with_metadata("opencode.has_result", "true");
            agent_part.json = Some(part.to_string());
            Some(agent_part)
        }
        "file" | "step-start" | "step-finish" | "compaction" => Some(
            AgentPart::json(part_id, part.to_string())
                .from_provider("opencode")
                .with_metadata("opencode.content_type", content_type),
        ),
        _ => Some(
            AgentPart::json(part_id, part.to_string())
                .from_provider("opencode")
                .with_metadata("opencode.content_type", content_type),
        ),
    }
}

pub fn read_opencode_provider_sessions(path: &Path) -> KernelResult<Vec<AgentSession>> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(opencode_inventory_error)?;
    let mut statement = connection
        .prepare(
            "SELECT s.id, s.parent_id, s.title, s.time_created, s.time_updated, \
             s.time_archived, s.model, s.cost, s.tokens_input, s.tokens_output, \
             s.directory, p.worktree FROM session s JOIN project p ON p.id = s.project_id \
             ORDER BY s.time_updated DESC, s.id DESC",
        )
        .map_err(opencode_inventory_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, f64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        })
        .map_err(opencode_inventory_error)?;
    let adapter = OpenCodeAdapter::new();
    let mut sessions = Vec::new();
    for row in rows {
        let (
            id,
            parent_session_id,
            title,
            created_at,
            updated_at,
            archived_at,
            model_json,
            cost,
            input_tokens,
            output_tokens,
            directory,
            worktree,
        ) = row.map_err(opencode_inventory_error)?;
        let model = model_json.as_deref().and_then(opencode_model_id);
        let title = non_empty(title).or(read_first_opencode_user_message(&connection, &id)?);
        let mut session = adapter.to_agent_session(&OpenCodeSession {
            id,
            parent_session_id,
            title,
            message_count: 0,
            prompt_tokens: input_tokens.max(0) as u64,
            completion_tokens: output_tokens.max(0) as u64,
            cost_cents: Some((cost.max(0.0) * 100.0).round() as u64),
            created_at: epoch_millis_to_rfc3339(created_at),
            updated_at: epoch_millis_to_rfc3339(updated_at),
            model,
            cwd: Some(if directory.trim().is_empty() {
                worktree
            } else {
                directory
            }),
        })?;
        if let Some(archived_at) = archived_at.and_then(epoch_millis_to_rfc3339) {
            session.state = SessionState::Archived;
            session.archived_at = Some(archived_at);
        }
        sessions.push(session);
    }
    Ok(sessions)
}

fn read_first_opencode_user_message(
    connection: &Connection,
    session_id: &str,
) -> KernelResult<Option<String>> {
    let message_id = connection
        .query_row(
            "SELECT id FROM message WHERE session_id = ?1 \
             AND json_extract(data, '$.role') = 'user' \
             ORDER BY time_created, id LIMIT 1",
            [session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(opencode_inventory_error)?;
    let Some(message_id) = message_id else {
        return Ok(None);
    };
    let mut statement = connection
        .prepare(
            "SELECT json_extract(data, '$.text') FROM part WHERE message_id = ?1 \
             AND json_extract(data, '$.type') = 'text' ORDER BY time_created, id",
        )
        .map_err(opencode_inventory_error)?;
    let rows = statement
        .query_map([message_id], |row| row.get::<_, Option<String>>(0))
        .map_err(opencode_inventory_error)?;
    let mut parts = Vec::new();
    for row in rows {
        if let Some(text) = row.map_err(opencode_inventory_error)?.and_then(non_empty) {
            parts.push(text);
        }
    }
    Ok(non_empty(parts.join("\n\n")))
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn opencode_model_id(value: &str) -> Option<String> {
    serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|value| value.get("id").and_then(Value::as_str).map(str::to_string))
        .or_else(|| (!value.trim().is_empty()).then(|| value.to_string()))
}

fn opencode_inventory_error(error: rusqlite::Error) -> KernelError {
    KernelError::provider_error("opencode_provider_session_inventory", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sessions_joined_to_their_project_worktree() {
        let path = std::env::temp_dir().join(format!(
            "sdkwork-opencode-provider-sessions-{}.sqlite",
            std::process::id()
        ));
        let connection = Connection::open(&path).expect("fixture database");
        connection
            .execute_batch(
                "CREATE TABLE project (id TEXT PRIMARY KEY, worktree TEXT NOT NULL); \
                 CREATE TABLE session (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, \
                 parent_id TEXT, title TEXT NOT NULL, time_created INTEGER NOT NULL, \
                 time_updated INTEGER NOT NULL, time_archived INTEGER, model TEXT, \
                 cost REAL NOT NULL, tokens_input INTEGER NOT NULL, tokens_output INTEGER NOT NULL, \
                 directory TEXT NOT NULL); \
                 CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, \
                 time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL); \
                 CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL, \
                 session_id TEXT NOT NULL, time_created INTEGER NOT NULL, \
                 time_updated INTEGER NOT NULL, data TEXT NOT NULL); \
                 INSERT INTO project VALUES ('project-1', 'E:/Work/BirdCoder'); \
                 INSERT INTO session VALUES ('session-1', 'project-1', NULL, 'OpenCode session', \
                 0, 1000, NULL, '{\"id\":\"opencode-model\"}', 1.25, 10, 20, \
                 'E:/Work/BirdCoder'); \
                 INSERT INTO session VALUES ('session-2', 'project-1', NULL, '   ', \
                 50, 2000, NULL, '{\"id\":\"opencode-model\"}', 0, 0, 0, \
                 'E:/Work/BirdCoder'); \
                 INSERT INTO message VALUES ('message-user', 'session-1', 100, 100, \
                 '{\"role\":\"user\"}'); \
                 INSERT INTO message VALUES ('message-user-2', 'session-2', 50, 50, \
                 '{\"role\":\"user\"}'); \
                 INSERT INTO message VALUES ('message-assistant', 'session-1', 200, 200, \
                 '{\"role\":\"assistant\"}'); \
                 INSERT INTO part VALUES ('part-user', 'message-user', 'session-1', 100, 100, \
                 '{\"type\":\"text\",\"text\":\"hello\"}'); \
                 INSERT INTO part VALUES ('part-user-2', 'message-user-2', 'session-2', 50, 50, \
                 '{\"type\":\"text\",\"text\":\"fallback OpenCode prompt\"}'); \
                 INSERT INTO part VALUES ('part-assistant', 'message-assistant', 'session-1', 200, 200, \
                 '{\"type\":\"text\",\"text\":\"world\"}'); \
                 INSERT INTO part VALUES ('part-reasoning', 'message-assistant', 'session-1', 201, 201, \
                 '{\"type\":\"reasoning\",\"text\":\"inspect the code\"}'); \
                 INSERT INTO part VALUES ('part-tool', 'message-assistant', 'session-1', 202, 202, \
                 '{\"type\":\"tool\",\"callID\":\"call-1\",\"tool\":\"mcp__docs__search\",\"state\":{\"status\":\"completed\",\"input\":{\"q\":\"session\"},\"output\":\"found\"}}'); \
                 INSERT INTO part VALUES ('part-patch', 'message-assistant', 'session-1', 203, 203, \
                 '{\"type\":\"patch\",\"hash\":\"patch-1\",\"files\":[\"src/main.ts\"]}'); \
                 INSERT INTO part VALUES ('part-step-finish', 'message-assistant', 'session-1', 204, 204, \
                 '{\"type\":\"step-finish\",\"reason\":\"stop\"}');",
            )
            .expect("fixture data");
        drop(connection);

        let sessions = read_opencode_provider_sessions(&path).expect("provider sessions");
        assert_eq!(sessions.len(), 2);
        let titled = sessions
            .iter()
            .find(|session| session.session_id == "session-1")
            .expect("provider-titled session");
        assert_eq!(titled.title.as_deref(), Some("OpenCode session"));
        assert_eq!(titled.cost_cents, Some(125));
        let fallback = sessions
            .iter()
            .find(|session| session.session_id == "session-2")
            .expect("fallback-titled session");
        assert_eq!(fallback.title.as_deref(), Some("fallback OpenCode prompt"));
        assert!(sessions.iter().all(|session| {
            session.activity.freshness
                == sdkwork_agent_kernel::SessionActivityFreshness::Unsupported
        }));
        let messages = read_opencode_provider_session_messages(&path, "session-1")
            .expect("provider transcript");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, AgentMessageRole::User);
        assert_eq!(messages[1].role, AgentMessageRole::Agent);
        assert_eq!(messages[1].parts.len(), 5);
        assert_eq!(
            messages[1].parts[1].metadata_value("opencode.content_type"),
            Some("reasoning")
        );
        assert_eq!(messages[1].parts[2].tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(
            messages[1].parts[2].name.as_deref(),
            Some("mcp__docs__search")
        );
        assert_eq!(
            messages[1].parts[2].metadata_value("opencode.has_result"),
            Some("true")
        );
        assert_eq!(
            messages[1].parts[3].metadata_value("opencode.content_type"),
            Some("patch")
        );
        assert_eq!(
            messages[1].parts[4].metadata_value("opencode.content_type"),
            Some("step-finish")
        );
        std::fs::remove_file(path).expect("remove fixture");
    }
}
