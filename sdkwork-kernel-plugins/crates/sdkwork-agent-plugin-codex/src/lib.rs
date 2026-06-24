mod agent_definition;
mod conformance;
pub mod ids;
mod manifest;
mod package;

pub use agent_definition::{codex_agent_definition, codex_agent_manifest};
pub use conformance::codex_conformance_profile;
pub use manifest::{
    codex_kernel_plugin_manifest, codex_provider_manifests, CodexKernelPlugin,
};
pub use package::codex_package_manifest;

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_plugin_core::SdkworkKernelPlugin;

    #[test]
    fn plugin_manifest_declares_codex_agent_and_providers() {
        let manifest = codex_kernel_plugin_manifest();
        assert_eq!(manifest.plugin_id, ids::PLUGIN_ID);
        assert_eq!(manifest.agent_id.as_deref(), Some(ids::AGENT_ID));
        assert!(manifest
            .provider_ids
            .contains(&ids::MODEL_PROVIDER_ID.to_string()));
    }

    #[test]
    fn configure_runtime_registers_codex_model_provider() {
        let plugin = CodexKernelPlugin::new();
        let builder = sdkwork_agent_kernel::RuntimeBuilder::new(
            "runtime.test",
            plugin.agent_manifest(),
        );
        let report = plugin.configure_runtime(builder).bootstrap().expect("bootstrap");
        assert!(report
            .runtime
            .model_provider_ids()
            .contains(&ids::MODEL_PROVIDER_ID.to_string()));
    }
}
