//! Agent-level input policy — declares accepted modalities and model-gap handling.

use super::kind::{validate_unique_modalities, AgentInputModality, CARD_INPUT_MODES};
use crate::KernelResult;

/// Action when a message part modality is not supported by the selected model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnsupportedInputModalityAction {
    /// Fail the request before model invocation.
    #[default]
    Reject,
    /// Drop unsupported parts and continue when at least one supported part remains.
    StripUnsupported,
}

impl UnsupportedInputModalityAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::StripUnsupported => "strip_unsupported",
        }
    }

    pub fn parse(input: &str) -> crate::KernelResult<Self> {
        match input {
            "reject" => Ok(Self::Reject),
            "strip_unsupported" => Ok(Self::StripUnsupported),
            _ => Err(crate::KernelError::validation(format!(
                "unknown unsupported input modality action: {input}"
            ))),
        }
    }
}

/// Declares which input modalities an agent accepts and how to handle model gaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInputPolicy {
    pub accepted_modalities: Vec<AgentInputModality>,
    pub require_model_support: bool,
    pub unsupported_action: UnsupportedInputModalityAction,
}

impl Default for AgentInputPolicy {
    fn default() -> Self {
        Self::text_only()
    }
}

impl AgentInputPolicy {
    pub fn text_only() -> Self {
        Self {
            accepted_modalities: vec![AgentInputModality::Text, AgentInputModality::Json],
            require_model_support: true,
            unsupported_action: UnsupportedInputModalityAction::Reject,
        }
    }

    pub fn multimodal_chat() -> Self {
        Self {
            accepted_modalities: vec![
                AgentInputModality::Text,
                AgentInputModality::Json,
                AgentInputModality::Image,
                AgentInputModality::Audio,
                AgentInputModality::Video,
                AgentInputModality::File,
            ],
            require_model_support: true,
            unsupported_action: UnsupportedInputModalityAction::Reject,
        }
    }

    pub fn voice_and_vision() -> Self {
        Self::multimodal_chat()
    }

    pub fn with_modality(mut self, modality: AgentInputModality) -> Self {
        if !self.accepted_modalities.contains(&modality) {
            self.accepted_modalities.push(modality);
        }
        self
    }

    pub fn with_require_model_support(mut self, require_model_support: bool) -> Self {
        self.require_model_support = require_model_support;
        self
    }

    pub fn with_unsupported_action(
        mut self,
        unsupported_action: UnsupportedInputModalityAction,
    ) -> Self {
        self.unsupported_action = unsupported_action;
        self
    }

    pub fn accepts_modality(&self, modality: AgentInputModality) -> bool {
        self.accepted_modalities.contains(&modality)
    }

    pub fn accepts_speech_input(&self) -> bool {
        self.accepts_modality(AgentInputModality::Audio)
            || self.accepts_modality(AgentInputModality::Music)
    }

    pub fn accepts_vision_input(&self) -> bool {
        self.accepts_modality(AgentInputModality::Image)
            || self.accepts_modality(AgentInputModality::Video)
    }

    pub fn requires_multimodal_model(&self) -> bool {
        self.accepted_modalities
            .iter()
            .copied()
            .any(AgentInputModality::requires_multimodal_model)
    }

    /// Projects policy into public agent-card `input_modes`.
    pub fn to_card_input_modes(&self) -> Vec<String> {
        let mut modes = Vec::new();
        for modality in &self.accepted_modalities {
            let card_mode = modality.to_card_mode().to_string();
            if !modes.iter().any(|existing| existing == &card_mode) {
                modes.push(card_mode);
            }
        }
        modes
    }

    pub fn from_card_input_modes(modes: &[String]) -> crate::KernelResult<Self> {
        let mut accepted = Vec::new();
        for mode in modes {
            let parsed = match mode.as_str() {
                "text" => AgentInputModality::Text,
                "json" => AgentInputModality::Json,
                "file" => AgentInputModality::File,
                "image" => AgentInputModality::Image,
                "audio" => AgentInputModality::Audio,
                "video" => AgentInputModality::Video,
                "artifact" => AgentInputModality::Artifact,
                other => AgentInputModality::parse(other)?,
            };
            if !accepted.contains(&parsed) {
                accepted.push(parsed);
            }
        }
        if accepted.is_empty() {
            return Err(crate::KernelError::validation(
                "agent card input_modes must not be empty",
            ));
        }
        Ok(Self {
            accepted_modalities: accepted,
            require_model_support: true,
            unsupported_action: UnsupportedInputModalityAction::Reject,
        })
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.accepted_modalities.is_empty() {
            return Err(crate::KernelError::validation(
                "input policy must accept at least one modality",
            ));
        }
        validate_unique_modalities(&self.accepted_modalities)?;
        for mode in self.to_card_input_modes() {
            if !CARD_INPUT_MODES.contains(&mode.as_str()) {
                return Err(crate::KernelError::validation(format!(
                    "input policy projects unknown card input mode: {mode}"
                )));
            }
        }
        Ok(())
    }
}
