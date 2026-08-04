//! Optional bridge to `sdkwork-models` catalog capability predicates.

use super::kind::{AgentInputModality, INPUT_MODALITY_MUSIC};
use super::resolve::{analyze_message_input, InputModalityCompatibility};
use crate::{AgentInputPolicy, AgentMessage, AgentPart, ModelDescriptor};
use sdkwork_models::{
    model_supports_audio_input, model_supports_image_input, model_supports_input_modality,
    model_supports_text_input, model_supports_video_input, ModelInfo,
};

pub fn catalog_model_supports_modality(model: &ModelInfo, modality: AgentInputModality) -> bool {
    match modality {
        AgentInputModality::Text => model_supports_text_input(model),
        AgentInputModality::Image => model_supports_image_input(model),
        AgentInputModality::Audio => model_supports_audio_input(model),
        AgentInputModality::Video => model_supports_video_input(model),
        AgentInputModality::Music => model_supports_input_modality(model, INPUT_MODALITY_MUSIC),
        AgentInputModality::Json
        | AgentInputModality::File
        | AgentInputModality::Binary
        | AgentInputModality::Artifact => model_supports_text_input(model),
    }
}

pub fn descriptor_from_catalog_model(model: &ModelInfo, provider_id: &str) -> ModelDescriptor {
    let mut descriptor = ModelDescriptor::new(
        &model.catalog_key,
        provider_id,
        &model.display_name,
        &model.family_code,
    )
    .with_version(&model.model_id);

    for modality in &model.input_modalities {
        descriptor = descriptor.with_input_mode(modality.as_str());
    }
    for modality in &model.output_modalities {
        descriptor = descriptor.with_output_mode(modality.as_str());
    }
    if model.supports_tools {
        descriptor = descriptor.with_capability("model.tool_call");
    }
    if model.supports_streaming {
        descriptor = descriptor.with_capability("model.streaming");
    }
    if model.supports_json_schema {
        descriptor = descriptor.with_capability("model.structured_output");
    }
    if model_supports_image_input(model)
        || model_supports_audio_input(model)
        || model_supports_video_input(model)
    {
        descriptor = descriptor.with_capability("model.multimodal_input");
    }
    descriptor
}

pub fn check_message_against_catalog_model(
    message: &AgentMessage,
    model: &ModelInfo,
) -> InputModalityCompatibility {
    let descriptor = descriptor_from_catalog_model(model, "provider.catalog");
    analyze_message_input(
        message,
        &AgentInputPolicy::multimodal_chat(),
        Some(&descriptor),
    )
}

pub fn part_matches_catalog_model(part: &AgentPart, model: &ModelInfo) -> bool {
    match part.input_modality() {
        Some(modality) => catalog_model_supports_modality(model, modality),
        None => true,
    }
}
