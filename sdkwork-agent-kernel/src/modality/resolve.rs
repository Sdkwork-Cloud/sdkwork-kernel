//! Input modality resolution — policy checks, model compatibility, normalization.

use super::contract::AgentInputContract;
use super::contract::ModelDeliveryStrategy;
use super::delivery::{
    apply_delivery_transforms, enforce_slot_constraints, InputModalityPreprocessor,
};
use super::kind::AgentInputModality;
use super::policy::{AgentInputPolicy, UnsupportedInputModalityAction};
use crate::{AgentMessage, AgentPart, KernelError, KernelResult, ModelDescriptor};

/// Options for contract-aware multimodal resolution.
pub struct ModelInputResolveOptions<'a> {
    pub input_policy: &'a AgentInputPolicy,
    pub input_contract: Option<&'a AgentInputContract>,
    pub model_descriptor: Option<&'a ModelDescriptor>,
    pub preprocessor: Option<&'a dyn InputModalityPreprocessor>,
}

/// Per-part compatibility outcome used for diagnostics and UI hints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputModalityPartReport {
    pub part_id: String,
    pub modality: Option<AgentInputModality>,
    pub supported_by_model: bool,
    pub accepted_by_policy: bool,
}

/// Aggregate compatibility report for one message against policy and model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputModalityCompatibility {
    pub part_reports: Vec<InputModalityPartReport>,
    pub supported_part_ids: Vec<String>,
    pub unsupported_part_ids: Vec<String>,
    pub unsupported_modalities: Vec<AgentInputModality>,
    pub rejected_by_policy: Vec<AgentInputModality>,
}

impl InputModalityCompatibility {
    pub fn is_fully_supported(&self) -> bool {
        self.unsupported_part_ids.is_empty() && self.rejected_by_policy.is_empty()
    }

    pub fn model_capability_id(&self) -> Option<String> {
        if self.unsupported_modalities.is_empty() {
            None
        } else {
            Some(format!(
                "model.multimodal_input:{}",
                self.unsupported_modalities
                    .iter()
                    .map(AgentInputModality::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            ))
        }
    }
}

/// Normalized multimodal input ready for `ModelRequest` after policy/model resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInputResolution {
    pub messages: Vec<AgentMessage>,
    pub compatibility: InputModalityCompatibility,
    pub stripped_part_ids: Vec<String>,
}

impl AgentPart {
    pub fn input_modality(&self) -> Option<AgentInputModality> {
        AgentInputModality::from_part_kind(self.kind)
    }
}

impl AgentMessage {
    pub fn input_modalities(&self) -> Vec<AgentInputModality> {
        let mut modalities = Vec::new();
        for part in &self.parts {
            if let Some(modality) = part.input_modality() {
                if !modalities.contains(&modality) {
                    modalities.push(modality);
                }
            }
        }
        modalities
    }
}

impl ModelDescriptor {
    pub fn supports_input_modality(&self, modality: AgentInputModality) -> bool {
        self.input_modes
            .iter()
            .any(|mode| mode == modality.as_str())
    }

    pub fn supports_multimodal_input(&self) -> bool {
        self.supports_capability("model.multimodal_input")
            || self.supports_input_modality(AgentInputModality::Image)
            || self.supports_input_modality(AgentInputModality::Audio)
            || self.supports_input_modality(AgentInputModality::Video)
            || self.supports_input_modality(AgentInputModality::Music)
    }

    pub fn missing_input_modalities(
        &self,
        required: &[AgentInputModality],
    ) -> Vec<AgentInputModality> {
        required
            .iter()
            .copied()
            .filter(|modality| !self.supports_input_modality(*modality))
            .collect()
    }
}

pub fn part_kind_to_input_modality(kind: crate::AgentPartKind) -> Option<AgentInputModality> {
    AgentInputModality::from_part_kind(kind)
}

pub fn validate_message_against_input_policy(
    message: &AgentMessage,
    input_policy: &AgentInputPolicy,
) -> KernelResult<()> {
    let compatibility = analyze_message_input(message, input_policy, None);
    if !compatibility.rejected_by_policy.is_empty() {
        return Err(KernelError::validation(format!(
            "agent definition does not accept input modalities: {}",
            compatibility
                .rejected_by_policy
                .iter()
                .map(AgentInputModality::as_str)
                .collect::<Vec<_>>()
                .join(",")
        )));
    }
    Ok(())
}

