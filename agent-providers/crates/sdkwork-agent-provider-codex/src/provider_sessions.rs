use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
    SessionState,
};
use sdkwork_agent_provider_core::{SessionAdapter, epoch_millis_to_rfc3339};
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
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(codex_transcript_io_error)?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(mut message) = codex_rollout_message(&value, session_id, line_index) else {
            continue;
        };
        if !seen_message_ids.insert(message.message_id.clone()) {
            continue;
        }
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            message = message.created_at(timestamp);
        }
        messages.push(message);
    }
    Ok(messages)
}

fn codex_rollout_message(
    value: &Value,
    session_id: &str,
    line_index: usize,
) -> Option<AgentMessage> {
    let payload = value.get("payload")?;
    match value.get("type").and_then(Value::as_str) {
        Some("response_item") => codex_response_item_message(payload, session_id),
        Some("event_msg") => codex_event_message(payload, session_id, line_index),
        _ => None,
    }
}

fn codex_response_item_message(payload: &Value, session_id: &str) -> Option<AgentMessage> {
    let payload_type = payload.get("type").and_then(Value::as_str)?;
    let message_id = codex_payload_id(payload, payload_type)?;
    let (role, parts) = match payload_type {
        "message" => (
            codex_message_role(payload.get("role").and_then(Value::as_str))?,
            codex_message_content_parts(payload, &message_id),
        ),
        "reasoning" => (
            AgentMessageRole::Agent,
            codex_reasoning_parts(payload, &message_id),
        ),
        "function_call"
        | "custom_tool_call"
        | "image_generation_call"
        | "local_shell_call"
        | "tool_search_call"
        | "web_search_call" => (
            AgentMessageRole::Agent,
            vec![codex_tool_call_part(payload, &message_id, payload_type)?],
        ),
        "function_call_output" | "custom_tool_call_output" | "tool_search_output" => (
            AgentMessageRole::Tool,
            vec![codex_tool_result_part(payload, &message_id, payload_type)?],
        ),
        "agent_message" => (
            AgentMessageRole::Agent,
            codex_agent_message_parts(payload, &message_id),
        ),
        _ => return None,
    };
    (!parts.is_empty()).then(|| {
        AgentMessage::new(message_id, role, parts)
            .for_session(session_id)
            .with_metadata("codex.item_type", payload_type)
    })
}

fn codex_event_message(
    payload: &Value,
    session_id: &str,
    line_index: usize,
) -> Option<AgentMessage> {
    let event_type = payload.get("type").and_then(Value::as_str)?;
    let fallback_id = format!("codex.event.{line_index}");
    let message_id = payload
        .get("event_id")
        .or_else(|| payload.get("call_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("codex.event.{event_type}.{value}"))
        .unwrap_or(fallback_id);
    let (role, parts) = match event_type {
        "agent_reasoning" => {
            let text = payload.get("text").and_then(Value::as_str)?.trim();
            if text.is_empty() {
                return None;
            }
            (
                AgentMessageRole::Agent,
                vec![
                    AgentPart::text(format!("{message_id}.reasoning"), text)
                        .from_provider("codex")
                        .with_metadata("codex.content_type", "reasoning"),
                ],
            )
        }
        "mcp_tool_call_end" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "mcp_tool_call",
                "mcp",
            )?],
        ),
        "plan_update" => (
            AgentMessageRole::Agent,
            vec![codex_plan_update_part(payload, session_id, &message_id)],
        ),
        "exec_command_end" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "command_execution",
                "shell_command",
            )?],
        ),
        "web_search_end" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "web_search",
                "web_search",
            )?],
        ),
        "dynamic_tool_call_response" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "dynamic_tool_call",
                "tool",
            )?],
        ),
        "view_image_tool_call" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "image_view",
                "view_image",
            )?],
        ),
        "image_generation_end" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "image_generation",
                "image_generation",
            )?],
        ),
        "patch_apply_end" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "file_change",
                "apply_patch",
            )?],
        ),
        "sub_agent_activity" => (
            AgentMessageRole::Agent,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "sub_agent_activity",
                "subagent_activity",
            )?],
        ),
        "collab_agent_spawn_end"
        | "collab_agent_interaction_end"
        | "collab_waiting_end"
        | "collab_close_end"
        | "collab_resume_end" => (
            AgentMessageRole::Tool,
            vec![codex_completed_tool_event_part(
                payload,
                &message_id,
                "collab_agent_tool_call",
                "agent",
            )?],
        ),
        "task_started" | "task_complete" | "context_compacted" => (
            AgentMessageRole::Adapter,
            vec![
                AgentPart::json(format!("{message_id}.status"), payload.to_string())
                    .from_provider("codex")
                    .with_metadata("codex.content_type", event_type),
            ],
        ),
        "turn_aborted" => {
            let reason = payload
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("Codex turn aborted");
            (
                AgentMessageRole::Adapter,
                vec![
                    AgentPart::error(format!("{message_id}.error"), "turn_aborted", reason)
                        .from_provider("codex")
                        .with_metadata("codex.content_type", "error"),
                ],
            )
        }
        _ => return None,
    };
    Some(
        AgentMessage::new(message_id, role, parts)
            .for_session(session_id)
            .with_metadata("codex.event_type", event_type),
    )
}

