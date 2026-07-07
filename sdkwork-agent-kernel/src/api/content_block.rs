//! Industry-aligned content blocks — converged model for multimodal chat APIs.
//!
//! | `ContentBlock` | OpenAI | Anthropic | Gemini | Kernel `AgentPartKind` |
//! | --- | --- | --- | --- | --- |
//! | `Text` | `text` | `text` | `text` | `text` |
//! | `Json` | — | — | — | `json` |
//! | `Image` | `image_url` | `image` | `inline_data` (image) | `image_ref` |
//! | `Audio` | `input_audio` | — | `inline_data` (audio) | `audio_ref` |
//! | `Video` | — | — | `video` | `video_ref` |
//! | `File` | `file` | `document` | `file_data` | `file_ref` |
//! | `Artifact` | — | — | — | `artifact_ref` |
//! | `ToolCall` | `tool_calls[]` | `tool_use` | `functionCall` | `tool_call_ref` |
//! | `ToolResult` | `tool` message | `tool_result` | `functionResponse` | `text` + metadata |

use crate::{AgentPart, AgentPartKind, ContentReference, KernelError, KernelResult};

/// Typed multimodal content unit — maps to exactly one kernel `AgentPart`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Json {
        value: String,
        schema: Option<String>,
    },
    Image {
        reference: ContentReference,
        mime_type: String,
        name: Option<String>,
    },
    Audio {
        reference: ContentReference,
        mime_type: String,
        name: Option<String>,
    },
    Video {
        reference: ContentReference,
        mime_type: String,
        name: Option<String>,
    },
    File {
        reference: ContentReference,
        mime_type: String,
        name: Option<String>,
    },
    Artifact {
        artifact_id: String,
    },
    ToolCall {
        tool_call_id: String,
        name: Option<String>,
    },
    ToolResult {
        tool_call_id: String,
        text: String,
    },
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn image(reference: ContentReference, mime_type: impl Into<String>) -> Self {
        Self::Image {
            reference,
            mime_type: mime_type.into(),
            name: None,
        }
    }

    pub fn image_url(uri: impl AsRef<str>, mime_type: impl Into<String>) -> KernelResult<Self> {
        Ok(Self::image(
            ContentReference::parse(uri.as_ref())?,
            mime_type,
        ))
    }

    pub fn audio(reference: ContentReference, mime_type: impl Into<String>) -> Self {
        Self::Audio {
            reference,
            mime_type: mime_type.into(),
            name: None,
        }
    }

    pub fn video(reference: ContentReference, mime_type: impl Into<String>) -> Self {
        Self::Video {
            reference,
            mime_type: mime_type.into(),
            name: None,
        }
    }

    pub fn file(reference: ContentReference, mime_type: impl Into<String>) -> Self {
        Self::File {
            reference,
            mime_type: mime_type.into(),
            name: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_call_id: tool_call_id.into(),
            text: text.into(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        let name = Some(name.into());
        match &mut self {
            Self::Image { name: slot, .. }
            | Self::Audio { name: slot, .. }
            | Self::Video { name: slot, .. }
            | Self::File { name: slot, .. }
            | Self::ToolCall { name: slot, .. } => *slot = name,
            _ => {}
        }
        self
    }

    pub fn to_part(self, part_id: impl Into<String>) -> KernelResult<AgentPart> {
        let part_id = part_id.into();
        let part = match self {
            Self::Text { text } => AgentPart::text(part_id, text),
            Self::Json { value, schema } => {
                let mut part = AgentPart::json(part_id, value);
                if let Some(schema) = schema {
                    part = part.with_schema(schema);
                }
                part
            }
            Self::Image {
                reference,
                mime_type,
                name,
            } => media_part(part_id, AgentPartKind::ImageRef, reference, mime_type, name)?,
            Self::Audio {
                reference,
                mime_type,
                name,
            } => media_part(part_id, AgentPartKind::AudioRef, reference, mime_type, name)?,
            Self::Video {
                reference,
                mime_type,
                name,
            } => media_part(part_id, AgentPartKind::VideoRef, reference, mime_type, name)?,
            Self::File {
                reference,
                mime_type,
                name,
            } => media_part(part_id, AgentPartKind::FileRef, reference, mime_type, name)?,
            Self::Artifact { artifact_id } => AgentPart::artifact_ref(part_id, artifact_id),
            Self::ToolCall { tool_call_id, name } => {
                let mut part = AgentPart::tool_call_ref(part_id, tool_call_id);
                if let Some(name) = name {
                    part = part.with_name(name);
                }
                part
            }
            Self::ToolResult { tool_call_id, text } => AgentPart::text(part_id, text)
                .with_metadata("tool_call_id", tool_call_id)
                .with_metadata("content_block", "tool_result"),
        };
        Ok(part)
    }

    pub fn from_part(part: &AgentPart) -> KernelResult<Self> {
        if part.metadata_value("content_block") == Some("tool_result") {
            let tool_call_id = part
                .metadata_value("tool_call_id")
                .ok_or_else(|| KernelError::validation("tool result part missing tool_call_id"))?;
            return Ok(Self::ToolResult {
                tool_call_id: tool_call_id.to_string(),
                text: part.text.clone().unwrap_or_default(),
            });
        }

        match part.kind {
            AgentPartKind::Text => Ok(Self::Text {
                text: part
                    .text
                    .clone()
                    .ok_or_else(|| KernelError::validation("text part missing text"))?,
            }),
            AgentPartKind::Json => Ok(Self::Json {
                value: part
                    .json
                    .clone()
                    .ok_or_else(|| KernelError::validation("json part missing json"))?,
                schema: part.schema.clone(),
            }),
            AgentPartKind::ImageRef => {
                media_block_from_part(part, |reference, mime_type, name| Self::Image {
                    reference,
                    mime_type,
                    name,
                })
            }
            AgentPartKind::AudioRef => {
                media_block_from_part(part, |reference, mime_type, name| Self::Audio {
                    reference,
                    mime_type,
                    name,
                })
            }
            AgentPartKind::VideoRef => {
                media_block_from_part(part, |reference, mime_type, name| Self::Video {
                    reference,
                    mime_type,
                    name,
                })
            }
            AgentPartKind::FileRef | AgentPartKind::BinaryRef => {
                media_block_from_part(part, |reference, mime_type, name| Self::File {
                    reference,
                    mime_type,
                    name,
                })
            }
            AgentPartKind::ArtifactRef => Ok(Self::Artifact {
                artifact_id: part
                    .artifact_id
                    .clone()
                    .ok_or_else(|| KernelError::validation("artifact part missing artifact_id"))?,
            }),
            AgentPartKind::ToolCallRef => Ok(Self::ToolCall {
                tool_call_id: part.tool_call_id.clone().ok_or_else(|| {
                    KernelError::validation("tool call part missing tool_call_id")
                })?,
                name: part.name.clone(),
            }),
            AgentPartKind::PolicyDecisionRef | AgentPartKind::Error => {
                Err(KernelError::validation(format!(
                    "content block mapping is not defined for part kind {}",
                    part.kind.as_str()
                )))
            }
        }
    }
}

fn media_part(
    part_id: String,
    kind: AgentPartKind,
    reference: ContentReference,
    mime_type: String,
    name: Option<String>,
) -> KernelResult<AgentPart> {
    let mut part = match kind {
        AgentPartKind::ImageRef => AgentPart::image_ref(part_id, reference.uri, mime_type),
        AgentPartKind::AudioRef => AgentPart::audio_ref(part_id, reference.uri, mime_type),
        AgentPartKind::VideoRef => AgentPart::video_ref(part_id, reference.uri, mime_type),
        AgentPartKind::FileRef => AgentPart::file_ref(part_id, reference.uri, mime_type),
        _ => {
            return Err(KernelError::validation(format!(
                "unsupported media part kind: {}",
                kind.as_str()
            )));
        }
    };
    if let Some(name) = name {
        part = part.with_name(name);
    }
    Ok(part)
}

fn media_block_from_part<F>(part: &AgentPart, build: F) -> KernelResult<ContentBlock>
where
    F: FnOnce(ContentReference, String, Option<String>) -> ContentBlock,
{
    let reference = part
        .content_reference()?
        .ok_or_else(|| KernelError::validation("media part missing content reference"))?;
    let mime_type = part
        .mime_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    Ok(build(reference, mime_type, part.name.clone()))
}
