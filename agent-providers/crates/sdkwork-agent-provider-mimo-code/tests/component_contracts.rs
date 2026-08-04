use sdkwork_agent_kernel::{AgentPackageSource, ModelProvider, RuntimeBuilder};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_mimo_code::{
    mimo_code_agent_installer, mimo_code_kernel_plugin_manifest, mimo_code_package_manifest,
    mimo_code_provider_manifests, MiMoCodeKernelPlugin, MiMoCodeModelProvider,
    MIMO_CODE_SDK_PACKAGE, MIMO_CODE_SDK_VERSION,
};
use serde_json::Value;

const COMPONENT_SPEC: &str = include_str!("../specs/component.spec.json");

#[test]
fn component_spec_matches_crate_identity() {
    let spec: Value = serde_json::from_str(COMPONENT_SPEC).expect("valid component spec");
    assert_eq!(
        spec["component"]["name"],
        "sdkwork-agent-provider-mimo-code"
    );
    assert_eq!(spec["kind"], "sdkwork.component.spec");
}

#[test]
fn model_provider_manifest_uses_canonical_provider_id() {
    let provider = MiMoCodeModelProvider::new();
    assert_eq!(provider.provider_manifest().provider_id, "provider.mimo");
}

#[test]
fn kernel_plugin_exposes_the_standard_installation_surface() {
    // The TypeScript runtime for @mimo-ai/sdk falls back to the local
    // sdk_probe mock backend when the official package is not installed; that
    // fallback is fail-closed unless the explicit mock override is enabled.
    std::env::set_var("SDKWORK_KERNEL_ENVIRONMENT", "development");
    std::env::set_var("SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS", "1");

    let manifest = mimo_code_kernel_plugin_manifest();
    assert_eq!(manifest.plugin_id, "plugin.intelligence.mimo-code");
    assert!(manifest.supports_profile("agent-installation"));
    assert!(manifest
        .provider_ids
        .contains(&"provider.agent.installer.mimo-code".to_string()));

    let provider_ids = mimo_code_provider_manifests()
        .into_iter()
        .map(|provider| provider.provider_id)
        .collect::<Vec<_>>();
    assert!(provider_ids.contains(&"provider.agent.installer.mimo-code".to_string()));

    let plugin = MiMoCodeKernelPlugin::new();
    assert_eq!(plugin.agent_manifest().agent_id, "agent.mimo-code");
    let report = plugin
        .configure_runtime(RuntimeBuilder::new(
            "runtime.mimo-code.installer",
            plugin.agent_manifest(),
        ))
        .bootstrap()
        .expect("mimo-code runtime bootstraps");
    assert_eq!(
        report
            .runtime
            .agent_installer()
            .expect("typed installer")
            .provider_manifest()
            .provider_id,
        "provider.agent.installer.mimo-code"
    );
}

#[test]
fn installer_descriptor_uses_the_latest_exact_sdk_version() {
    let installer = mimo_code_agent_installer();
    assert_eq!(
        installer.provider_id(),
        "provider.agent.installer.mimo-code"
    );
    assert_eq!(installer.packages()[0].package_id, MIMO_CODE_SDK_PACKAGE);
    assert_eq!(installer.packages()[0].version, MIMO_CODE_SDK_VERSION);
    assert_eq!(
        mimo_code_package_manifest().source,
        AgentPackageSource::registry("npm", MIMO_CODE_SDK_PACKAGE, MIMO_CODE_SDK_VERSION)
    );
}
