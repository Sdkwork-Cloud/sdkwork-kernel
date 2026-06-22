use sdkwork_agent_plugin_core::StandardPluginIds;
use sdkwork_agent_plugin_rig::ids as rig_ids;

/// A hosted agent binding exposed by this kernel process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisteredAgent {
    pub agent_id: &'static str,
    pub runtime_agent_id: &'static str,
    pub default_model_provider_id: &'static str,
}

const HOSTED_AGENTS: &[RegisteredAgent] = &[RegisteredAgent {
    agent_id: rig_ids::AGENT_ID,
    runtime_agent_id: rig_ids::AGENT_ID,
    default_model_provider_id: rig_ids::MODEL_PROVIDER_ID,
}];

#[cfg(debug_assertions)]
const DEV_AGENT_ALIASES: &[(&str, &str)] = &[
    ("agent.1", rig_ids::AGENT_ID),
    ("agent.2", rig_ids::AGENT_ID),
    ("agent.intelligence.general", rig_ids::AGENT_ID),
];

/// Resolve a hosted agent binding when the id is registered on this kernel host.
pub fn resolve_registered_agent(agent_id: &str) -> Option<RegisteredAgent> {
    if StandardPluginIds::validate_agent_id(agent_id).is_err() {
        return None;
    }

    if let Some(found) = HOSTED_AGENTS
        .iter()
        .find(|entry| entry.agent_id == agent_id)
    {
        return Some(*found);
    }

    #[cfg(debug_assertions)]
    {
        if let Some((_, canonical)) = DEV_AGENT_ALIASES
            .iter()
            .find(|(alias, _)| *alias == agent_id)
        {
            return HOSTED_AGENTS
                .iter()
                .find(|entry| entry.agent_id == *canonical)
                .copied();
        }
    }

    None
}

/// Validate that an agent id is syntactically valid and hosted by this kernel.
pub fn validate_hosted_agent_id(agent_id: &str) -> Result<RegisteredAgent, String> {
    StandardPluginIds::validate_agent_id(agent_id)
        .map_err(|error| format!("invalid agent id: {error}"))?;
    resolve_registered_agent(agent_id).ok_or_else(|| {
        format!("agent id is not hosted by this kernel process: {agent_id}")
    })
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

    #[test]
    fn resolves_canonical_rig_agent() {
        let agent = validate_hosted_agent_id(rig_ids::AGENT_ID).expect("rig agent should resolve");
        assert_eq!(agent.default_model_provider_id, rig_ids::MODEL_PROVIDER_ID);
    }

    #[test]
    fn rejects_unknown_agent_id() {
        assert!(validate_hosted_agent_id("agent.intelligence.unknown").is_err());
    }

    #[test]
    fn dev_alias_resolves_in_debug_builds() {
        let agent = resolve_registered_agent("agent.1").expect("dev alias should resolve");
        assert_eq!(agent.agent_id, rig_ids::AGENT_ID);
    }
}
