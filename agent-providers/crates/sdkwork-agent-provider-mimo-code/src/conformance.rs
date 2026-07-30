use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn mimo_code_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("mimo-code-sdk-runtime")
        .require_profile("runtime-manifest")
        .require_profile("agent-installation")
        .require_profile("provider-model")
        .require_profile("provider-tool")
        .require_profile("security-baseline")
}
