use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn openclaw_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("openclaw-sdk-runtime")
        .require_profile("runtime-manifest")
        .require_profile("agent-installation")
        .require_profile("provider-model")
        .require_profile("security-baseline")
}
