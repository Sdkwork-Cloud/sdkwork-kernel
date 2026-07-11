//! Agent provider integration SPI for external agent framework adaptation.
//!
//! See `specs/AGENT_PROVIDER_INTEGRATION_SPEC.md` and `specs/AGENT_PROVIDER_BINDING_SPEC.md`.

mod backend;
mod binding;
mod bridge;
mod capability;
mod driver;
mod integration;
mod mapping;
mod negotiation;
mod registry;
mod runtime;
mod selector;

pub use backend::{
    default_backend_priority, default_transport_priority, ProviderTransportKind, SdkBackendKind,
};
pub use binding::{
    AgentSdkBindingManifest, BackendCandidate, CapabilityBinding, CapabilityExecutionScope,
    IntegrationSource, LanguagePackages, ManifestStatus, NpmPackageRef, PythonPackageRef,
    RustPackageRef, SelectionPolicy,
};
pub use bridge::{
    model_response_from_runtime, tool_result_from_runtime, wire_runtime_providers,
    RuntimeBackedProviders, SdkRuntimeBackedModelProvider, SdkRuntimeBackedToolProvider,
    SDK_CAPABILITY_MODEL_CHAT, SDK_CAPABILITY_SESSION_LIFECYCLE, SDK_CAPABILITY_SKILL_INVOKE,
    SDK_CAPABILITY_TOOL_INVOKE,
};
pub use capability::{
    describe_capability, SdkCapabilityDescriptor, SdkCapabilityId, STANDARD_SDK_CAPABILITIES,
};
pub use driver::{
    AgentSdkCapabilityDriver, SdkDriverHealth, SdkDriverStatus, StaticCapabilityDriver,
};
pub use integration::{
    bootstrap_binding, register_manifest_drivers, AgentSdkIntegration, CLAUDE_CODE_BINDING_ID,
    CODEX_BINDING_ID, GEMINI_CLI_BINDING_ID, HERMES_BINDING_ID, OPENCLAW_BINDING_ID,
    OPENCODE_BINDING_ID, RIG_BINDING_ID,
};
pub use mapping::{
    AgentRuntimeAdapter, ConversationManager, MessageAdapter, ModelAdapter, PolicyAdapter,
    SessionAdapter, SessionConfig, SessionLifecycleProvider, StreamAdapter, ToolAdapter,
};
pub use negotiation::{NegotiatedCapability, SdkCapabilityNegotiation, SdkNegotiationError};
pub use registry::{BindingRegistry, DriverRegistry, RegisteredBinding};
pub use runtime::{
    ProviderTransportRouter, SdkBackendRuntime, SdkRuntimeError, SdkRuntimeExecutionOptions,
    SdkRuntimeOperation, SdkRuntimeOperationKind, SdkRuntimeRequest, SdkRuntimeResponse,
    SdkRuntimeRouter,
};
pub use selector::{select_backend, BackendSelection};
