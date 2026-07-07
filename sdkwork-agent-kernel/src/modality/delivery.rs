//! Modality delivery — slot constraints and preprocess execution before model invoke.

use super::contract::{AgentInputContract, ModelDeliveryStrategy};
use super::kind::AgentInputModality;
use crate::{
    AgentMessage, AgentPart, AgentSkillProvider, AgentSkillRequest, AgentSkillStatus, KernelError,
    KernelResult,
};

/// Transforms a multimodal part before model invocation (skill/tool gateway).
pub trait InputModalityPreprocessor {
    fn preprocess_part(
        &self,
        processor_id: &str,
        part: &AgentPart,
        output_modality: AgentInputModality,
    ) -> KernelResult<AgentPart>;
}

/// Skill-backed preprocessor — maps `processor_id` to `AgentSkillProvider::invoke_skill`.
pub struct SkillInputModalityPreprocessor<'a> {
    pub skill_provider: &'a dyn AgentSkillProvider,
}

impl InputModalityPreprocessor for SkillInputModalityPreprocessor<'_> {
    fn preprocess_part(
        &self,
        processor_id: &str,
        part: &AgentPart,
        output_modality: AgentInputModality,
    ) -> KernelResult<AgentPart> {
        let mut request =
            AgentSkillRequest::new(format!("input.preprocess.{}", part.part_id), processor_id);
        if let Some(content_ref) = &part.content_ref {
            request = request.with_argument("content_ref", content_ref.clone());
        }
        if let Some(artifact_id) = &part.artifact_id {
            request = request.with_argument("artifact_id", artifact_id.clone());
        }
        if let Some(mime_type) = &part.mime_type {
            request = request.with_argument("mime_type", mime_type.clone());
        }
        if let Some(text) = &part.text {
            request = request.with_argument("text", text.clone());
        }
        request = request.with_argument("output_modality", output_modality.as_str());

        let result = self.skill_provider.invoke_skill(request)?;
        if result.status != AgentSkillStatus::Succeeded {
            return Err(KernelError::provider_error(
                format!("input.preprocess.{processor_id}"),
                result
                    .error
                    .unwrap_or_else(|| format!("preprocess skill {processor_id} failed")),
            ));
        }

        Ok(part_from_preprocessed_output(
            &part.part_id,
            output_modality,
            &result.output,
        ))
    }
}

pub fn enforce_slot_constraints(
    message: &AgentMessage,
    input_contract: &AgentInputContract,
) -> KernelResult<()> {
    let mut counts: std::collections::HashMap<AgentInputModality, u32> =
        std::collections::HashMap::new();

    for part in &message.parts {
        let Some(modality) = part.input_modality() else {
            continue;
        };
        let slot = input_contract.slot(modality).ok_or_else(|| {
            KernelError::validation(format!(
                "input modality {} is not enabled in interaction contract",
                modality.as_str()
            ))
        })?;

        if let Some(mime_type) = part.mime_type.as_deref() {
            if !slot.accepts_mime_type(mime_type) {
                return Err(KernelError::validation(format!(
                    "mime type {mime_type} is not allowed for modality {}",
                    modality.as_str()
                )));
            }
        }

        if let Some(max_bytes) = slot.max_bytes {
            let byte_length = part
                .metadata_value("byte_length")
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| {
                    KernelError::validation(format!(
                        "part {} requires metadata.byte_length when max_bytes is configured",
                        part.part_id
                    ))
                })?;
            if byte_length > max_bytes {
                return Err(KernelError::validation(format!(
                    "part {} exceeds max_bytes ({max_bytes}) for modality {}",
                    part.part_id,
                    modality.as_str()
                )));
            }
        }

        *counts.entry(modality).or_insert(0) += 1;
        if let Some(max_parts) = slot.max_parts_per_message {
            if counts[&modality] > max_parts {
                return Err(KernelError::validation(format!(
                    "modality {} exceeds max_parts_per_message ({max_parts})",
                    modality.as_str()
                )));
            }
        }
    }

    Ok(())
}

pub fn apply_delivery_transforms(
    message: &AgentMessage,
    input_contract: &AgentInputContract,
    preprocessor: Option<&dyn InputModalityPreprocessor>,
) -> KernelResult<AgentMessage> {
    let mut parts = Vec::with_capacity(message.parts.len());

    for part in &message.parts {
        let Some(modality) = part.input_modality() else {
            parts.push(part.clone());
            continue;
        };

        match input_contract.delivery_for(modality) {
            ModelDeliveryStrategy::Native | ModelDeliveryStrategy::Reject => {
                parts.push(part.clone());
            }
            ModelDeliveryStrategy::Preprocess {
                processor_id,
                output_modality,
            } => {
                let Some(preprocessor) = preprocessor else {
                    return Err(KernelError::CapabilityMissing {
                        capability_id: format!("input.preprocess.{processor_id}"),
                    });
                };
                parts.push(preprocessor.preprocess_part(&processor_id, part, output_modality)?);
            }
        }
    }

    rebuild_message_with_parts(message, parts)
}

fn part_from_preprocessed_output(
    part_id: &str,
    output_modality: AgentInputModality,
    output: &str,
) -> AgentPart {
    match output_modality {
        AgentInputModality::Json => AgentPart::json(format!("{part_id}.preprocessed"), output),
        AgentInputModality::Text => AgentPart::text(format!("{part_id}.preprocessed"), output),
        AgentInputModality::Image | AgentInputModality::Video => AgentPart::image_ref(
            format!("{part_id}.preprocessed"),
            output,
            "application/octet-stream",
        ),
        AgentInputModality::Audio | AgentInputModality::Music => AgentPart::audio_ref(
            format!("{part_id}.preprocessed"),
            output,
            "application/octet-stream",
        ),
        AgentInputModality::File | AgentInputModality::Binary => AgentPart::file_ref(
            format!("{part_id}.preprocessed"),
            output,
            "application/octet-stream",
        ),
        AgentInputModality::Artifact => {
            AgentPart::artifact_ref(format!("{part_id}.preprocessed"), output)
        }
    }
}

fn rebuild_message_with_parts(
    message: &AgentMessage,
    parts: Vec<AgentPart>,
) -> KernelResult<AgentMessage> {
    let mut rebuilt = AgentMessage::new(message.message_id.clone(), message.role, parts);
    if let Some(session_id) = &message.session_id {
        rebuilt = rebuilt.for_session(session_id.clone());
    }
    if let Some(task_id) = &message.task_id {
        rebuilt = rebuilt.for_task(task_id.clone());
    }
    if let Some(run_id) = &message.run_id {
        rebuilt = rebuilt.for_run(run_id.clone());
    }
    if let Some(step_id) = &message.step_id {
        rebuilt = rebuilt.for_step(step_id.clone());
    }
    if let Some(created_at) = &message.created_at {
        rebuilt = rebuilt.created_at(created_at.clone());
    }
    if let Some(trace_context) = &message.trace_context {
        rebuilt = rebuilt.with_trace_context(trace_context.clone());
    }
    for (key, value) in &message.metadata {
        rebuilt = rebuilt.with_metadata(key.clone(), value.clone());
    }
    if message.untrusted {
        rebuilt = rebuilt.mark_untrusted();
    }
    rebuilt.validate()?;
    Ok(rebuilt)
}
