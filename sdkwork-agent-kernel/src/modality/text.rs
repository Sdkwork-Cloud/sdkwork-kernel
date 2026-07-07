//! Text projection helpers for legacy provider `messages` fields.

use crate::AgentMessage;

/// Flattens structured messages into plain text lines for legacy model providers.
pub fn agent_messages_to_text_lines(messages: &[AgentMessage]) -> Vec<String> {
    messages
        .iter()
        .map(flatten_message_to_text)
        .filter(|line| !line.is_empty())
        .collect()
}

pub fn flatten_message_to_text(message: &AgentMessage) -> String {
    let mut segments = Vec::new();
    for part in &message.parts {
        if let Some(text) = &part.text {
            segments.push(text.clone());
            continue;
        }
        if let Some(json) = &part.json {
            segments.push(json.clone());
            continue;
        }
        if let Some(content_ref) = &part.content_ref {
            let mime = part
                .mime_type
                .as_deref()
                .unwrap_or("application/octet-stream");
            segments.push(format!(
                "[{kind}:{mime}:{content_ref}]",
                kind = part.kind.as_str()
            ));
            continue;
        }
        if let Some(artifact_id) = &part.artifact_id {
            segments.push(format!("[artifact:{artifact_id}]"));
        }
    }
    segments.join("\n")
}

/// Builds a single structured message from legacy plain-text lines.
pub fn agent_messages_from_text_lines(
    role: crate::AgentMessageRole,
    lines: &[String],
) -> Vec<AgentMessage> {
    let parts: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| crate::AgentPart::text(format!("part.{index}"), line.clone()))
        .collect();
    if parts.is_empty() {
        return Vec::new();
    }
    vec![AgentMessage::new("message.synthesized", role, parts)]
}