pub fn check_message_against_model_descriptor(
    message: &AgentMessage,
    descriptor: &ModelDescriptor,
) -> InputModalityCompatibility {
    analyze_message_input(
        message,
        &AgentInputPolicy::multimodal_chat(),
        Some(descriptor),
    )
}

pub fn analyze_message_input(
    message: &AgentMessage,
    input_policy: &AgentInputPolicy,
    model_descriptor: Option<&ModelDescriptor>,
) -> InputModalityCompatibility {
    let mut part_reports = Vec::new();
    let mut supported_part_ids = Vec::new();
    let mut unsupported_part_ids = Vec::new();
    let mut unsupported_modalities = Vec::new();
    let mut rejected_by_policy = Vec::new();

    for part in &message.parts {
        let modality = part.input_modality();
        let accepted_by_policy = modality.is_none_or(|value| input_policy.accepts_modality(value));
        let supported_by_model = match (modality, model_descriptor) {
            (Some(value), Some(descriptor)) => descriptor.supports_input_modality(value),
            _ => true,
        };

        part_reports.push(InputModalityPartReport {
            part_id: part.part_id.clone(),
            modality,
            supported_by_model,
            accepted_by_policy,
        });

        if !accepted_by_policy {
            if let Some(value) = modality {
                if !rejected_by_policy.contains(&value) {
                    rejected_by_policy.push(value);
                }
            }
            unsupported_part_ids.push(part.part_id.clone());
            continue;
        }

        if supported_by_model {
            supported_part_ids.push(part.part_id.clone());
        } else {
            unsupported_part_ids.push(part.part_id.clone());
            if let Some(value) = modality {
                if !unsupported_modalities.contains(&value) {
                    unsupported_modalities.push(value);
                }
            }
        }
    }

    InputModalityCompatibility {
        part_reports,
        supported_part_ids,
        unsupported_part_ids,
        unsupported_modalities,
        rejected_by_policy,
    }
}

pub fn resolve_model_input(
    messages: &[AgentMessage],
    input_policy: &AgentInputPolicy,
    model_descriptor: Option<&ModelDescriptor>,
) -> KernelResult<ModelInputResolution> {
    resolve_model_input_with_options(
        messages,
        &ModelInputResolveOptions {
            input_policy,
            input_contract: None,
            model_descriptor,
            preprocessor: None,
        },
    )
}

