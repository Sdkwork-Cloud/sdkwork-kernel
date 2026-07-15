use sdkwork_agent_plugin_core::KernelPluginConformanceProfile;

pub fn hermes_conformance_profile() -> KernelPluginConformanceProfile {
    KernelPluginConformanceProfile::new("hermes-sdk-runtime")
        .require_profile("runtime-manifest")
        .require_profile("provider-model")
        .require_profile("security-baseline")
}
