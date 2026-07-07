//! Protocol RPC ingress — maps structured chat payloads into kernel `AgentMessage` parts.

use super::content_ref::ContentReference;
use super::kind::AgentInputModality;
use crate::{AgentMessage, AgentMessageRole, AgentPart, AgentPartKind, KernelError, KernelResult};

const CHAT_PAYLOAD_SCHEMA: &str = "sdkwork.agent.rpc.chat.input.v1";

/// Parses agent chat RPC payload into structured messages.
///
/// Plain text payloads become a single text part. JSON objects with a `parts`
/// array use `sdkwork.agent.rpc.chat.input.v1` shape.
pub fn parse_chat_rpc_payload(
    protocol_request_id: &str,
    payload: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Err(KernelError::validation(
            "agent chat RPC payload must not be empty",
        ));
    }

    if trimmed.starts_with('{') {
        return parse_structured_chat_payload(protocol_request_id, trimmed);
    }

    Ok(vec![AgentMessage::new(
        format!("message.{protocol_request_id}"),
        AgentMessageRole::User,
        vec![AgentPart::text("part.0", trimmed)],
    )])
}

fn parse_structured_chat_payload(
    protocol_request_id: &str,
    payload: &str,
) -> KernelResult<Vec<AgentMessage>> {
    let root: serde_json::Value = serde_json::from_str(payload).map_err(|error| {
        KernelError::validation(format!(
            "chat RPC structured payload is not valid JSON: {error}"
        ))
    })?;

    if let Some(messages) = root.get("messages").and_then(|value| value.as_array()) {
        let mut parsed = Vec::with_capacity(messages.len());
        for (index, message) in messages.iter().enumerate() {
            parsed.push(parse_message_object(
                message,
                &format!("message.{protocol_request_id}.{index}"),
            )?);
        }
        if parsed.is_empty() {
            return Err(KernelError::validation(
                "chat RPC structured payload messages must not be empty",
            ));
        }
        return Ok(parsed);
    }

    Ok(vec![parse_message_object(
        &root,
        &format!("message.{protocol_request_id}"),
    )?])
}

fn parse_message_object(
    message: &serde_json::Value,
    default_message_id: &str,
) -> KernelResult<AgentMessage> {
    let message_id = message
        .get("message_id")
        .and_then(|value| value.as_str())
        .unwrap_or(default_message_id);
    let role = parse_message_role(
        message
            .get("role")
            .and_then(|value| value.as_str())
            .unwrap_or("user"),
    )?;
    let parts_value = message
        .get("parts")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            KernelError::validation(format!(
                "structured chat payload must include parts array ({CHAT_PAYLOAD_SCHEMA})"
            ))
        })?;
    if parts_value.is_empty() {
        return Err(KernelError::validation(
            "structured chat payload parts must not be empty",
        ));
    }

    let mut parts = Vec::with_capacity(parts_value.len());
    for (index, part) in parts_value.iter().enumerate() {
        parts.push(parse_part_object(part, index)?);
    }

    let built = AgentMessage::new(message_id, role, parts);
    built.validate()?;
    Ok(built)
}

fn parse_message_role(role: &str) -> KernelResult<AgentMessageRole> {
    match role {
        "user" => Ok(AgentMessageRole::User),
        "agent" => Ok(AgentMessageRole::Agent),
        "system" => Ok(AgentMessageRole::System),
        "model" => Ok(AgentMessageRole::Model),
        "tool" => Ok(AgentMessageRole::Tool),
        "policy" => Ok(AgentMessageRole::Policy),
        "adapter" => Ok(AgentMessageRole::Adapter),
        _ => Err(KernelError::validation(format!(
            "unknown agent message role in chat RPC payload: {role}"
        ))),
    }
}

