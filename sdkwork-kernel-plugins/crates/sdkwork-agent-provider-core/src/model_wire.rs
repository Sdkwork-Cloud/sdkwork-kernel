//! Provider-neutral structured model input — industry wire projection layer.
//!
//! Maps kernel `ModelRequest::input_messages` to [`ModelWireMessage`] values that
//! OpenAI, Anthropic, Gemini, and subprocess transports can consume without
//! re-parsing legacy `messages` text lines.

use sdkwork_agent_kernel::{
    agent_messages_from_text_lines,
    api::{ContentBlock, ConversationRole},
    AgentMessage, AgentMessageRole, AgentPartKind, KernelError, KernelResult, ModelRequest,
};
use serde_json::{json, Value};

/// Single turn in a provider wire conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelWireMessage {
    pub role: ConversationRole,
    pub blocks: Vec<ContentBlock>,
}

impl ModelWireMessage {
    pub fn new(role: ConversationRole, blocks: Vec<ContentBlock>) -> Self {
        Self { role, blocks }
    }

    pub fn is_multimodal(&self) -> bool {
        self.blocks
            .iter()
            .any(|block| !matches!(block, ContentBlock::Text { .. } | ContentBlock::Json { .. }))
    }
}

/// Resolves structured wire messages from a model request.
///
/// Prefers `input_messages`; falls back to legacy `messages` text projection.
pub fn resolve_model_wire_messages(request: &ModelRequest) -> KernelResult<Vec<ModelWireMessage>> {
    if !request.input_messages.is_empty() {
        return agent_messages_to_wire(&request.input_messages);
    }
    if request.messages.is_empty() {
        return Ok(Vec::new());
    }
    let synthesized = agent_messages_from_text_lines(AgentMessageRole::User, &request.messages);
    agent_messages_to_wire(&synthesized)
}

/// Returns true when the request carries structured multimodal input.
pub fn model_request_has_structured_input(request: &ModelRequest) -> bool {
    if !request.input_messages.is_empty() {
        return request
            .input_messages
            .iter()
            .any(message_has_multimodal_parts);
    }
    false
}

