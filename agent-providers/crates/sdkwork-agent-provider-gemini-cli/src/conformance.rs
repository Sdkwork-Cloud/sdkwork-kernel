use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn gemini_cli_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("gemini-cli-sdk-runtime")
        .require_profile("runtime-manifest")
        .require_profile("provider-model")
        .require_profile("security-baseline")
}