pub fn resolve_model_input_with_options(
    messages: &[AgentMessage],
    options: &ModelInputResolveOptions<'_>,
) -> KernelResult<ModelInputResolution> {
    options.input_policy.validate()?;
    if let Some(contract) = options.input_contract {
        contract.validate()?;
    }

    let mut normalized = Vec::with_capacity(messages.len());
    let mut merged = InputModalityCompatibility {
        part_reports: Vec::new(),
        supported_part_ids: Vec::new(),
        unsupported_part_ids: Vec::new(),
        unsupported_modalities: Vec::new(),
        rejected_by_policy: Vec::new(),
    };
    let mut stripped_part_ids = Vec::new();

    for message in messages {
        message.validate()?;

        if let Some(contract) = options.input_contract {
            enforce_slot_constraints(message, contract)?;
        }

        let prepared = if let Some(contract) = options.input_contract {
            apply_delivery_transforms(message, contract, options.preprocessor)?
        } else {
            message.clone()
        };

        let compatibility =
            analyze_message_input(&prepared, options.input_policy, options.model_descriptor);
        merge_compatibility(&mut merged, &compatibility);

        if !compatibility.rejected_by_policy.is_empty() {
            return Err(KernelError::validation(format!(
                "input modalities [{}] are not accepted by agent policy",
                compatibility
                    .rejected_by_policy
                    .iter()
                    .map(AgentInputModality::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            )));
        }

        let resolved = if let Some(descriptor) = options.model_descriptor {
            if options.input_policy.require_model_support && !compatibility.is_fully_supported() {
                let force_reject = compatibility.unsupported_modalities.iter().any(|modality| {
                    options
                        .input_contract
                        .map(|contract| {
                            matches!(
                                contract.delivery_for(*modality),
                                ModelDeliveryStrategy::Reject
                            )
                        })
                        .unwrap_or(false)
                });
                if force_reject {
                    return Err(KernelError::CapabilityMissing {
                        capability_id: compatibility
                            .model_capability_id()
                            .unwrap_or_else(|| "model.multimodal_input".to_string()),
                    });
                }
                match options.input_policy.unsupported_action {
                    UnsupportedInputModalityAction::Reject => {
                        return Err(KernelError::CapabilityMissing {
                            capability_id: compatibility
                                .model_capability_id()
                                .unwrap_or_else(|| "model.multimodal_input".to_string()),
                        });
                    }
                    UnsupportedInputModalityAction::StripUnsupported => {
                        let (stripped, removed) = strip_unsupported_parts(&prepared, descriptor);
                        if stripped.parts.is_empty() {
                            return Err(KernelError::validation(
                                "no supported input parts remain after stripping unsupported modalities",
                            ));
                        }
                        stripped_part_ids.extend(removed);
                        stripped
                    }
                }
            } else {
                prepared
            }
        } else {
            prepared
        };

        normalized.push(resolved);
    }

    Ok(ModelInputResolution {
        messages: normalized,
        compatibility: merged,
        stripped_part_ids,
    })
}

pub fn validate_structured_model_input(
    messages: &[AgentMessage],
    input_policy: &AgentInputPolicy,
    model_descriptor: Option<&ModelDescriptor>,
) -> KernelResult<Vec<AgentMessage>> {
    resolve_model_input(messages, input_policy, model_descriptor)
        .map(|resolution| resolution.messages)
}

pub fn validate_structured_model_input_with_options(
    messages: &[AgentMessage],
    options: &ModelInputResolveOptions<'_>,
) -> KernelResult<Vec<AgentMessage>> {
    resolve_model_input_with_options(messages, options).map(|resolution| resolution.messages)
}

fn strip_unsupported_parts(
    message: &AgentMessage,
    descriptor: &ModelDescriptor,
) -> (AgentMessage, Vec<String>) {
    let mut removed = Vec::new();
    let parts = message
        .parts
        .iter()
        .filter(|part| {
            let keep = part
                .input_modality()
                .is_none_or(|modality| descriptor.supports_input_modality(modality));
            if !keep {
                removed.push(part.part_id.clone());
            }
            keep
        })
        .cloned()
        .collect();

    let mut stripped = AgentMessage::new(message.message_id.clone(), message.role, parts);
    if let Some(session_id) = &message.session_id {
        stripped = stripped.for_session(session_id.clone());
    }
    if let Some(task_id) = &message.task_id {
        stripped = stripped.for_task(task_id.clone());
    }
    if let Some(run_id) = &message.run_id {
        stripped = stripped.for_run(run_id.clone());
    }
    if let Some(step_id) = &message.step_id {
        stripped = stripped.for_step(step_id.clone());
    }
    if let Some(created_at) = &message.created_at {
        stripped = stripped.created_at(created_at.clone());
    }
    if let Some(trace_context) = &message.trace_context {
        stripped = stripped.with_trace_context(trace_context.clone());
    }
    for (key, value) in &message.metadata {
        stripped = stripped.with_metadata(key.clone(), value.clone());
    }
    if message.untrusted {
        stripped = stripped.mark_untrusted();
    }

    (stripped, removed)
}

fn merge_compatibility(
    target: &mut InputModalityCompatibility,
    source: &InputModalityCompatibility,
) {
    target.part_reports.extend(source.part_reports.clone());
    for part_id in &source.supported_part_ids {
        if !target.supported_part_ids.contains(part_id) {
            target.supported_part_ids.push(part_id.clone());
        }
    }
    for part_id in &source.unsupported_part_ids {
        if !target.unsupported_part_ids.contains(part_id) {
            target.unsupported_part_ids.push(part_id.clone());
        }
    }
    for modality in &source.unsupported_modalities {
        if !target.unsupported_modalities.contains(modality) {
            target.unsupported_modalities.push(*modality);
        }
    }
    for modality in &source.rejected_by_policy {
        if !target.rejected_by_policy.contains(modality) {
            target.rejected_by_policy.push(*modality);
        }
    }
}