/// Human-readable summary for legacy text-only provider adapters.
pub fn wire_messages_summary(messages: &[ModelWireMessage]) -> String {
    messages
        .iter()
        .map(|message| {
            let role = message.role.as_str();
            let body = message
                .blocks
                .iter()
                .map(content_block_summary)
                .collect::<Vec<_>>()
                .join(" ");
            format!("{role}: {body}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// OpenAI Chat Completions `messages[]` JSON projection.
pub fn wire_messages_to_openai_json(messages: &[ModelWireMessage]) -> KernelResult<Value> {
    let payload: Vec<Value> = messages
        .iter()
        .map(|message| {
            Ok(json!({
                "role": message.role.as_openai_role(),
                "content": wire_blocks_to_openai_content(&message.blocks)?,
            }))
        })
        .collect::<KernelResult<_>>()?;
    Ok(Value::Array(payload))
}

/// Anthropic Messages API `messages[]` JSON projection.
pub fn wire_messages_to_anthropic_json(messages: &[ModelWireMessage]) -> KernelResult<Value> {
    let payload: Vec<Value> = messages
        .iter()
        .filter(|message| message.role != ConversationRole::System)
        .map(|message| {
            Ok(json!({
                "role": message.role.as_anthropic_role(),
                "content": wire_blocks_to_anthropic_content(&message.blocks)?,
            }))
        })
        .collect::<KernelResult<_>>()?;
    Ok(Value::Array(payload))
}

/// Extract system blocks for Anthropic `system` field.
pub fn wire_system_text(messages: &[ModelWireMessage]) -> Option<String> {
    let mut segments = Vec::new();
    for message in messages {
        if message.role != ConversationRole::System {
            continue;
        }
        for block in &message.blocks {
            if let ContentBlock::Text { text } = block {
                segments.push(text.clone());
            }
        }
    }
    if segments.is_empty() {
        None
    } else {
        Some(segments.join("\n"))
    }
}

fn agent_messages_to_wire(messages: &[AgentMessage]) -> KernelResult<Vec<ModelWireMessage>> {
    let mut wire = Vec::with_capacity(messages.len());
    for message in messages {
        let role = ConversationRole::from_kernel_role(message.role).ok_or_else(|| {
            KernelError::validation(format!(
                "wire mapping does not support kernel role {}",
                message.role.as_str()
            ))
        })?;
        let mut blocks = Vec::with_capacity(message.parts.len());
        for part in &message.parts {
            blocks.push(ContentBlock::from_part(part)?);
        }
        wire.push(ModelWireMessage::new(role, blocks));
    }
    Ok(wire)
}

fn message_has_multimodal_parts(message: &AgentMessage) -> bool {
    message.parts.iter().any(|part| {
        matches!(
            part.kind,
            AgentPartKind::ImageRef
                | AgentPartKind::AudioRef
                | AgentPartKind::VideoRef
                | AgentPartKind::FileRef
                | AgentPartKind::BinaryRef
                | AgentPartKind::ArtifactRef
                | AgentPartKind::ToolCallRef
        )
    })
}

fn content_block_summary(block: &ContentBlock) -> String {
    match block {
        ContentBlock::Text { text } => text.clone(),
        ContentBlock::Json { value, .. } => value.clone(),
        ContentBlock::Image {
            reference,
            mime_type,
            ..
        } => {
            format!("[image:{mime_type}:{}]", reference.uri)
        }
        ContentBlock::Audio {
            reference,
            mime_type,
            ..
        } => {
            format!("[audio:{mime_type}:{}]", reference.uri)
        }
        ContentBlock::Video {
            reference,
            mime_type,
            ..
        } => {
            format!("[video:{mime_type}:{}]", reference.uri)
        }
        ContentBlock::File {
            reference,
            mime_type,
            ..
        } => {
            format!("[file:{mime_type}:{}]", reference.uri)
        }
        ContentBlock::Artifact { artifact_id } => format!("[artifact:{artifact_id}]"),
        ContentBlock::ToolCall { tool_call_id, name } => {
            if let Some(name) = name {
                format!("[tool_call:{name}:{tool_call_id}]")
            } else {
                format!("[tool_call:{tool_call_id}]")
            }
        }
        ContentBlock::ToolResult { tool_call_id, text } => {
            format!("[tool_result:{tool_call_id}] {text}")
        }
    }
}

fn wire_blocks_to_openai_content(blocks: &[ContentBlock]) -> KernelResult<Vec<Value>> {
    blocks.iter().map(block_to_openai_content).collect()
}

fn block_to_openai_content(block: &ContentBlock) -> KernelResult<Value> {
    Ok(match block {
        ContentBlock::Text { text } => json!({ "type": "text", "text": text }),
        ContentBlock::Json { value, .. } => json!({ "type": "text", "text": value }),
        ContentBlock::Image { reference, .. } => json!({
            "type": "image_url",
            "image_url": { "url": reference.uri },
        }),
        ContentBlock::Audio {
            reference,
            mime_type,
            ..
        } => json!({
            "type": "input_audio",
            "input_audio": { "data": reference.uri, "format": mime_type },
        }),
        ContentBlock::File {
            reference,
            mime_type,
            ..
        } => json!({
            "type": "file",
            "file": { "file_id": reference.uri, "mime_type": mime_type },
        }),
        ContentBlock::Video {
            reference,
            mime_type,
            ..
        } => json!({
            "type": "text",
            "text": format!("[video:{mime_type}:{}]", reference.uri),
        }),
        ContentBlock::Artifact { artifact_id } => json!({
            "type": "text",
            "text": format!("[artifact:{artifact_id}]"),
        }),
        ContentBlock::ToolCall { tool_call_id, name } => json!({
            "type": "text",
            "text": format!(
                "[tool_call:{}]",
                name.as_deref().unwrap_or(tool_call_id)
            ),
        }),
        ContentBlock::ToolResult { tool_call_id, text } => json!({
            "type": "text",
            "text": format!("[tool_result:{tool_call_id}] {text}"),
        }),
    })
}

fn wire_blocks_to_anthropic_content(blocks: &[ContentBlock]) -> KernelResult<Vec<Value>> {
    blocks.iter().map(block_to_anthropic_content).collect()
}

fn block_to_anthropic_content(block: &ContentBlock) -> KernelResult<Value> {
    Ok(match block {
        ContentBlock::Text { text } => json!({ "type": "text", "text": text }),
        ContentBlock::Json { value, .. } => json!({ "type": "text", "text": value }),
        ContentBlock::Image {
            reference,
            mime_type,
            ..
        } => json!({
            "type": "image",
            "source": {
                "type": "url",
                "url": reference.uri,
                "media_type": mime_type,
            },
        }),
        ContentBlock::File {
            reference,
            mime_type,
            ..
        } => json!({
            "type": "document",
            "source": {
                "type": "url",
                "url": reference.uri,
                "media_type": mime_type,
            },
        }),
        ContentBlock::Audio {
            reference,
            mime_type,
            ..
        }
        | ContentBlock::Video {
            reference,
            mime_type,
            ..
        } => json!({
            "type": "text",
            "text": format!("[media:{mime_type}:{}]", reference.uri),
        }),
        ContentBlock::Artifact { artifact_id } => json!({
            "type": "text",
            "text": format!("[artifact:{artifact_id}]"),
        }),
        ContentBlock::ToolCall { tool_call_id, name } => json!({
            "type": "tool_use",
            "id": tool_call_id,
            "name": name.clone().unwrap_or_else(|| "tool".to_string()),
            "input": {},
        }),
        ContentBlock::ToolResult { tool_call_id, text } => json!({
            "type": "tool_result",
            "tool_use_id": tool_call_id,
            "content": text,
        }),
    })
}

/// Builds SDK runtime `model_chat` / `model_chat_stream` operation fields from a kernel request.
pub fn build_model_chat_operation(
    request: &ModelRequest,
) -> KernelResult<(String, Vec<String>, Option<Value>)> {
    let messages = request.effective_text_lines();
    let wire_messages = if !request.input_messages.is_empty() {
        let wire = resolve_model_wire_messages(request)?;
        Some(wire_messages_to_openai_json(&wire)?)
    } else {
        None
    };
    Ok((request.model_request_id.clone(), messages, wire_messages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{AgentMessage, AgentMessageRole, AgentPart, ContentReference};

    #[test]
    fn resolves_structured_input_messages() {
        let mut request = ModelRequest::new("req.1", vec!["legacy".to_string()]);
        request.input_messages = vec![AgentMessage::new(
            "msg.1",
            AgentMessageRole::User,
            vec![
                AgentPart::text("part.text", "hello"),
                AgentPart::image_ref(
                    "part.image",
                    ContentReference::host("images/a.png").uri,
                    "image/png",
                ),
            ],
        )];

        let wire = resolve_model_wire_messages(&request).expect("wire");
        assert_eq!(wire.len(), 1);
        assert!(wire[0].is_multimodal());
        assert!(model_request_has_structured_input(&request));
    }

    #[test]
    fn build_model_chat_operation_emits_wire_for_canonical_input_messages() {
        let mut request = ModelRequest::new("req.1", vec!["legacy".to_string()]);
        request.input_messages = vec![AgentMessage::new(
            "msg.1",
            AgentMessageRole::User,
            vec![AgentPart::text("part.text", "hello")],
        )];

        let (_, messages, wire) = build_model_chat_operation(&request).expect("operation");
        assert_eq!(messages, vec!["hello".to_string()]);
        assert!(wire.is_some());
    }

    #[test]
    fn openai_json_includes_image_url_part() {
        let wire = vec![ModelWireMessage::new(
            ConversationRole::User,
            vec![
                ContentBlock::text("describe"),
                ContentBlock::image(
                    ContentReference::parse("https://example.com/a.png").expect("url"),
                    "image/png",
                ),
            ],
        )];
        let json = wire_messages_to_openai_json(&wire).expect("json");
        let content = json[0]["content"].as_array().expect("content");
        assert_eq!(content[1]["type"], "image_url");
    }
}
