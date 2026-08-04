use sdkwork_agent_plugin_core::StandardPluginIds;
use sdkwork_agent_provider_claude_code::ids as claude_code_ids;
use sdkwork_agent_provider_codex::ids as codex_ids;
use sdkwork_agent_provider_gemini_cli::ids as gemini_cli_ids;
use sdkwork_agent_provider_hermes::ids as hermes_ids;
use sdkwork_agent_provider_openclaw::ids as openclaw_ids;
use sdkwork_agent_provider_opencode::ids as opencode_ids;
use sdkwork_agent_provider_rig::ids as rig_ids;

use crate::runtime_bootstrap::{kernel_agent_plugin_kind_from_env, KernelAgentPluginKind};

/// A hosted agent binding exposed by this kernel process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisteredAgent {
    pub agent_id: &'static str,
    pub runtime_agent_id: &'static str,
    pub default_model_provider_id: &'static str,
}

/// Returns the single hosted agent binding for the active kernel plugin.
pub fn active_hosted_agent() -> RegisteredAgent {
    match kernel_agent_plugin_kind_from_env() {
        KernelAgentPluginKind::Rig => RegisteredAgent {
            agent_id: rig_ids::AGENT_ID,
            runtime_agent_id: rig_ids::AGENT_ID,
            default_model_provider_id: rig_ids::MODEL_PROVIDER_ID,
        },
        KernelAgentPluginKind::OpenClaw => RegisteredAgent {
            agent_id: openclaw_ids::AGENT_ID,
            runtime_agent_id: openclaw_ids::AGENT_ID,
            default_model_provider_id: openclaw_ids::MODEL_PROVIDER_ID,
        },
        KernelAgentPluginKind::Hermes => RegisteredAgent {
            agent_id: hermes_ids::AGENT_ID,
            runtime_agent_id: hermes_ids::AGENT_ID,
            default_model_provider_id: hermes_ids::MODEL_PROVIDER_ID,
        },
        KernelAgentPluginKind::Codex => RegisteredAgent {
            agent_id: codex_ids::AGENT_ID,
            runtime_agent_id: codex_ids::AGENT_ID,
            default_model_provider_id: codex_ids::MODEL_PROVIDER_ID,
        },
        KernelAgentPluginKind::ClaudeCode => RegisteredAgent {
            agent_id: claude_code_ids::AGENT_ID,
            runtime_agent_id: claude_code_ids::AGENT_ID,
            default_model_provider_id: claude_code_ids::MODEL_PROVIDER_ID,
        },
        KernelAgentPluginKind::OpenCode => RegisteredAgent {
            agent_id: opencode_ids::AGENT_ID,
            runtime_agent_id: opencode_ids::AGENT_ID,
            default_model_provider_id: opencode_ids::MODEL_PROVIDER_ID,
        },
        KernelAgentPluginKind::GeminiCli => RegisteredAgent {
            agent_id: gemini_cli_ids::AGENT_ID,
            runtime_agent_id: gemini_cli_ids::AGENT_ID,
            default_model_provider_id: gemini_cli_ids::MODEL_PROVIDER_ID,
        },
    }
}

#[cfg(debug_assertions)]
fn dev_agent_alias_canonical(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "agent.1" | "agent.2" | "agent.general" => {
            Some(active_hosted_agent().agent_id)
        }
        _ => None,
    }
}

/// Resolve a hosted agent binding when the id is registered on this kernel host.
pub fn resolve_registered_agent(agent_id: &str) -> Option<RegisteredAgent> {
    if StandardPluginIds::validate_agent_id(agent_id).is_err() {
        return None;
    }

    let hosted = active_hosted_agent();
    if agent_id == hosted.agent_id {
        return Some(hosted);
    }

    #[cfg(debug_assertions)]
    if dev_agent_alias_canonical(agent_id).is_some() {
        return Some(hosted);
    }

    None
}

/// Validate that an agent id is syntactically valid and hosted by this kernel.
pub fn validate_hosted_agent_id(agent_id: &str) -> Result<RegisteredAgent, String> {
    StandardPluginIds::validate_agent_id(agent_id)
        .map_err(|error| format!("invalid agent id: {error}"))?;
    resolve_registered_agent(agent_id)
        .ok_or_else(|| format!("agent id is not hosted by this kernel process: {agent_id}"))
}

/// Stamp default provider metadata for a hosted agent when the caller omitted it.
pub fn apply_hosted_agent_defaults(
    metadata: &mut std::collections::HashMap<String, String>,
    registered: RegisteredAgent,
) {
    metadata
        .entry("modelProvider".to_string())
        .or_insert_with(|| registered.default_model_provider_id.to_string());
    metadata
        .entry("hostedRuntimeAgentId".to_string())
        .or_insert_with(|| registered.runtime_agent_id.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_bootstrap::KERNEL_AGENT_PLUGIN_ENV;
    use crate::testing::env::{lock, VarGuard};

    #[test]
    fn resolves_canonical_rig_agent_by_default() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, None);
        let agent = validate_hosted_agent_id(rig_ids::AGENT_ID).expect("rig agent should resolve");
        assert_eq!(agent.default_model_provider_id, rig_ids::MODEL_PROVIDER_ID);
    }

    #[test]
    fn resolves_openclaw_agent_when_plugin_env_set() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("openclaw"));
        let agent = validate_hosted_agent_id(openclaw_ids::AGENT_ID)
            .expect("openclaw agent should resolve");
        assert_eq!(
            agent.default_model_provider_id,
            openclaw_ids::MODEL_PROVIDER_ID
        );
        assert!(validate_hosted_agent_id(rig_ids::AGENT_ID).is_err());
    }

    #[test]
    fn resolves_hermes_agent_when_plugin_env_set() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("hermes"));
        let agent =
            validate_hosted_agent_id(hermes_ids::AGENT_ID).expect("hermes agent should resolve");
        assert_eq!(
            agent.default_model_provider_id,
            hermes_ids::MODEL_PROVIDER_ID
        );
    }

    #[test]
    fn resolves_codex_agent_when_plugin_env_set() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("codex"));
        let agent =
            validate_hosted_agent_id(codex_ids::AGENT_ID).expect("codex agent should resolve");
        assert_eq!(
            agent.default_model_provider_id,
            codex_ids::MODEL_PROVIDER_ID
        );
    }

    #[test]
    fn resolves_new_sdk_backed_agents_when_plugin_env_set() {
        for (plugin, agent_id, model_provider_id) in [
            (
                "claude-code",
                claude_code_ids::AGENT_ID,
                claude_code_ids::MODEL_PROVIDER_ID,
            ),
            (
                "opencode",
                opencode_ids::AGENT_ID,
                opencode_ids::MODEL_PROVIDER_ID,
            ),
            (
                "gemini-cli",
                gemini_cli_ids::AGENT_ID,
                gemini_cli_ids::MODEL_PROVIDER_ID,
            ),
        ] {
            let _lock = lock();
            let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some(plugin));
            let agent = validate_hosted_agent_id(agent_id).expect("agent should resolve");
            assert_eq!(agent.default_model_provider_id, model_provider_id);
        }
    }

    #[test]
    fn rejects_unknown_agent_id() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, None);
        assert!(validate_hosted_agent_id("agent.unknown").is_err());
    }

    #[test]
    fn dev_alias_resolves_in_debug_builds() {
        let _lock = lock();
        let _plugin = VarGuard::set(KERNEL_AGENT_PLUGIN_ENV, Some("codex"));
        let agent = resolve_registered_agent("agent.1").expect("dev alias should resolve");
        assert_eq!(agent.agent_id, codex_ids::AGENT_ID);
    }
}
