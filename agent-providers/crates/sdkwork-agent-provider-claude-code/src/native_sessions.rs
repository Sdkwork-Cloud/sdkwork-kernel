use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelResult,
};
use sdkwork_agent_provider_core::SessionAdapter;
use serde_json::Value;

use crate::{ClaudeCodeAdapter, ClaudeCodeProcessState};

pub fn discover_claude_code_native_sessions() -> KernelResult<Vec<AgentSession>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let projects_path = PathBuf::from(home).join(".claude").join("projects");
    if !projects_path.is_dir() {
        return Ok(Vec::new());
    }
    read_claude_code_native_sessions(&projects_path)
}

pub fn discover_claude_code_native_session_messages(
    session_id: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Ok(Vec::new());
    };
    let projects_path = PathBuf::from(home).join(".claude").join("projects");
    if !projects_path.is_dir() {
        return Ok(Vec::new());
    }
    read_claude_code_native_session_messages(&projects_path, session_id)
}

pub fn read_claude_code_native_session_messages(
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
    let file = File::open(path).map_err(claude_inventory_error)?;
    let mut messages = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(claude_inventory_error)?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("sessionId").and_then(Value::as_str) != Some(session_id)
            || value
                .get("isSidechain")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            continue;
        }
        let role = match value.get("type").and_then(Value::as_str) {
            Some("user") => AgentMessageRole::User,
            Some("assistant") => AgentMessageRole::Agent,
            _ => continue,
        };
        let Some(message_id) = value
            .get("uuid")
            .and_then(Value::as_str)
            .or_else(|| value.pointer("/message/id").and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(content_value) = value.pointer("/message/content") else {
            continue;
        };
        let content = match content_value {
            Value::String(content) => content.trim().to_string(),
            Value::Array(parts) => parts
                .iter()
                .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n"),
            _ => String::new(),
        };
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

pub fn read_claude_code_native_sessions(projects_path: &Path) -> KernelResult<Vec<AgentSession>> {
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
        title: Some(format!("Claude Code {session_id}")),
        created_at,
        updated_at,
    }))
}

fn claude_inventory_error(error: std::io::Error) -> KernelError {
    KernelError::provider_error("claude_code_native_inventory", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_jsonl_session_identity_and_cwd() {
        let root = std::env::temp_dir().join(format!(
            "sdkwork-claude-native-sessions-{}",
            std::process::id()
        ));
        let project = root.join("E--Work-BirdCoder");
        std::fs::create_dir_all(&project).expect("fixture directory");
        let path = project.join("session-1.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"user\",\"uuid\":\"message-user\",\"sessionId\":\"session-1\",\"cwd\":\"E:\\\\Work\\\\BirdCoder\",\"timestamp\":\"2026-07-01T00:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
                "{\"type\":\"assistant\",\"uuid\":\"message-assistant\",\"sessionId\":\"session-1\",\"cwd\":\"E:\\\\Work\\\\BirdCoder\",\"timestamp\":\"2026-07-01T00:01:00Z\",\"message\":{\"role\":\"assistant\",\"model\":\"claude-sonnet\",\"content\":[{\"type\":\"text\",\"text\":\"world\"}]}}\n"
            ),
        )
        .expect("fixture jsonl");

        let sessions = read_claude_code_native_sessions(&root).expect("native sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "session-1");
        assert_eq!(sessions[0].model.as_deref(), Some("claude-sonnet"));
        let messages = read_claude_code_native_session_messages(&root, "session-1")
            .expect("native transcript");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, AgentMessageRole::User);
        assert_eq!(messages[1].role, AgentMessageRole::Agent);
        std::fs::remove_dir_all(root).expect("remove fixture");
    }
}
