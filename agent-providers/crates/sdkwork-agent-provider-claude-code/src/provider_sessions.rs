use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
};
use sdkwork_agent_provider_core::{
    ProviderSessionHistoryBudget, ProviderSessionHistoryLimits, SessionAdapter,
};
use serde_json::Value;

use crate::{ClaudeCodeAdapter, ClaudeCodeProcessState};

pub fn discover_claude_code_provider_sessions() -> KernelResult<Vec<AgentSession>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let projects_path = PathBuf::from(home).join(".claude").join("projects");
    if !projects_path.is_dir() {
        return Ok(Vec::new());
    }
    read_claude_code_provider_sessions(&projects_path)
}

pub fn discover_claude_code_provider_session_messages(
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let projects_path = PathBuf::from(home).join(".claude").join("projects");
    if !projects_path.is_dir() {
        return Ok(Vec::new());
    }
    read_claude_code_provider_session_messages(&projects_path, session_id)
}

pub fn read_claude_code_provider_session_messages(
    projects_path: &Path,
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    for project_entry in fs::read_dir(projects_path).map_err(claude_inventory_error)? {
        let project_path = project_entry.map_err(claude_inventory_error)?.path();
        if !project_path.is_dir() {
            continue;
        }
        let direct_path = project_path.join(format!("{session_id}.jsonl"));
        if direct_path.is_file() {
            return read_claude_session_messages_file(&direct_path, session_id);
        }
        for session_entry in fs::read_dir(project_path).map_err(claude_inventory_error)? {
            let session_path = session_entry.map_err(claude_inventory_error)?.path();
            if session_path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                continue;
            }
            let messages = read_claude_session_messages_file(&session_path, session_id)?;
            if !messages.is_empty() {
                return Ok(messages);
            }
        }
    }
    Ok(Vec::new())
}

fn read_claude_session_messages_file(
    path: &Path,
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    read_claude_session_messages_file_with_limits(
        path,
        session_id,
        ProviderSessionHistoryLimits::default(),
    )
}

fn read_claude_session_messages_file_with_limits(
    path: &Path,
    session_id: &str,
    limits: ProviderSessionHistoryLimits,
) -> KernelResult<Vec<AgentMessage>> {
    let mut budget = ProviderSessionHistoryBudget::new(limits);
    budget.validate_file_size(path)?;
    let file = File::open(path).map_err(claude_inventory_error)?;
    let mut messages = Vec::new();
    let mut message_indexes = HashMap::new();
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(claude_inventory_error)?;
        budget.record_source(line.len())?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("sessionId").and_then(Value::as_str) != Some(session_id) {
            continue;
        }
        let Some(role) = claude_message_role(&value) else {
            continue;
        };
        let Some(message_id) = claude_message_id(&value, line_index) else {
            continue;
        };
        let mut parts = claude_message_parts(&value, &message_id);
        if parts.is_empty() {
            parts = claude_top_level_parts(&value, &message_id);
        }
        if parts.is_empty() {
            continue;
        }
        let mut message =
            AgentMessage::new(message_id.clone(), role, parts).for_session(session_id);
        if value
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            message = message.with_metadata("claude.is_sidechain", "true");
        }
        if let Some(parent_uuid) = value
            .get("parentUuid")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            message = message.with_metadata("claude.parent_uuid", parent_uuid);
        }
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            message = message.created_at(timestamp);
        }
        if let Some(message_index) = message_indexes.get(&message_id).copied() {
            messages[message_index] = message;
        } else {
            budget.record_message()?;
            message_indexes.insert(message_id, messages.len());
            messages.push(message);
        }
    }
    Ok(messages)
}

fn claude_message_role(value: &Value) -> Option<AgentMessageRole> {
    let role = value
        .pointer("/message/role")
        .and_then(Value::as_str)
        .or_else(|| value.get("type").and_then(Value::as_str));
    match role {
        Some("user") => Some(AgentMessageRole::User),
        Some("assistant") => Some(AgentMessageRole::Agent),
        Some("system") => Some(AgentMessageRole::System),
        Some("tool") => Some(AgentMessageRole::Tool),
        Some("attachment") => Some(AgentMessageRole::User),
        Some("queue-operation" | "tool_use_summary") => Some(AgentMessageRole::Adapter),
        _ => None,
    }
}

