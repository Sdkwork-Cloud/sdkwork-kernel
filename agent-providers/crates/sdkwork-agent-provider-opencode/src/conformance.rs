use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn opencode_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("opencode-sdk-runtime")
        .require_profile("runtime-manifest")
        .require_profile("provider-model")
        .require_profile("provider-tool")
        .require_profile("security-baseline")
}