fn codex_payload_id(payload: &Value, payload_type: &str) -> Option<String> {
    if let Some(id) = payload
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(id.to_string());
    }

    payload
        .get("call_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("codex.{payload_type}.{value}"))
}

fn codex_message_role(role: Option<&str>) -> Option<AgentMessageRole> {
    match role {
        Some("user") => Some(AgentMessageRole::User),
        Some("assistant") => Some(AgentMessageRole::Agent),
        Some("system" | "developer") => Some(AgentMessageRole::System),
        Some("tool") => Some(AgentMessageRole::Tool),
        _ => None,
    }
}

fn codex_message_content_parts(payload: &Value, message_id: &str) -> Vec<AgentPart> {
    payload
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, content)| {
            let content_type = content.get("type").and_then(Value::as_str)?;
            let part_id = format!("{message_id}.content.{index}");
            match content_type {
                "input_text" | "output_text" => {
                    let text = content.get("text").and_then(Value::as_str)?.trim();
                    (!text.is_empty()).then(|| {
                        AgentPart::text(part_id, text)
                            .from_provider("codex")
                            .with_metadata("codex.content_type", content_type)
                    })
                }
                _ => Some(
                    AgentPart::json(part_id, content.to_string())
                        .from_provider("codex")
                        .with_metadata("codex.content_type", content_type),
                ),
            }
        })
        .collect()
}

fn codex_reasoning_parts(payload: &Value, message_id: &str) -> Vec<AgentPart> {
    payload
        .get("summary")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, summary)| {
            let text = summary
                .get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?;
            Some(
                AgentPart::text(format!("{message_id}.summary.{index}"), text)
                    .from_provider("codex")
                    .with_metadata("codex.content_type", "reasoning"),
            )
        })
        .collect()
}

fn codex_agent_message_parts(payload: &Value, message_id: &str) -> Vec<AgentPart> {
    let Some(content) = payload.get("content") else {
        return Vec::new();
    };
    match content {
        Value::String(text) if !text.trim().is_empty() => {
            vec![
                AgentPart::text(format!("{message_id}.text"), text.trim())
                    .from_provider("codex")
                    .with_metadata("codex.content_type", "agent_message"),
            ]
        }
        Value::Array(_) => codex_message_content_parts(payload, message_id),
        _ => Vec::new(),
    }
}

fn codex_tool_call_part(
    payload: &Value,
    message_id: &str,
    payload_type: &str,
) -> Option<AgentPart> {
    let tool_call_id = payload
        .get("call_id")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_str)?;
    let tool_name = payload
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("tool");
    let mut part = AgentPart::tool_call_ref(format!("{message_id}.call"), tool_call_id)
        .with_name(tool_name)
        .from_provider("codex")
        .with_metadata("codex.content_type", payload_type);
    part.json = Some(payload.to_string());
    if let Some(status) = payload.get("status").and_then(Value::as_str) {
        part = part.with_metadata("codex.status", status);
    }
    Some(part)
}

fn codex_tool_result_part(
    payload: &Value,
    message_id: &str,
    payload_type: &str,
) -> Option<AgentPart> {
    let tool_call_id = payload.get("call_id").and_then(Value::as_str)?;
    Some(
        AgentPart::json(format!("{message_id}.result"), payload.to_string())
            .with_name("tool")
            .from_provider("codex")
            .with_metadata("codex.content_type", payload_type)
            .with_metadata("codex.tool_call_id", tool_call_id)
            .with_metadata("codex.status", "completed"),
    )
}

