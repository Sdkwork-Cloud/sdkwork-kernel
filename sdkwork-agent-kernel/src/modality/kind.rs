//! Canonical input modality identifiers shared by agent definitions, agent cards,
//! model descriptors, and `sdkwork-models` catalog records.

use crate::{AgentPartKind, KernelError, KernelResult};

pub const INPUT_MODALITY_TEXT: &str = "text";
pub const INPUT_MODALITY_IMAGE: &str = "image";
pub const INPUT_MODALITY_AUDIO: &str = "audio";
pub const INPUT_MODALITY_VIDEO: &str = "video";
pub const INPUT_MODALITY_MUSIC: &str = "music";
pub const INPUT_MODALITY_JSON: &str = "json";
pub const INPUT_MODALITY_FILE: &str = "file";
pub const INPUT_MODALITY_BINARY: &str = "binary";
pub const INPUT_MODALITY_ARTIFACT: &str = "artifact";

/// Catalog-aligned modality ids (`sdkwork-models::MODEL_INPUT_MODALITIES` superset).
pub const INPUT_MODALITIES: &[&str] = &[
    INPUT_MODALITY_TEXT,
    INPUT_MODALITY_JSON,
    INPUT_MODALITY_FILE,
    INPUT_MODALITY_BINARY,
    INPUT_MODALITY_ARTIFACT,
    INPUT_MODALITY_IMAGE,
    INPUT_MODALITY_AUDIO,
    INPUT_MODALITY_VIDEO,
    INPUT_MODALITY_MUSIC,
];

/// Public agent-card `input_modes` vocabulary (A2A-safe subset).
pub const CARD_INPUT_MODES: &[&str] = &[
    INPUT_MODALITY_TEXT,
    INPUT_MODALITY_JSON,
    INPUT_MODALITY_FILE,
    INPUT_MODALITY_IMAGE,
    INPUT_MODALITY_AUDIO,
    INPUT_MODALITY_VIDEO,
    INPUT_MODALITY_ARTIFACT,
];

/// Kernel-owned input modality for agent definitions and model compatibility checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentInputModality {
    Text,
    Json,
    File,
    Binary,
    Artifact,
    Image,
    Audio,
    Video,
    Music,
}

impl AgentInputModality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => INPUT_MODALITY_TEXT,
            Self::Json => INPUT_MODALITY_JSON,
            Self::File => INPUT_MODALITY_FILE,
            Self::Binary => INPUT_MODALITY_BINARY,
            Self::Artifact => INPUT_MODALITY_ARTIFACT,
            Self::Image => INPUT_MODALITY_IMAGE,
            Self::Audio => INPUT_MODALITY_AUDIO,
            Self::Video => INPUT_MODALITY_VIDEO,
            Self::Music => INPUT_MODALITY_MUSIC,
        }
    }

    pub fn parse(input: &str) -> KernelResult<Self> {
        match input {
            INPUT_MODALITY_TEXT => Ok(Self::Text),
            INPUT_MODALITY_JSON => Ok(Self::Json),
            INPUT_MODALITY_FILE => Ok(Self::File),
            INPUT_MODALITY_BINARY => Ok(Self::Binary),
            INPUT_MODALITY_ARTIFACT => Ok(Self::Artifact),
            INPUT_MODALITY_IMAGE => Ok(Self::Image),
            INPUT_MODALITY_AUDIO => Ok(Self::Audio),
            INPUT_MODALITY_VIDEO => Ok(Self::Video),
            INPUT_MODALITY_MUSIC => Ok(Self::Music),
            _ => Err(KernelError::validation(format!(
                "unknown input modality: {input}"
            ))),
        }
    }

    pub fn from_part_kind(kind: AgentPartKind) -> Option<Self> {
        match kind {
            AgentPartKind::Text | AgentPartKind::Error => Some(Self::Text),
            AgentPartKind::Json => Some(Self::Json),
            AgentPartKind::FileRef => Some(Self::File),
            AgentPartKind::BinaryRef => Some(Self::Binary),
            AgentPartKind::ArtifactRef => Some(Self::Artifact),
            AgentPartKind::ImageRef => Some(Self::Image),
            AgentPartKind::AudioRef => Some(Self::Audio),
            AgentPartKind::VideoRef => Some(Self::Video),
            AgentPartKind::ToolCallRef | AgentPartKind::PolicyDecisionRef => None,
        }
    }

    pub fn to_part_kind(self) -> AgentPartKind {
        match self {
            Self::Text => AgentPartKind::Text,
            Self::Json => AgentPartKind::Json,
            Self::File => AgentPartKind::FileRef,
            Self::Binary => AgentPartKind::BinaryRef,
            Self::Artifact => AgentPartKind::ArtifactRef,
            Self::Image => AgentPartKind::ImageRef,
            Self::Audio => AgentPartKind::AudioRef,
            Self::Video => AgentPartKind::VideoRef,
            Self::Music => AgentPartKind::AudioRef,
        }
    }

    /// Maps kernel modality to public agent-card `input_modes` entries.
    pub fn to_card_mode(self) -> &'static str {
        match self {
            Self::Binary => INPUT_MODALITY_FILE,
            Self::Music => INPUT_MODALITY_AUDIO,
            Self::Json
            | Self::Text
            | Self::File
            | Self::Artifact
            | Self::Image
            | Self::Audio
            | Self::Video => self.as_str(),
        }
    }

    pub fn is_media(self) -> bool {
        matches!(self, Self::Image | Self::Audio | Self::Video | Self::Music)
    }

    pub fn requires_multimodal_model(self) -> bool {
        self.is_media()
    }

    pub fn is_speech_input(self) -> bool {
        matches!(self, Self::Audio | Self::Music)
    }
}

pub fn parse_input_modalities(values: &[String]) -> KernelResult<Vec<AgentInputModality>> {
    values
        .iter()
        .map(|value| AgentInputModality::parse(value))
        .collect()
}

pub fn validate_unique_modalities(modalities: &[AgentInputModality]) -> KernelResult<()> {
    for (index, modality) in modalities.iter().enumerate() {
        if modalities
            .iter()
            .skip(index + 1)
            .any(|other| other == modality)
        {
            return Err(KernelError::validation(format!(
                "duplicate input modality: {}",
                modality.as_str()
            )));
        }
    }
    Ok(())
}

/// Infer catalog modality from a MIME type (`image/*`, `audio/*`, `video/*`, ...).
pub fn infer_modality_from_mime_type(mime_type: &str) -> Option<AgentInputModality> {
    let mime = mime_type.trim().to_ascii_lowercase();
    if mime.starts_with("image/") {
        Some(AgentInputModality::Image)
    } else if mime.starts_with("audio/") {
        Some(AgentInputModality::Audio)
    } else if mime.starts_with("video/") {
        Some(AgentInputModality::Video)
    } else if mime == "application/json" || mime.ends_with("+json") {
        Some(AgentInputModality::Json)
    } else if mime.starts_with("text/") {
        Some(AgentInputModality::Text)
    } else {
        None
    }
}
