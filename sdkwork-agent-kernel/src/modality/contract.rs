//! Standard agent interaction contract — separates product I/O from API wire encoding.
//!
//! | Layer | Owns | Does not own |
//! | --- | --- | --- |
//! | `AgentInteractionContract` | accepted modalities, delivery strategy, limits | OpenAI/Anthropic/Gemini part JSON |
//! | `AgentMessage` / `ContentReference` | canonical kernel objects | HTTP routes |
//! | `ProtocolAdapter` | ingress/egress wire ↔ kernel | model invocation |
//! | `ModelProvider` | kernel messages ↔ vendor model API | agent business policy |

use super::kind::{validate_unique_modalities, AgentInputModality};
use super::policy::{AgentInputPolicy, UnsupportedInputModalityAction};
use crate::KernelResult;

/// How a modality reaches the model when native support is missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ModelDeliveryStrategy {
    /// Pass through only when the selected model declares the modality.
    #[default]
    Native,
    /// Run a named preprocessor (skill/tool) and deliver as another modality.
    Preprocess {
        processor_id: String,
        output_modality: AgentInputModality,
    },
    /// Fail closed before model invocation.
    Reject,
}

impl ModelDeliveryStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Preprocess { .. } => "preprocess",
            Self::Reject => "reject",
        }
    }

    pub fn parse(
        kind: &str,
        processor_id: Option<&str>,
        output_modality: Option<&str>,
    ) -> crate::KernelResult<Self> {
        match kind {
            "native" => Ok(Self::Native),
            "reject" => Ok(Self::Reject),
            "preprocess" => {
                let processor_id = processor_id.ok_or_else(|| {
                    crate::KernelError::validation("preprocess delivery requires processor_id")
                })?;
                let output = output_modality.unwrap_or("text");
                Ok(Self::Preprocess {
                    processor_id: processor_id.to_string(),
                    output_modality: AgentInputModality::parse(output)?,
                })
            }
            _ => Err(crate::KernelError::validation(format!(
                "unknown model delivery strategy: {kind}"
            ))),
        }
    }
}

/// Per-modality slot in the agent input contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalitySlot {
    pub modality: AgentInputModality,
    pub enabled: bool,
    pub max_parts_per_message: Option<u32>,
    pub allowed_mime_types: Vec<String>,
    pub max_bytes: Option<u64>,
    pub delivery: ModelDeliveryStrategy,
}

impl ModalitySlot {
    pub fn enabled(modality: AgentInputModality) -> Self {
        Self {
            modality,
            enabled: true,
            max_parts_per_message: None,
            allowed_mime_types: Vec::new(),
            max_bytes: None,
            delivery: ModelDeliveryStrategy::Native,
        }
    }

    pub fn with_delivery(mut self, delivery: ModelDeliveryStrategy) -> Self {
        self.delivery = delivery;
        self
    }

    pub fn with_max_parts_per_message(mut self, max_parts_per_message: u32) -> Self {
        self.max_parts_per_message = Some(max_parts_per_message);
        self
    }

    pub fn with_allowed_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.allowed_mime_types.push(mime_type.into());
        self
    }

    pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = Some(max_bytes);
        self
    }

    pub fn accepts_mime_type(&self, mime_type: &str) -> bool {
        self.allowed_mime_types.is_empty()
            || self
                .allowed_mime_types
                .iter()
                .any(|allowed| allowed == mime_type)
    }
}

/// Standard input side of `AgentInteractionContract`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInputContract {
    pub slots: Vec<ModalitySlot>,
    pub require_model_support: bool,
    pub unsupported_action: UnsupportedInputModalityAction,
}

impl Default for AgentInputContract {
    fn default() -> Self {
        Self::from_legacy_policy(&AgentInputPolicy::default())
    }
}

impl AgentInputContract {
    pub fn text_only() -> Self {
        Self::from_legacy_policy(&AgentInputPolicy::text_only())
    }

    pub fn multimodal_chat() -> Self {
        Self::from_legacy_policy(&AgentInputPolicy::multimodal_chat())
    }

    pub fn from_legacy_policy(policy: &AgentInputPolicy) -> Self {
        let slots = policy
            .accepted_modalities
            .iter()
            .map(|modality| ModalitySlot::enabled(*modality))
            .collect();
        Self {
            slots,
            require_model_support: policy.require_model_support,
            unsupported_action: policy.unsupported_action,
        }
    }

    pub fn to_legacy_policy(&self) -> AgentInputPolicy {
        AgentInputPolicy {
            accepted_modalities: self.enabled_modalities(),
            require_model_support: self.require_model_support,
            unsupported_action: self.unsupported_action,
        }
    }

    pub fn slot(&self, modality: AgentInputModality) -> Option<&ModalitySlot> {
        self.slots
            .iter()
            .find(|slot| slot.modality == modality && slot.enabled)
    }

    pub fn accepts_modality(&self, modality: AgentInputModality) -> bool {
        self.slot(modality).is_some()
    }

    pub fn enabled_modalities(&self) -> Vec<AgentInputModality> {
        self.slots
            .iter()
            .filter(|slot| slot.enabled)
            .map(|slot| slot.modality)
            .collect()
    }

    pub fn delivery_for(&self, modality: AgentInputModality) -> ModelDeliveryStrategy {
        self.slot(modality)
            .map(|slot| slot.delivery.clone())
            .unwrap_or(ModelDeliveryStrategy::Reject)
    }

    pub fn to_card_input_modes(&self) -> Vec<String> {
        self.to_legacy_policy().to_card_input_modes()
    }

    pub fn validate(&self) -> KernelResult<()> {
        let modalities: Vec<_> = self.slots.iter().map(|slot| slot.modality).collect();
        validate_unique_modalities(&modalities)?;
        if self.enabled_modalities().is_empty() {
            return Err(crate::KernelError::validation(
                "input contract must enable at least one modality slot",
            ));
        }
        self.to_legacy_policy().validate()
    }
}

/// Standard output side — projects to agent-card `output_modes`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentOutputContract {
    pub modalities: Vec<AgentInputModality>,
}

impl AgentOutputContract {
    pub fn text_json() -> Self {
        Self {
            modalities: vec![AgentInputModality::Text, AgentInputModality::Json],
        }
    }

    pub fn to_card_output_modes(&self) -> Vec<String> {
        let mut modes = Vec::new();
        for modality in &self.modalities {
            let card_mode = modality.to_card_mode().to_string();
            if !modes.contains(&card_mode) {
                modes.push(card_mode);
            }
        }
        modes
    }
}

/// Canonical I/O contract for `AgentDefinition` — API vendors map at adapter/provider layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInteractionContract {
    pub schema_version: String,
    pub input: AgentInputContract,
    pub output: AgentOutputContract,
}

impl Default for AgentInteractionContract {
    fn default() -> Self {
        Self::text_chat()
    }
}

impl AgentInteractionContract {
    pub fn text_chat() -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            input: AgentInputContract::text_only(),
            output: AgentOutputContract::text_json(),
        }
    }

    pub fn multimodal_chat() -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            input: AgentInputContract::multimodal_chat(),
            output: AgentOutputContract::text_json(),
        }
    }

    pub fn from_legacy_input_policy(policy: &AgentInputPolicy) -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            input: AgentInputContract::from_legacy_policy(policy),
            output: AgentOutputContract::text_json(),
        }
    }

    pub fn input_policy(&self) -> AgentInputPolicy {
        self.input.to_legacy_policy()
    }

    pub fn validate(&self) -> KernelResult<()> {
        self.input.validate()
    }
}
