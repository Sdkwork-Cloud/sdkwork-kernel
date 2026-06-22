use sdkwork_agent_kernel::{AgentRuntime, KernelResult, RuntimeBuilder};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_plugin_rig::RigKernelPlugin;
use sdkwork_agent_plugin_rig::ids;

/// Bootstrap the canonical local agent runtime using the Rig kernel plugin.
pub fn bootstrap_agent_runtime() -> KernelResult<AgentRuntime> {
    let plugin = RigKernelPlugin::fail_closed();
    let manifest = plugin.agent_manifest();
    if manifest.agent_id != ids::AGENT_ID {
        return Err(sdkwork_agent_kernel::KernelError::Validation {
            message: format!(
                "rig plugin agent_id mismatch: expected {}, got {}",
                ids::AGENT_ID, manifest.agent_id
            ),
        });
    }
    let builder = RuntimeBuilder::new("runtime.local", manifest)
        .with_agent_package_manifest(plugin.package_manifest())
        .with_security_profile("fail_closed=true");
    let report = plugin.configure_runtime(builder).bootstrap()?;
    Ok(report.runtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::RuntimeState;
    use sdkwork_agent_plugin_rig::ids;

    #[test]
    fn bootstrap_registers_rig_typed_providers() {
        let runtime = bootstrap_agent_runtime().expect("runtime should bootstrap");
        assert_eq!(runtime.state(), RuntimeState::Degraded);
        assert!(runtime.model_provider_ids().contains(&ids::MODEL_PROVIDER_ID.to_string()));
        assert!(runtime.tool_provider_ids().contains(&ids::TOOL_PROVIDER_ID.to_string()));
    }
}