fn claude_message_id(value: &Value, line_index: usize) -> Option<String> {
    if let Some(message_id) = value
        .get("uuid")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/message/id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(message_id.to_string());
    }
    let record_type = value.get("type").and_then(Value::as_str)?;
    let subtype = value
        .get("subtype")
        .and_then(Value::as_str)
        .unwrap_or("record");
    Some(format!("claude.{record_type}.{subtype}.{line_index}"))
}

fn claude_top_level_parts(value: &Value, message_id: &str) -> Vec<AgentPart> {
    let Some(record_type) = value.get("type").and_then(Value::as_str) else {
        return Vec::new();
    };
    let subtype = value.get("subtype").and_then(Value::as_str);
    let content_type = subtype.unwrap_or(record_type);
    if record_type == "assistant"
        && (value.get("error").is_some()
            || value
                .get("isApiErrorMessage")
                .and_then(Value::as_bool)
                .unwrap_or(false))
    {
        let error = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Claude Code provider error");
        return vec![AgentPart::error(
            format!("{message_id}.error"),
            "claude_provider_error",
            error,
        )
        .from_provider("claude-code")
        .with_metadata("claude.content_type", "error")];
    }
    if matches!(record_type, "attachment" | "queue-operation") {
        return vec![
            AgentPart::json(format!("{message_id}.record"), value.to_string())
                .from_provider("claude-code")
                .with_metadata("claude.content_type", content_type),
        ];
    }
    let is_tool_lifecycle = record_type == "tool_use_summary"
        || subtype.is_some_and(|value| {
            matches!(
                value,
                "hook_progress"
                    | "hook_response"
                    | "hook_started"
                    | "permission_denied"
                    | "task_notification"
                    | "task_progress"
                    | "task_started"
                    | "task_updated"
            )
        });
    if !is_tool_lifecycle {
        return (record_type == "system")
            .then(|| {
                AgentPart::json(format!("{message_id}.system"), value.to_string())
                    .from_provider("claude-code")
                    .with_metadata("claude.content_type", content_type)
            })
            .into_iter()
            .collect();
    }

    let tool_call_id = value
        .get("tool_use_id")
        .or_else(|| value.get("toolUseId"))
        .or_else(|| value.get("task_id"))
        .or_else(|| value.get("hook_id"))
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(message_id);
    let tool_name = value
        .get("tool_name")
        .or_else(|| value.get("hook_name"))
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if content_type.starts_with("hook_") {
                "hook"
            } else if content_type.starts_with("task_") {
                "task"
            } else {
                "tool"
            }
        });
    let explicit_status = value
        .get("status")
        .or_else(|| value.get("outcome"))
        .and_then(Value::as_str);
    let status = explicit_status.unwrap_or(match content_type {
        "hook_progress" | "hook_started" | "task_progress" | "task_started" => "pending",
        "permission_denied" => "cancelled",
        _ => "completed",
    });
    let has_result = !matches!(
        content_type,
        "hook_progress" | "hook_started" | "task_progress" | "task_started"
    );
    let mut part = AgentPart::tool_call_ref(format!("{message_id}.event"), tool_call_id)
        .with_name(tool_name)
        .from_provider("claude-code")
        .with_metadata("claude.content_type", content_type)
        .with_metadata("claude.status", status)
        .with_metadata(
            "claude.has_result",
            if has_result { "true" } else { "false" },
        );
    part.json = Some(value.to_string());
    vec![part]
}

fn claude_message_parts(value: &Value, message_id: &str) -> Vec<AgentPart> {
    let Some(content) = value.pointer("/message/content") else {
        return Vec::new();
    };
    match content {
        Value::String(text) => {
            let text = text.trim();
            if text.is_empty() {
                Vec::new()
            } else {
                vec![AgentPart::text(format!("{message_id}.text"), text)
                    .from_provider("claude-code")
                    .with_metadata("claude.content_type", "text")]
            }
        }
        Value::Array(parts) => parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| claude_content_part(part, message_id, index))
            .collect(),
        _ => Vec::new(),
    }
}