fn codex_plan_update_part(payload: &Value, session_id: &str, message_id: &str) -> AgentPart {
    let provider_item = serde_json::json!({
        "method": "turn/plan/updated",
        "params": {
            "threadId": session_id,
            "turnId": payload.get("turn_id"),
            "explanation": payload.get("explanation"),
            "plan": payload.get("plan"),
        },
    });
    let mut part = AgentPart::tool_call_ref(format!("{message_id}.plan"), message_id)
        .with_name("update_plan")
        .from_provider("codex")
        .with_metadata("codex.content_type", "turn_plan_updated")
        .with_metadata("codex.status", "completed")
        .with_metadata("codex.has_result", "true");
    part.json = Some(provider_item.to_string());
    part
}

fn codex_duration_ms(value: Option<&Value>) -> Option<u64> {
    let duration = value?;
    if let Some(milliseconds) = duration.as_u64() {
        return Some(milliseconds);
    }
    let seconds = duration.get("secs").and_then(Value::as_u64)?;
    let nanos = duration
        .get("nanos")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    seconds
        .checked_mul(1_000)
        .and_then(|milliseconds| milliseconds.checked_add(nanos / 1_000_000))
}

fn codex_command_text(payload: &Value) -> Option<String> {
    let command = payload.get("command")?;
    if let Some(command) = command
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(command.to_string());
    }
    let command = command
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    (!command.is_empty()).then_some(command)
}

