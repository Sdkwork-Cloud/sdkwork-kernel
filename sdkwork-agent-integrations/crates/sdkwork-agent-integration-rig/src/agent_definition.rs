use sdkwork_agent_kernel::{AgentManifest, CapabilityRequirement};

use crate::ids;

pub fn rig_agent_manifest() -> AgentManifest {
    AgentManifest {
        schema_version: "0.1.0".to_string(),
        manifest_type: "agent".to_string(),
        agent_id: ids::AGENT_ID.to_string(),
        name: "rig-general-agent".to_string(),
        display_name: "Rig General Agent".to_string(),
        description: "SDKWork-standard Rig-backed general agent runtime.".to_string(),
        version: "0.1.0".to_string(),
        domain: "intelligence".to_string(),
        required_capabilities: vec![
            "model.catalog".to_string(),
            "model.chat".to_string(),
            "policy.evaluate".to_string(),
            "agent.install".to_string(),
            "agent.configure".to_string(),
        ],
        optional_capabilities: vec![
            "model.streaming".to_string(),
            "model.tool_call".to_string(),
            "tool.invoke".to_string(),
            "planning.create".to_string(),
        ],
        required_capability_requirements: vec![
            CapabilityRequirement::new("model.catalog").with_min_version("0.1.0"),
            CapabilityRequirement::new("model.chat").with_min_version("0.1.0"),
            CapabilityRequirement::new("policy.evaluate").with_min_version("0.1.0"),
            CapabilityRequirement::new("agent.install").with_min_version("0.1.0"),
            CapabilityRequirement::new("agent.configure").with_min_version("0.1.0"),
        ],
        optional_capability_requirements: vec![
            CapabilityRequirement::new("model.streaming").with_min_version("0.1.0"),
            CapabilityRequirement::new("model.tool_call").with_min_version("0.1.0"),
            CapabilityRequirement::new("tool.invoke").with_min_version("0.1.0"),
            CapabilityRequirement::new("planning.create").with_min_version("0.1.0"),
        ],
        event_families: vec![
            "agent.runtime.*".to_string(),
            "agent.model.*".to_string(),
            "agent.tool.*".to_string(),
            "agent.policy.*".to_string(),
            "agent.install.*".to_string(),
            "agent.configure.*".to_string(),
        ],
        owner_name: "sdkwork-platform".to_string(),
        status: "candidate".to_string(),
    }
}
