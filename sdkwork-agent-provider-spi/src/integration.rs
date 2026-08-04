//! Shared helpers for wiring external agent SDK bindings into adapters.
use crate::backend::SdkBackendKind;
use crate::binding::AgentSdkBindingManifest;
use crate::driver::StaticCapabilityDriver;
use crate::negotiation::{SdkCapabilityNegotiation, SdkNegotiationError};
use crate::registry::{BindingRegistry, DriverRegistry};
use std::sync::Arc;

/// Canonical provider binding ids: `binding.{engine}`.
///
/// The final segment carries the provider's product name, so it can differ
/// from the engine key by design: `gemini-cli` (Gemini CLI product), `mimo`
/// (Mimo), `rig-general` (Rig general agent) and `rig-rust` (Rig Rust
/// provider) keep their product names instead of the bare engine key. These
/// values are persisted and exposed through the Agents catalog SDK; changing
/// a segment requires a data migration and an SDK regeneration.
pub const CODEX_BINDING_ID: &str = "binding.codex";
pub const CLAUDE_CODE_BINDING_ID: &str = "binding.claude-code";
pub const GEMINI_CLI_BINDING_ID: &str = "binding.gemini-cli";
pub const HERMES_BINDING_ID: &str = "binding.hermes";
pub const MIMO_CODE_BINDING_ID: &str = "binding.mimo-code";
pub const OPENCLAW_BINDING_ID: &str = "binding.openclaw";
pub const OPENCODE_BINDING_ID: &str = "binding.opencode";
pub const RIG_BINDING_ID: &str = "binding.rig";

/// Registers one static healthy driver entry for every backend candidate in a manifest.
pub fn register_manifest_drivers(manifest: &AgentSdkBindingManifest, drivers: &mut DriverRegistry) {
    for capability in &manifest.capabilities {
        for backend in &capability.backends {
            if drivers.get(&backend.driver_id).is_some() {
                continue;
            }
            drivers.register(Arc::new(StaticCapabilityDriver::new(
                backend.driver_id.clone(),
                capability.capability_id.clone(),
                backend.kind,
            )));
        }
    }
}

/// Loads a binding manifest, registers static drivers, and negotiates capabilities.
pub fn bootstrap_binding(
    manifest: AgentSdkBindingManifest,
    drivers: &mut DriverRegistry,
    bindings: &mut BindingRegistry,
) -> Result<SdkCapabilityNegotiation, SdkNegotiationError> {
    let binding_id = manifest.binding_id.clone();
    register_manifest_drivers(&manifest, drivers);
    bindings.register(manifest);
    bindings.negotiate(&binding_id, drivers)
}

/// Resolved SDK integration state after successful binding negotiation.
#[derive(Debug, Clone)]
pub struct AgentSdkIntegration {
    pub negotiation: SdkCapabilityNegotiation,
}

impl AgentSdkIntegration {
    pub fn new(negotiation: SdkCapabilityNegotiation) -> Self {
        Self { negotiation }
    }

    pub fn agent_id(&self) -> &str {
        &self.negotiation.agent_id
    }

    pub fn binding_id(&self) -> &str {
        &self.negotiation.binding_id
    }

    pub fn selected_driver_id(&self, capability_id: &str) -> Option<&str> {
        self.negotiation
            .selected_driver(capability_id)
            .map(|entry| entry.driver_id.as_str())
    }

    pub fn selected_backend_kind(&self, capability_id: &str) -> Option<SdkBackendKind> {
        self.negotiation
            .selected_driver(capability_id)
            .map(|entry| entry.backend_kind)
    }
}