fn codex_completed_tool_event_part(
    payload: &Value,
    message_id: &str,
    item_type: &str,
    fallback_tool_name: &str,
) -> Option<AgentPart> {
    let tool_call_id = payload
        .get("call_id")
        .or_else(|| payload.get("event_id"))
        .and_then(Value::as_str)?;
    let invocation = payload.get("invocation");
    let tool_name = invocation
        .and_then(|value| value.get("tool"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("tool").and_then(Value::as_str))
        .or_else(|| payload.get("name").and_then(Value::as_str))
        .unwrap_or(fallback_tool_name);
    let status = if payload.get("success").and_then(Value::as_bool) == Some(false)
        || payload.get("error").is_some_and(|value| !value.is_null())
        || payload.pointer("/result/Err").is_some()
        || payload
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|value| matches!(value, "failed" | "declined" | "cancelled"))
        || payload
            .get("exit_code")
            .and_then(Value::as_i64)
            .is_some_and(|value| value != 0)
    {
        "failed"
    } else {
        "completed"
    };
    let provider_item = serde_json::json!({
        "id": tool_call_id,
        "type": item_type,
        "server": invocation.and_then(|value| value.get("server")),
        "tool": tool_name,
        "arguments": invocation.and_then(|value| value.get("arguments")),
        "command": codex_command_text(payload),
        "cwd": payload.get("cwd"),
        "query": payload.get("query"),
        "action": payload.get("action"),
        "result": payload.pointer("/result/Ok")
            .or_else(|| payload.get("result"))
            .or_else(|| payload.get("stdout")),
        "output": payload.get("aggregated_output").or_else(|| payload.get("stdout")),
        "error": payload.get("error").or_else(|| payload.pointer("/result/Err")),
        "status": status,
        "durationMs": codex_duration_ms(payload.get("duration")),
        "exitCode": payload.get("exit_code"),
        "changes": payload.get("changes"),
        "contentItems": payload.get("content_items"),
        "results": payload.get("results"),
        "path": payload.get("path"),
        "revisedPrompt": payload.get("revised_prompt"),
        "kind": payload.get("kind"),
        "agentThreadId": payload.get("agent_thread_id"),
        "agentPath": payload.get("agent_path"),
        "turnId": payload.get("turn_id"),
        "providerEvent": payload,
    });
    let mut part = AgentPart::tool_call_ref(format!("{message_id}.tool"), tool_call_id)
        .with_name(tool_name)
        .from_provider("codex")
        .with_metadata("codex.content_type", item_type)
        .with_metadata("codex.status", status)
        .with_metadata("codex.has_result", "true");
    part.json = Some(provider_item.to_string());
    Some(part)
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
             preview, tokens_used, name, first_user_message, rollout_path, source, thread_source \
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
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<String>>(18)?,
                row.get::<_, Option<String>>(19)?,
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
            rollout_path,
            source,
            thread_source,
        ) = row.map_err(codex_inventory_error)?;
        let lineage = codex_session_lineage(
            source.as_deref(),
            thread_source.as_deref(),
            rollout_path.as_deref(),
        );
        let mut session = adapter.to_agent_session(&CodexSessionMeta {
            id,
            forked_from_id: lineage.forked_from_id,
            parent_thread_id: lineage.parent_thread_id,
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

#[derive(Debug, Default, PartialEq, Eq)]
struct CodexSessionLineage {
    forked_from_id: Option<String>,
    parent_thread_id: Option<String>,
}

fn codex_session_lineage(
    source: Option<&str>,
    thread_source: Option<&str>,
    rollout_path: Option<&str>,
) -> CodexSessionLineage {
    let mut lineage = source
        .and_then(|source| serde_json::from_str::<Value>(source).ok())
        .map(|source| codex_source_lineage(&source))
        .unwrap_or_default();
    let is_subagent = thread_source
        .map(str::trim)
        .is_some_and(|source| source.eq_ignore_ascii_case("subagent"));

    let needs_rollout_lineage = if is_subagent {
        lineage.parent_thread_id.is_none()
    } else {
        lineage.parent_thread_id.is_some() && lineage.forked_from_id.is_none()
    };
    if needs_rollout_lineage {
        if let Some(rollout_lineage) = rollout_path
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .and_then(|path| read_codex_rollout_lineage(Path::new(path)))
        {
            lineage.parent_thread_id = lineage
                .parent_thread_id
                .or(rollout_lineage.parent_thread_id);
            lineage.forked_from_id = lineage.forked_from_id.or(rollout_lineage.forked_from_id);
        }
    }

    if is_subagent {
        lineage.parent_thread_id = lineage
            .parent_thread_id
            .clone()
            .or_else(|| lineage.forked_from_id.clone());
        lineage.forked_from_id = lineage
            .forked_from_id
            .clone()
            .or_else(|| lineage.parent_thread_id.clone());
    }
    lineage
}

fn read_codex_rollout_lineage(path: &Path) -> Option<CodexSessionLineage> {
    let file = File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(20) {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let payload = value.get("payload")?;
        let source_lineage = payload
            .get("source")
            .map(codex_source_lineage)
            .unwrap_or_default();
        return Some(CodexSessionLineage {
            forked_from_id: json_non_empty_string(payload.get("forked_from_id"))
                .or(source_lineage.forked_from_id),
            parent_thread_id: json_non_empty_string(payload.get("parent_thread_id"))
                .or(source_lineage.parent_thread_id),
        });
    }
    None
}

fn codex_source_lineage(source: &Value) -> CodexSessionLineage {
    CodexSessionLineage {
        forked_from_id: json_pointer_string(
            source,
            &[
                "/subagent/thread_spawn/forked_from_id",
                "/subagent/forked_from_id",
                "/forked_from_id",
            ],
        ),
        parent_thread_id: json_pointer_string(
            source,
            &[
                "/subagent/thread_spawn/parent_thread_id",
                "/subagent/parent_thread_id",
                "/parent_thread_id",
            ],
        ),
    }
}

fn json_pointer_string(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| json_non_empty_string(value.pointer(pointer)))
}

fn json_non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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
                 name TEXT, first_user_message TEXT NOT NULL, rollout_path TEXT, source TEXT, \
                 thread_source TEXT);",
            )
            .expect("fixture schema");
        let transaction = connection.transaction().expect("fixture transaction");
        for index in 0..225 {
            transaction
                .execute(
                    "INSERT INTO threads VALUES (?1, 0, ?2, 'provider.model.codex', \
                     '\\\\?\\E:\\Work\\BirdCoder', ?3, 0, NULL, 'gpt-5', NULL, NULL, \
                     NULL, 'on-request', ?4, 0, ?5, ?6, NULL, NULL, 'user')",
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
    fn reads_subagent_lineage_from_source_and_rollout_metadata() {
        let root = std::env::temp_dir().join(format!(
            "sdkwork-codex-provider-lineage-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("fixture directory");
        let rollout_path = root.join("subagent.jsonl");
        std::fs::write(
            &rollout_path,
            concat!(
                "{\"timestamp\":\"2026-07-29T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{",
                "\"id\":\"child-session\",\"forked_from_id\":\"parent-session\",",
                "\"parent_thread_id\":\"rollout-parent\",\"thread_source\":\"subagent\"}}\n"
            ),
        )
        .expect("fixture rollout");
        let state_path = root.join("state.sqlite");
        let connection = Connection::open(&state_path).expect("fixture database");
        connection
            .execute_batch(
                "CREATE TABLE threads (id TEXT PRIMARY KEY, created_at_ms INTEGER, \
                 updated_at_ms INTEGER, model_provider TEXT NOT NULL, cwd TEXT NOT NULL, \
                 title TEXT NOT NULL, archived INTEGER NOT NULL, archived_at INTEGER, \
                 model TEXT, reasoning_effort TEXT, agent_nickname TEXT, agent_role TEXT, \
                 approval_mode TEXT NOT NULL, preview TEXT NOT NULL, tokens_used INTEGER NOT NULL, \
                 name TEXT, first_user_message TEXT NOT NULL, rollout_path TEXT, source TEXT, \
                 thread_source TEXT);",
            )
            .expect("fixture schema");
        connection
            .execute(
                "INSERT INTO threads VALUES ('child-session', 0, 1, 'provider.model.codex', \
                 'E:\\Work\\BirdCoder', 'Child session', 0, NULL, 'gpt-5', NULL, 'Poincare', \
                 'worker', 'on-request', 'Child preview', 0, NULL, 'Child prompt', ?1, ?2, \
                 'subagent')",
                rusqlite::params![
                    rollout_path.to_string_lossy().as_ref(),
                    r#"{"subagent":{"thread_spawn":{"parent_thread_id":"parent-session","depth":1}}}"#,
                ],
            )
            .expect("fixture row");
        drop(connection);

        let sessions = read_codex_provider_sessions(&state_path).expect("provider sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].kind,
            sdkwork_agent_kernel::SessionKind::Subagent
        );
        assert_eq!(
            sessions[0].parent_session_id.as_deref(),
            Some("parent-session")
        );
        assert_eq!(
            sessions[0].forked_from_id.as_deref(),
            Some("parent-session")
        );
        std::fs::remove_dir_all(root).expect("remove fixture directory");
    }

    #[test]
    fn missing_historical_rollout_keeps_source_parent_lineage() {
        let lineage = codex_session_lineage(
            Some(
                r#"{"subagent":{"thread_spawn":{"parent_thread_id":"parent-session","depth":1}}}"#,
            ),
            Some("subagent"),
            Some("Z:\\missing\\rollout.jsonl"),
        );

        assert_eq!(lineage.parent_thread_id.as_deref(), Some("parent-session"));
        assert_eq!(lineage.forked_from_id.as_deref(), Some("parent-session"));
    }

    #[test]
    fn falls_back_to_rollout_metadata_when_source_lineage_is_missing() {
        let rollout_path = std::env::temp_dir().join(format!(
            "sdkwork-codex-provider-lineage-fallback-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &rollout_path,
            concat!(
                "{\"timestamp\":\"2026-07-29T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{",
                "\"id\":\"child-session\",\"forked_from_id\":\"parent-session\",",
                "\"parent_thread_id\":\"parent-session\",\"thread_source\":\"subagent\"}}\n"
            ),
        )
        .expect("fallback rollout");

        let lineage =
            codex_session_lineage(Some("\"cli\""), Some("subagent"), rollout_path.to_str());

        assert_eq!(lineage.parent_thread_id.as_deref(), Some("parent-session"));
        assert_eq!(lineage.forked_from_id.as_deref(), Some("parent-session"));
        std::fs::remove_file(rollout_path).expect("remove fallback rollout");
    }

    #[test]
    fn reads_text_reasoning_tool_and_mcp_items_from_rollout() {
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
                "{\"timestamp\":\"2026-07-01T00:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"reasoning\",\"id\":\"reasoning-1\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"inspect the workspace\"}]}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:02Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"id\":\"call-item-1\",\"call_id\":\"call-1\",\"name\":\"shell_command\",\"arguments\":\"{\\\"command\\\":\\\"cargo test\\\"}\"}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:03Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"id\":\"output-item-1\",\"call_id\":\"call-1\",\"output\":\"ok\"}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:04Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"mcp_tool_call_end\",\"call_id\":\"mcp-1\",\"invocation\":{\"server\":\"docs\",\"tool\":\"search\",\"arguments\":{\"q\":\"session items\"}},\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"found\"}]}}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:05Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"patch_apply_end\",\"call_id\":\"patch-1\",\"turn_id\":\"turn-1\",\"changes\":[{\"path\":\"src/main.rs\",\"kind\":\"update\"}],\"stdout\":\"Done\",\"success\":true}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:06Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"plan_update\",\"explanation\":\"Align the transcript\",\"plan\":[{\"step\":\"Inspect protocol\",\"status\":\"completed\"},{\"step\":\"Repair projection\",\"status\":\"in_progress\"}]}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:07Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"exec_command_end\",\"call_id\":\"exec-1\",\"turn_id\":\"turn-1\",\"command\":[\"cargo\",\"test\"],\"cwd\":\"/workspace\",\"stdout\":\"ok\",\"stderr\":\"\",\"aggregated_output\":\"ok\",\"exit_code\":0,\"duration\":{\"secs\":1,\"nanos\":250000000},\"status\":\"completed\"}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:08Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"dynamic_tool_call_response\",\"call_id\":\"dynamic-1\",\"turn_id\":\"turn-1\",\"tool\":\"lookup\",\"arguments\":{\"id\":\"123\"},\"content_items\":[{\"type\":\"input_text\",\"text\":\"found\"}],\"success\":true,\"duration\":{\"secs\":0,\"nanos\":42000000}}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:09Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"web_search_end\",\"call_id\":\"search-1\",\"query\":\"Codex protocol\",\"action\":{\"type\":\"search\",\"query\":\"Codex protocol\"},\"results\":[{\"url\":\"https://developers.openai.com/codex/app-server\"}]}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:10Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"view_image_tool_call\",\"call_id\":\"image-1\",\"path\":\"/workspace/screenshot.png\"}}\n",
                "{\"timestamp\":\"2026-07-01T00:00:11Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"message-assistant\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"world\"}]}}\n"
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
        assert_eq!(messages.len(), 12, "{messages:#?}");
        assert_eq!(messages[0].role, AgentMessageRole::User);
        assert_eq!(messages[0].parts[0].text.as_deref(), Some("hello"));
        assert_eq!(
            messages[1].parts[0].text.as_deref(),
            Some("inspect the workspace")
        );
        assert_eq!(
            messages[1].parts[0].metadata_value("codex.content_type"),
            Some("reasoning")
        );
        assert_eq!(messages[2].parts[0].tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(messages[2].parts[0].name.as_deref(), Some("shell_command"));
        assert_eq!(messages[3].role, AgentMessageRole::Tool);
        assert_eq!(
            messages[3].parts[0].metadata_value("codex.tool_call_id"),
            Some("call-1")
        );
        assert_eq!(messages[4].parts[0].name.as_deref(), Some("search"));
        assert_eq!(
            messages[4].parts[0].metadata_value("codex.has_result"),
            Some("true")
        );
        assert_eq!(messages[5].parts[0].name.as_deref(), Some("apply_patch"));
        let patch_payload = serde_json::from_str::<Value>(
            messages[5].parts[0].json.as_deref().expect("patch payload"),
        )
        .expect("structured patch payload");
        assert_eq!(
            patch_payload
                .pointer("/changes/0/path")
                .and_then(Value::as_str),
            Some("src/main.rs")
        );
        let plan_payload = serde_json::from_str::<Value>(
            messages[6].parts[0].json.as_deref().expect("plan payload"),
        )
        .expect("structured plan payload");
        assert_eq!(
            plan_payload.get("method").and_then(Value::as_str),
            Some("turn/plan/updated")
        );
        assert_eq!(messages[6].parts[0].name.as_deref(), Some("update_plan"));
        let command_payload = serde_json::from_str::<Value>(
            messages[7].parts[0]
                .json
                .as_deref()
                .expect("command payload"),
        )
        .expect("structured command payload");
        assert_eq!(
            command_payload.get("durationMs").and_then(Value::as_u64),
            Some(1_250)
        );
        assert_eq!(messages[8].parts[0].name.as_deref(), Some("lookup"));
        assert_eq!(messages[9].parts[0].name.as_deref(), Some("web_search"));
        let image_payload = serde_json::from_str::<Value>(
            messages[10].parts[0]
                .json
                .as_deref()
                .expect("image payload"),
        )
        .expect("structured image payload");
        assert_eq!(
            image_payload.get("path").and_then(Value::as_str),
            Some("/workspace/screenshot.png")
        );
        assert_eq!(messages[11].role, AgentMessageRole::Agent);
        assert_eq!(messages[11].parts[0].text.as_deref(), Some("world"));
        std::fs::remove_dir_all(root).expect("remove fixture");
    }
}