fn parse_part_object(part: &serde_json::Value, index: usize) -> KernelResult<AgentPart> {
    let part_id = part
        .get("part_id")
        .and_then(|value| value.as_str())
        .unwrap_or(&format!("part.{index}"))
        .to_string();
    let kind = part
        .get("kind")
        .and_then(|value| value.as_str())
        .ok_or_else(|| KernelError::validation("chat RPC part kind is required"))?;
    let kind = parse_part_kind(kind)?;

    let mut built = match kind {
        AgentPartKind::Text => {
            let text = required_string_field(part, "text", "text part")?;
            AgentPart::text(part_id, text)
        }
        AgentPartKind::Json => {
            let json = required_string_field(part, "json", "json part")?;
            AgentPart::json(part_id, json)
        }
        AgentPartKind::BinaryRef | AgentPartKind::FileRef => {
            let content_ref = required_content_ref(part)?;
            let mime_type =
                required_string_field(part, "mime_type", &format!("{} part", kind.as_str()))?;
            if kind == AgentPartKind::BinaryRef {
                AgentPart::binary_ref(part_id, content_ref, mime_type)
            } else {
                AgentPart::file_ref(part_id, content_ref, mime_type)
            }
        }
        AgentPartKind::ArtifactRef => {
            let artifact_id = required_string_field(part, "artifact_id", "artifact_ref part")?;
            AgentPart::artifact_ref(part_id, artifact_id)
        }
        AgentPartKind::ImageRef | AgentPartKind::AudioRef | AgentPartKind::VideoRef => {
            let content_ref = required_content_ref(part)?;
            let mime_type =
                required_string_field(part, "mime_type", &format!("{} part", kind.as_str()))?;
            match kind {
                AgentPartKind::ImageRef => AgentPart::image_ref(part_id, content_ref, mime_type),
                AgentPartKind::AudioRef => AgentPart::audio_ref(part_id, content_ref, mime_type),
                AgentPartKind::VideoRef => AgentPart::video_ref(part_id, content_ref, mime_type),
                _ => unreachable!(),
            }
        }
        AgentPartKind::ToolCallRef => {
            let tool_call_id = required_string_field(part, "tool_call_id", "tool_call_ref part")?;
            AgentPart::tool_call_ref(part_id, tool_call_id)
        }
        AgentPartKind::PolicyDecisionRef => {
            let policy_decision_id =
                required_string_field(part, "policy_decision_id", "policy_decision_ref part")?;
            AgentPart::policy_decision_ref(part_id, policy_decision_id)
        }
        AgentPartKind::Error => {
            let error_code = required_string_field(part, "error_code", "error part")?;
            let text = required_string_field(part, "text", "error part")?;
            AgentPart::error(part_id, error_code, text)
        }
    };

    if let Some(name) = part.get("name").and_then(|value| value.as_str()) {
        built = built.with_name(name);
    }
    if let Some(schema) = part.get("schema").and_then(|value| value.as_str()) {
        built = built.with_schema(schema);
    }
    if let Some(provenance) = part.get("provenance").and_then(|value| value.as_str()) {
        built = built.from_provider(provenance);
    }
    if let Some(byte_length) = part.get("byte_length").and_then(|value| value.as_u64()) {
        built = built.with_metadata("byte_length", byte_length.to_string());
    }

    if let Some(modality) = built.input_modality() {
        validate_part_modality_reference(&built, modality)?;
    }

    Ok(built)
}

fn parse_part_kind(kind: &str) -> KernelResult<AgentPartKind> {
    match kind {
        "text" => Ok(AgentPartKind::Text),
        "json" => Ok(AgentPartKind::Json),
        "binary_ref" => Ok(AgentPartKind::BinaryRef),
        "file_ref" => Ok(AgentPartKind::FileRef),
        "artifact_ref" => Ok(AgentPartKind::ArtifactRef),
        "image_ref" => Ok(AgentPartKind::ImageRef),
        "audio_ref" => Ok(AgentPartKind::AudioRef),
        "video_ref" => Ok(AgentPartKind::VideoRef),
        "tool_call_ref" => Ok(AgentPartKind::ToolCallRef),
        "policy_decision_ref" => Ok(AgentPartKind::PolicyDecisionRef),
        "error" => Ok(AgentPartKind::Error),
        _ => Err(KernelError::validation(format!(
            "unknown chat RPC part kind: {kind}"
        ))),
    }
}

fn required_string_field(
    part: &serde_json::Value,
    field: &str,
    context: &str,
) -> KernelResult<String> {
    part.get(field)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            KernelError::validation(format!("{context} requires non-empty field: {field}"))
        })
}

fn required_content_ref(part: &serde_json::Value) -> KernelResult<String> {
    let content_ref = required_string_field(part, "content_ref", "content reference part")?;
    ContentReference::parse(&content_ref)?;
    Ok(content_ref)
}

fn validate_part_modality_reference(
    part: &AgentPart,
    modality: AgentInputModality,
) -> KernelResult<()> {
    if matches!(
        modality,
        AgentInputModality::Image
            | AgentInputModality::Audio
            | AgentInputModality::Video
            | AgentInputModality::File
            | AgentInputModality::Binary
    ) && part.content_ref.is_none()
    {
        return Err(KernelError::validation(format!(
            "modality {} requires content_ref",
            modality.as_str()
        )));
    }
    Ok(())
}