fn claude_content_part(part: &Value, message_id: &str, index: usize) -> Option<AgentPart> {
    let content_type = part.get("type").and_then(Value::as_str)?;
    let part_id = format!("{message_id}.content.{index}");
    match content_type {
        "text" => {
            let text = part.get("text").and_then(Value::as_str)?.trim();
            (!text.is_empty()).then(|| {
                AgentPart::text(part_id, text)
                    .from_provider("claude-code")
                    .with_metadata("claude.content_type", "text")
            })
        }
        "thinking" => {
            let text = part.get("thinking").and_then(Value::as_str)?.trim();
            (!text.is_empty()).then(|| {
                AgentPart::text(part_id, text)
                    .from_provider("claude-code")
                    .with_metadata("claude.content_type", "thinking")
            })
        }
        "tool_use" | "server_tool_use" | "mcp_tool_use" => {
            let tool_call_id = part.get("id").and_then(Value::as_str)?;
            let tool_name = part
                .get("name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("tool");
            let mut agent_part = AgentPart::tool_call_ref(part_id, tool_call_id)
                .with_name(tool_name)
                .from_provider("claude-code")
                .with_metadata("claude.content_type", content_type)
                .with_metadata("claude.status", "pending");
            agent_part.json = Some(part.to_string());
            Some(agent_part)
        }
        "advisor_tool_result"
        | "bash_code_execution_tool_result"
        | "code_execution_tool_result"
        | "mcp_tool_result"
        | "text_editor_code_execution_tool_result"
        | "tool_result"
        | "tool_search_tool_result"
        | "web_fetch_tool_result"
        | "web_search_tool_result" => {
            let tool_call_id = part
                .get("tool_use_id")
                .or_else(|| part.get("toolUseId"))
                .and_then(Value::as_str)?;
            let is_error = part
                .get("is_error")
                .or_else(|| part.get("isError"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some(
                AgentPart::json(part_id, part.to_string())
                    .with_name("tool")
                    .from_provider("claude-code")
                    .with_metadata("claude.content_type", content_type)
                    .with_metadata("claude.tool_call_id", tool_call_id)
                    .with_metadata(
                        "claude.status",
                        if is_error { "failed" } else { "completed" },
                    ),
            )
        }
        "tool_progress" => {
            let tool_call_id = part
                .get("tool_use_id")
                .or_else(|| part.get("toolUseId"))
                .or_else(|| part.get("id"))
                .and_then(Value::as_str)?;
            let tool_name = part
                .get("tool_name")
                .or_else(|| part.get("name"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("tool");
            let mut agent_part = AgentPart::tool_call_ref(part_id, tool_call_id)
                .with_name(tool_name)
                .from_provider("claude-code")
                .with_metadata("claude.content_type", content_type)
                .with_metadata("claude.status", "pending");
            agent_part.json = Some(part.to_string());
            Some(agent_part)
        }
        "image" | "document" => Some(
            AgentPart::json(part_id, part.to_string())
                .from_provider("claude-code")
                .with_metadata("claude.content_type", content_type),
        ),
        _ => Some(
            AgentPart::json(part_id, part.to_string())
                .from_provider("claude-code")
                .with_metadata("claude.content_type", content_type),
        ),
    }
}

pub fn read_claude_code_provider_sessions(projects_path: &Path) -> KernelResult<Vec<AgentSession>> {
    let adapter = ClaudeCodeAdapter::new();
    let mut sessions = Vec::new();
    for project_entry in fs::read_dir(projects_path).map_err(claude_inventory_error)? {
        let project_path = project_entry.map_err(claude_inventory_error)?.path();
        if !project_path.is_dir() {
            continue;
        }
        for session_entry in fs::read_dir(project_path).map_err(claude_inventory_error)? {
            let session_path = session_entry.map_err(claude_inventory_error)?.path();
            if session_path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                continue;
            }
            if let Some(state) = read_claude_session_file(&session_path)? {
                sessions.push(adapter.to_agent_session(&state)?);
            }
        }
    }
    Ok(sessions)
}

fn read_claude_session_file(path: &Path) -> KernelResult<Option<ClaudeCodeProcessState>> {
    let file = File::open(path).map_err(claude_inventory_error)?;
    let fallback_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let mut session_id = fallback_id;
    let mut cwd = None;
    let mut model = None;
    let mut provider_title = None;
    let mut first_user_message = None;
    let mut created_at = None;
    let mut updated_at = None;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(claude_inventory_error)?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(value) = value.get("sessionId").and_then(Value::as_str) {
            session_id = value.to_string();
        }
        let is_sidechain = value
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if provider_title.is_none() && !is_sidechain {
            provider_title = value
                .get("slug")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
        }
        if first_user_message.is_none()
            && !is_sidechain
            && value.get("type").and_then(Value::as_str) == Some("user")
        {
            first_user_message = claude_message_text(&value);
        }
        if cwd.is_none() {
            cwd = value.get("cwd").and_then(Value::as_str).map(str::to_string);
        }
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            created_at.get_or_insert_with(|| timestamp.to_string());
            updated_at = Some(timestamp.to_string());
        }
        if model.is_none() {
            model = value
                .get("message")
                .and_then(|message| message.get("model"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
    }
    if session_id.trim().is_empty() || cwd.is_none() {
        return Ok(None);
    }
    Ok(Some(ClaudeCodeProcessState {
        session_id: session_id.clone(),
        agent_type: "main".to_string(),
        model,
        cwd,
        title: provider_title.or(first_user_message),
        created_at,
        updated_at,
    }))
}

fn claude_message_text(value: &Value) -> Option<String> {
    let content_value = value.pointer("/message/content")?;
    let content = match content_value {
        Value::String(content) => content.trim().to_string(),
        Value::Array(parts) => parts
            .iter()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    };
    (!content.is_empty()).then_some(content)
}

fn claude_inventory_error(error: std::io::Error) -> KernelError {
    KernelError::provider_error("claude_code_provider_session_inventory", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_jsonl_session_identity_and_cwd() {
        let root = std::env::temp_dir().join(format!(
            "sdkwork-claude-provider-sessions-{}",
            std::process::id()
        ));
        let project = root.join("E--Work-BirdCoder");
        std::fs::create_dir_all(&project).expect("fixture directory");
        let path = project.join("session-1.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"uuid\":\"message-user\",\"sessionId\":\"session-1\",\"cwd\":\"E:\\\\Work\\\\BirdCoder\",\"timestamp\":\"2026-07-01T00:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
                "{\"type\":\"assistant\",\"uuid\":\"message-assistant\",\"sessionId\":\"session-1\",\"slug\":\"provider-title\",\"cwd\":\"E:\\\\Work\\\\BirdCoder\",\"timestamp\":\"2026-07-01T00:01:00Z\",\"message\":{\"role\":\"assistant\",\"model\":\"claude-sonnet\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"inspect the source\",\"signature\":\"opaque\"},{\"type\":\"tool_use\",\"id\":\"tool-1\",\"name\":\"mcp__docs__search\",\"input\":{\"q\":\"session items\"}},{\"type\":\"text\",\"text\":\"world\"}]}}\n",
                "{\"type\":\"user\",\"uuid\":\"message-tool-result\",\"sessionId\":\"session-1\",\"cwd\":\"E:\\\\Work\\\\BirdCoder\",\"timestamp\":\"2026-07-01T00:01:01Z\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"tool-1\",\"content\":[{\"type\":\"text\",\"text\":\"found\"}]}]}}\n",
                "{\"type\":\"system\",\"subtype\":\"hook_response\",\"uuid\":\"message-hook\",\"sessionId\":\"session-1\",\"timestamp\":\"2026-07-01T00:01:02Z\",\"tool_use_id\":\"tool-1\",\"hook_name\":\"post-tool\",\"outcome\":\"success\",\"output\":\"checked\"}\n",
                "{\"type\":\"queue-operation\",\"sessionId\":\"session-1\",\"timestamp\":\"2026-07-01T00:01:03Z\",\"operation\":\"enqueue\",\"content\":\"next prompt\"}\n"
            ),
        )
        .expect("fixture jsonl");
        std::fs::write(
            project.join("session-2.jsonl"),
            "{\"type\":\"user\",\"uuid\":\"message-user-2\",\"sessionId\":\"session-2\",\"cwd\":\"E:\\\\Work\\\\BirdCoder\",\"timestamp\":\"2026-07-01T00:02:00Z\",\"message\":{\"role\":\"user\",\"content\":\"fallback user prompt\"}}\n",
        )
        .expect("fallback fixture jsonl");

        let sessions = read_claude_code_provider_sessions(&root).expect("provider sessions");
        assert_eq!(sessions.len(), 2);
        let titled = sessions
            .iter()
            .find(|session| session.session_id == "session-1")
            .expect("provider-titled session");
        assert_eq!(titled.title.as_deref(), Some("provider-title"));
        assert_eq!(titled.model.as_deref(), Some("claude-sonnet"));
        let fallback = sessions
            .iter()
            .find(|session| session.session_id == "session-2")
            .expect("fallback-titled session");
        assert_eq!(fallback.title.as_deref(), Some("fallback user prompt"));
        assert!(sessions.iter().all(|session| {
            session.activity.freshness
                == sdkwork_agent_kernel::SessionActivityFreshness::Unsupported
        }));
        let messages = read_claude_code_provider_session_messages(&root, "session-1")
            .expect("provider transcript");
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].role, AgentMessageRole::User);
        assert_eq!(messages[1].role, AgentMessageRole::Agent);
        assert_eq!(messages[1].parts.len(), 3);
        assert_eq!(
            messages[1].parts[0].metadata_value("claude.content_type"),
            Some("thinking")
        );
        assert_eq!(messages[1].parts[1].tool_call_id.as_deref(), Some("tool-1"));
        assert_eq!(
            messages[1].parts[1].name.as_deref(),
            Some("mcp__docs__search")
        );
        assert_eq!(messages[2].role, AgentMessageRole::User);
        assert_eq!(
            messages[2].parts[0].metadata_value("claude.tool_call_id"),
            Some("tool-1")
        );
        assert_eq!(messages[3].role, AgentMessageRole::System);
        assert_eq!(messages[3].parts[0].tool_call_id.as_deref(), Some("tool-1"));
        assert_eq!(
            messages[3].parts[0].metadata_value("claude.content_type"),
            Some("hook_response")
        );
        assert_eq!(
            messages[3].parts[0].metadata_value("claude.has_result"),
            Some("true")
        );
        assert_eq!(messages[4].role, AgentMessageRole::Adapter);
        assert_eq!(
            messages[4].parts[0].metadata_value("claude.content_type"),
            Some("queue-operation")
        );
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn rejects_history_files_beyond_the_serialized_byte_budget_before_parsing() {
        let path = std::env::temp_dir().join(format!(
            "sdkwork-claude-provider-session-budget-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "{\"type\":\"user\",\"uuid\":\"message-user\",\"sessionId\":\"session-budget\"}\n",
        )
        .expect("history budget fixture");

        let error = read_claude_session_messages_file_with_limits(
            &path,
            "session-budget",
            ProviderSessionHistoryLimits {
                max_source_records: 10,
                max_messages: 10,
                max_serialized_bytes: 8,
            },
        )
        .expect_err("oversized history file must fail");
        assert!(error.to_string().contains("exceeds 8 serialized bytes"));
        std::fs::remove_file(path).expect("remove history budget fixture");
    }

    #[test]
    fn latest_duplicate_message_replaces_partial_content_without_reordering() {
        let path = std::env::temp_dir().join(format!(
            "sdkwork-claude-provider-session-latest-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"assistant\",\"uuid\":\"message-assistant\",\"sessionId\":\"session-latest\",\"timestamp\":\"2026-07-01T00:00:00Z\",\"message\":{\"role\":\"assistant\",\"content\":\"partial\"}}\n",
                "{\"type\":\"user\",\"uuid\":\"message-user\",\"sessionId\":\"session-latest\",\"timestamp\":\"2026-07-01T00:00:01Z\",\"message\":{\"role\":\"user\",\"content\":\"next\"}}\n",
                "{\"type\":\"assistant\",\"uuid\":\"message-assistant\",\"sessionId\":\"session-latest\",\"timestamp\":\"2026-07-01T00:00:02Z\",\"message\":{\"role\":\"assistant\",\"content\":\"final answer\"}}\n"
            ),
        )
        .expect("latest message fixture");

        let messages = read_claude_session_messages_file(&path, "session-latest")
            .expect("provider transcript");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_id, "message-assistant");
        assert_eq!(messages[0].parts[0].text.as_deref(), Some("final answer"));
        assert_eq!(
            messages[0].created_at.as_deref(),
            Some("2026-07-01T00:00:02Z")
        );
        assert_eq!(messages[1].message_id, "message-user");
        std::fs::remove_file(path).expect("remove latest message fixture");
    }
}
