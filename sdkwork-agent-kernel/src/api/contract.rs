//! Fluent builder for `AgentInteractionContract`.

use crate::{
    AgentInputContract, AgentInputModality, AgentInteractionContract, AgentOutputContract,
    KernelResult, ModalitySlot, ModelDeliveryStrategy,
};

/// Builds an interaction contract with per-modality delivery policies.
#[derive(Debug, Clone)]
pub struct InteractionContractBuilder {
    slots: Vec<ModalitySlot>,
    require_model_support: bool,
    output_modalities: Vec<AgentInputModality>,
}

impl InteractionContractBuilder {
    pub fn text_chat() -> Self {
        Self {
            slots: vec![
                ModalitySlot::enabled(AgentInputModality::Text),
                ModalitySlot::enabled(AgentInputModality::Json),
            ],
            require_model_support: true,
            output_modalities: vec![AgentInputModality::Text, AgentInputModality::Json],
        }
    }

    pub fn multimodal_chat() -> Self {
        Self {
            slots: vec![
                ModalitySlot::enabled(AgentInputModality::Text),
                ModalitySlot::enabled(AgentInputModality::Json),
                ModalitySlot::enabled(AgentInputModality::Image),
                ModalitySlot::enabled(AgentInputModality::Audio),
                ModalitySlot::enabled(AgentInputModality::Video),
                ModalitySlot::enabled(AgentInputModality::File),
            ],
            require_model_support: true,
            output_modalities: vec![AgentInputModality::Text, AgentInputModality::Json],
        }
    }

    pub fn enable(mut self, modality: AgentInputModality) -> Self {
        if !self.slots.iter().any(|slot| slot.modality == modality) {
            self.slots.push(ModalitySlot::enabled(modality));
        }
        self
    }

    pub fn preprocess_audio(mut self, processor_id: impl Into<String>) -> Self {
        self.upsert_slot(
            AgentInputModality::Audio,
            ModalitySlot::enabled(AgentInputModality::Audio).with_delivery(
                ModelDeliveryStrategy::Preprocess {
                    processor_id: processor_id.into(),
                    output_modality: AgentInputModality::Text,
                },
            ),
        );
        self
    }

    pub fn require_model_support(mut self, require_model_support: bool) -> Self {
        self.require_model_support = require_model_support;
        self
    }

    pub fn output_text_json(mut self) -> Self {
        self.output_modalities = vec![AgentInputModality::Text, AgentInputModality::Json];
        self
    }

    pub fn build(self) -> KernelResult<AgentInteractionContract> {
        let contract = AgentInteractionContract {
            schema_version: "1.0.0".to_string(),
            input: AgentInputContract {
                slots: self.slots,
                require_model_support: self.require_model_support,
                unsupported_action: crate::UnsupportedInputModalityAction::Reject,
            },
            output: AgentOutputContract {
                modalities: self.output_modalities,
            },
        };
        contract.validate()?;
        Ok(contract)
    }

    fn upsert_slot(&mut self, modality: AgentInputModality, slot: ModalitySlot) {
        if let Some(existing) = self.slots.iter_mut().find(|item| item.modality == modality) {
            *existing = slot;
        } else {
            self.slots.push(slot);
        }
    }
}
