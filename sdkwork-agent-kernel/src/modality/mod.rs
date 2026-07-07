//! Agent input modality SPI — policy, resolution, and optional catalog bridge.
//!
//! Vocabulary is aligned with `sdkwork-models` `input_modalities` and public
//! agent-card `input_modes`. Callers should use this module instead of ad hoc
//! MIME or provider-specific checks.

pub mod content_ref;
pub mod contract;
pub mod delivery;
mod kind;
mod policy;
mod resolve;
pub mod rpc_ingress;
pub mod text;

#[cfg(feature = "sdkwork-models")]
pub mod catalog;

pub use content_ref::{ContentReference, ContentReferenceScheme};
pub use contract::{
    AgentInputContract, AgentInteractionContract, AgentOutputContract, ModalitySlot,
    ModelDeliveryStrategy,
};
pub use delivery::{
    apply_delivery_transforms, enforce_slot_constraints, InputModalityPreprocessor,
    SkillInputModalityPreprocessor,
};
pub use kind::{
    infer_modality_from_mime_type, parse_input_modalities, AgentInputModality, CARD_INPUT_MODES,
    INPUT_MODALITIES, INPUT_MODALITY_ARTIFACT, INPUT_MODALITY_AUDIO, INPUT_MODALITY_BINARY,
    INPUT_MODALITY_FILE, INPUT_MODALITY_IMAGE, INPUT_MODALITY_JSON, INPUT_MODALITY_MUSIC,
    INPUT_MODALITY_TEXT, INPUT_MODALITY_VIDEO,
};
pub use policy::{AgentInputPolicy, UnsupportedInputModalityAction};
pub use resolve::{
    analyze_message_input, check_message_against_model_descriptor, part_kind_to_input_modality,
    resolve_model_input, resolve_model_input_with_options, validate_message_against_input_policy,
    validate_structured_model_input, validate_structured_model_input_with_options,
    InputModalityCompatibility, InputModalityPartReport, ModelInputResolution,
    ModelInputResolveOptions,
};
pub use rpc_ingress::parse_chat_rpc_payload;
pub use text::{
    agent_messages_from_text_lines, agent_messages_to_text_lines, flatten_message_to_text,
};
