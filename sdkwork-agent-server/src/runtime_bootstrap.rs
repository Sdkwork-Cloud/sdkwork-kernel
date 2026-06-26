use sdkwork_agent_kernel::{AgentRuntime, KernelError, KernelResult, RuntimeBuilder};
use sdkwork_agent_plugin_core::SdkworkKernelPlugin;
use sdkwork_agent_provider_codex::{CodexKernelPlugin, ids as codex_ids};
use sdkwork_agent_provider_hermes::{HermesKernelPlugin, ids as hermes_ids};
use sdkwork_agent_provider_openclaw::{OpenClawKernelPlugin, ids as openclaw_ids};
use sdkwork_agent_provider_rig::{RigKernelPlugin, ids as rig_ids};

/// Environment variable selecting the active kernel agent plugin (`rig`, `openclaw`, `hermes`, `codex`).
pub const KERNEL_AGENT_PLUGIN_ENV: &str = "SDKWORK_KERNEL_AGENT_PLUGIN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAgentPluginKind {
    Rig,
    OpenClaw,
    Hermes,
    Codex,
}

pub fn kernel_agent_plugin_kind_from_env() -> KernelAgentPluginKind {
    std::env::var(KERNEL_AGENT_PLUGIN_ENV)
        .ok()
        .and_then(|value| parse_kernel_agent_plugin_kind(&value))
        .unwrap_or(KernelAgentPluginKind::Rig)
}

pub fn parse_kernel_agent_plugin_kind(value: &str) -> Option<KernelAgentPluginKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "rig" => Some(KernelAgentPluginKind::Rig),
        "openclaw" | "open-claw" => Some(KernelAgentPluginKind::OpenClaw),
        "hermes" | "hermes-agent" => Some(KernelAgentPluginKind::Hermes),
        "codex" | "openai-codex" => Some(KernelAgentPluginKind::Codex),
        _ => None,
    }
}

/// Bootstrap the local agent runtime using the selected kernel plugin.
pub fn bootstrap_agent_runtime() -> KernelResult<AgentRuntime> {
    match kernel_agent_plugin_kind_from_env() {
        KernelAgentPluginKind::Rig => bootstrap_rig_runtime(),
        KernelAgentPluginKind::OpenClaw => bootstrap_openclaw_runtime(),
        KernelAgentPluginKind::Hermes => bootstrap_hermes_runtime(),
        KernelAgentPluginKind::Codex => bootstrap_codex_runtime(),
    }
}

fn bootstrap_rig_runtime() -> KernelResult<AgentRuntime> {
    bootstrap_plugin_runtime(
        &RigKernelPlugin::fail_closed(),
        rig_ids::AGENT_ID,
        "rig",
    )
}

fn bootstrap_openclaw_runtime() -> KernelResult<AgentRuntime> {
    bootstrap_plugin_runtime(
        &OpenClawKernelPlugin::new(),
        openclaw_ids::AGENT_ID,
        "openclaw",
    )
}

fn bootstrap_hermes_runtime() -> KernelResult<AgentRuntime> {
    bootstrap_plugin_runtime(
        &HermesKernelPlugin::new(),
        hermes_ids::AGENT_ID,
        "hermes",
    )
}

fn bootstrap_codex_runtime() -> KernelResult<AgentRuntime> {
    bootstrap_plugin_runtime(
        &CodexKernelPlugin::new(),
        codex_ids::AGENT_ID,
        "codex",
    )
}

fn bootstrap_plugin_runtime(
    plugin: &dyn SdkworkKernelPlugin,
    expected_agent_id: &str,
    plugin_name: &str,
) -> KernelResult<AgentRuntime> {
    let manifest = plugin.agent_manifest();
    if manifest.agent_id != expected_agent_id {
        return Err(KernelError::Validation {
            message: format!(
                "{plugin_name} plugin agent_id mismatch: expected {expected_agent_id}, got {}",
                manifest.agent_id
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
    use crate::testing::env::{lock, VarGuard};
    use sdkwork_agent_kernel::RuntimeState;

    #[test]
    fn parse_kernel_agent_plugin_kind_accepts_aliases() {
        assert_eq!(
            parse_kernel_agent_plugin_kind("openclaw"),
            Some(KernelAgentPluginKind::OpenClaw)
        );
        assert_eq!(
            parse_kernel_agent_plugin_kind("hermes-agent"),
            Some(KernelAgentPluginKind::Hermes)
        );
        assert_eq!(
            parse_kernel_agent_plugin_kind("openai-codex"),
            Some(KernelAgentPluginKind::Codex)
        );
        assert_eq!(parse_kernel_agent_plugin_kind("unknown"), None);
    }

    #[test]
    fn bootstrap_registers_rig_typed_providers_by_default() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, None);
        let runtime = bootstrap_agent_runtime().expect("runtime should bootstrap");
        assert_eq!(runtime.state(), RuntimeState::Degraded);
        assert!(runtime
            .model_provider_ids()
            .contains(&rig_ids::MODEL_PROVIDER_ID.to_string()));
    }

    #[test]
    fn bootstrap_selects_openclaw_plugin_from_env() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("openclaw"));
        let runtime = bootstrap_agent_runtime().expect("openclaw runtime should bootstrap");
        assert!(runtime
            .model_provider_ids()
            .contains(&openclaw_ids::MODEL_PROVIDER_ID.to_string()));
    }

    #[test]
    fn bootstrap_selects_hermes_plugin_from_env() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("hermes"));
        let runtime = bootstrap_agent_runtime().expect("hermes runtime should bootstrap");
        assert!(runtime
            .model_provider_ids()
            .contains(&hermes_ids::MODEL_PROVIDER_ID.to_string()));
    }

    #[test]
    fn bootstrap_selects_codex_plugin_from_env() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("codex"));
        let runtime = bootstrap_agent_runtime().expect("codex runtime should bootstrap");
        assert!(runtime
            .model_provider_ids()
            .contains(&codex_ids::MODEL_PROVIDER_ID.to_string()));
    }
}
