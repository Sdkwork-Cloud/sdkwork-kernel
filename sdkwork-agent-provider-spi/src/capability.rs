use serde::{Deserialize, Serialize};

/// Standard SDK capability identifier (`sdk.<domain>.<operation>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SdkCapabilityId(pub String);

impl SdkCapabilityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SdkCapabilityId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Declarative description of one SDK capability family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdkCapabilityDescriptor {
    pub capability_id: &'static str,
    pub display_name: &'static str,
    pub kernel_spi_family: &'static str,
}

/// Canonical SDK capability catalog aligned with `AGENT_SDK_SPI_SPEC.md`.
pub const STANDARD_SDK_CAPABILITIES: &[SdkCapabilityDescriptor] = &[
    SdkCapabilityDescriptor {
        capability_id: "sdk.session.lifecycle",
        display_name: "Session lifecycle",
        kernel_spi_family: "session.lifecycle",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.session.history",
        display_name: "Session history",
        kernel_spi_family: "conversation",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.session.control",
        display_name: "Session control",
        kernel_spi_family: "session.control",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.model.chat",
        display_name: "Model chat",
        kernel_spi_family: "model",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.model.stream",
        display_name: "Model stream",
        kernel_spi_family: "model.stream",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.tool.discover",
        display_name: "Tool discovery",
        kernel_spi_family: "tool",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.tool.invoke",
        display_name: "Tool invocation",
        kernel_spi_family: "tool",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.skill.discover",
        display_name: "Skill discovery",
        kernel_spi_family: "skill",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.skill.invoke",
        display_name: "Skill invocation",
        kernel_spi_family: "skill",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.policy.approval",
        display_name: "Policy approval",
        kernel_spi_family: "policy",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.mcp.tools",
        display_name: "MCP tools",
        kernel_spi_family: "mcp",
    },
    SdkCapabilityDescriptor {
        capability_id: "sdk.agent.delegate",
        display_name: "Agent delegation",
        kernel_spi_family: "collaboration",
    },
];

/// Returns the standard descriptor for a capability id when present.
pub fn describe_capability(capability_id: &str) -> Option<&'static SdkCapabilityDescriptor> {
    STANDARD_SDK_CAPABILITIES
        .iter()
        .find(|entry| entry.capability_id == capability_id)
}
