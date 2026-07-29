use crate::{KernelError, KernelResult};

pub const ASK_FOR_APPROVAL_ACCESS_MODE_ID: &str = "ask_for_approval";
pub const APPROVE_FOR_ME_ACCESS_MODE_ID: &str = "approve_for_me";
pub const FULL_ACCESS_MODE_ID: &str = "full_access";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionApprovalBehavior {
    UserReview,
    AutomaticReview,
    Never,
    ProviderDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionWorkspaceAccess {
    ReadOnly,
    WorkspaceWrite,
    FullAccess,
    ProviderDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionNetworkAccess {
    Restricted,
    Enabled,
    ProviderDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentExecutionRiskLevel {
    Scoped,
    Elevated,
    Unrestricted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionAccessModeDescriptor {
    pub mode_id: String,
    pub display_name: String,
    pub description: String,
    pub approval_behavior: AgentExecutionApprovalBehavior,
    pub workspace_access: AgentExecutionWorkspaceAccess,
    pub network_access: AgentExecutionNetworkAccess,
    pub risk_level: AgentExecutionRiskLevel,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

impl AgentExecutionAccessModeDescriptor {
    pub fn new(
        mode_id: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        approval_behavior: AgentExecutionApprovalBehavior,
        workspace_access: AgentExecutionWorkspaceAccess,
        network_access: AgentExecutionNetworkAccess,
        risk_level: AgentExecutionRiskLevel,
    ) -> Self {
        Self {
            mode_id: mode_id.into(),
            display_name: display_name.into(),
            description: description.into(),
            approval_behavior,
            workspace_access,
            network_access,
            risk_level,
            enabled: true,
            disabled_reason: None,
        }
    }

    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.enabled = false;
        self.disabled_reason = Some(reason.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionSettingsSpec {
    pub schema_version: String,
    pub agent_id: String,
    pub default_access_mode_id: String,
    pub access_modes: Vec<AgentExecutionAccessModeDescriptor>,
}

impl AgentExecutionSettingsSpec {
    pub fn new(agent_id: impl Into<String>, default_access_mode_id: impl Into<String>) -> Self {
        Self {
            schema_version: "0.1.0".to_string(),
            agent_id: agent_id.into(),
            default_access_mode_id: default_access_mode_id.into(),
            access_modes: Vec::new(),
        }
    }

    pub fn add_access_mode(mut self, access_mode: AgentExecutionAccessModeDescriptor) -> Self {
        self.access_modes.push(access_mode);
        self
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.agent_id.trim().is_empty() {
            return Err(KernelError::validation(
                "execution settings spec agent_id must not be blank",
            ));
        }
        if self.access_modes.is_empty() {
            return Err(KernelError::validation(
                "execution settings spec must declare at least one access mode",
            ));
        }

        let mut mode_ids = std::collections::BTreeSet::new();
        for access_mode in &self.access_modes {
            if access_mode.mode_id.trim().is_empty() {
                return Err(KernelError::validation(
                    "execution access mode id must not be blank",
                ));
            }
            if !mode_ids.insert(access_mode.mode_id.as_str()) {
                return Err(KernelError::validation(format!(
                    "duplicate execution access mode id: {}",
                    access_mode.mode_id
                )));
            }
            if !access_mode.enabled && access_mode.disabled_reason.is_none() {
                return Err(KernelError::validation(format!(
                    "disabled execution access mode requires a reason: {}",
                    access_mode.mode_id
                )));
            }
        }

        let Some(default_mode) = self
            .access_modes
            .iter()
            .find(|access_mode| access_mode.mode_id == self.default_access_mode_id)
        else {
            return Err(KernelError::validation(
                "default execution access mode is not declared",
            ));
        };
        if !default_mode.enabled {
            return Err(KernelError::validation(
                "default execution access mode must be enabled",
            ));
        }
        Ok(())
    }

    pub fn resolve_access_mode(
        &self,
        requested_mode_id: Option<&str>,
    ) -> KernelResult<&AgentExecutionAccessModeDescriptor> {
        self.validate()?;
        let mode_id = requested_mode_id
            .map(str::trim)
            .filter(|mode_id| !mode_id.is_empty())
            .unwrap_or(self.default_access_mode_id.as_str());
        let access_mode = self
            .access_modes
            .iter()
            .find(|access_mode| access_mode.mode_id == mode_id)
            .ok_or_else(|| {
                KernelError::validation(format!(
                    "unsupported execution access mode for {}: {mode_id}",
                    self.agent_id
                ))
            })?;
        if !access_mode.enabled {
            return Err(KernelError::validation(
                access_mode
                    .disabled_reason
                    .clone()
                    .unwrap_or_else(|| format!("execution access mode is disabled: {mode_id}")),
            ));
        }
        Ok(access_mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionSettingsRequest {
    pub agent_id: String,
    pub access_mode_id: Option<String>,
}

impl AgentExecutionSettingsRequest {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            access_mode_id: None,
        }
    }

    pub fn with_access_mode(mut self, access_mode_id: impl Into<String>) -> Self {
        self.access_mode_id = Some(access_mode_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentExecutionProviderOptionValue {
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionProviderOption {
    pub key: String,
    pub value: AgentExecutionProviderOptionValue,
}

impl AgentExecutionProviderOption {
    pub fn string(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: AgentExecutionProviderOptionValue::String(value.into()),
        }
    }

    pub fn boolean(key: impl Into<String>, value: bool) -> Self {
        Self {
            key: key.into(),
            value: AgentExecutionProviderOptionValue::Boolean(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionSettingsResolution {
    pub agent_id: String,
    pub access_mode_id: String,
    pub provider_options: Vec<AgentExecutionProviderOption>,
}

impl AgentExecutionSettingsResolution {
    pub fn new(agent_id: impl Into<String>, access_mode_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            access_mode_id: access_mode_id.into(),
            provider_options: Vec::new(),
        }
    }

    pub fn add_provider_option(mut self, option: AgentExecutionProviderOption) -> Self {
        self.provider_options
            .retain(|entry| entry.key != option.key);
        self.provider_options.push(option);
        self
    }

    pub fn provider_option(&self, key: &str) -> Option<&AgentExecutionProviderOptionValue> {
        self.provider_options
            .iter()
            .find(|option| option.key == key)
            .map(|option| &option.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> AgentExecutionSettingsSpec {
        AgentExecutionSettingsSpec::new("agent.test", ASK_FOR_APPROVAL_ACCESS_MODE_ID)
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                ASK_FOR_APPROVAL_ACCESS_MODE_ID,
                "Ask for approval",
                "Ask before risky work",
                AgentExecutionApprovalBehavior::UserReview,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Scoped,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                FULL_ACCESS_MODE_ID,
                "Full access",
                "Run without approval prompts",
                AgentExecutionApprovalBehavior::Never,
                AgentExecutionWorkspaceAccess::FullAccess,
                AgentExecutionNetworkAccess::Enabled,
                AgentExecutionRiskLevel::Unrestricted,
            ))
    }

    #[test]
    fn validates_and_resolves_default_access_mode() {
        let spec = sample_spec();
        assert!(spec.validate().is_ok());
        assert_eq!(
            spec.resolve_access_mode(None)
                .expect("default mode")
                .mode_id,
            ASK_FOR_APPROVAL_ACCESS_MODE_ID
        );
    }

    #[test]
    fn rejects_disabled_and_unknown_access_modes() {
        let disabled = AgentExecutionSettingsSpec::new("agent.test", "enabled")
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                "enabled",
                "Enabled",
                "Enabled",
                AgentExecutionApprovalBehavior::ProviderDefault,
                AgentExecutionWorkspaceAccess::ProviderDefault,
                AgentExecutionNetworkAccess::ProviderDefault,
                AgentExecutionRiskLevel::Scoped,
            ))
            .add_access_mode(
                AgentExecutionAccessModeDescriptor::new(
                    "disabled",
                    "Disabled",
                    "Disabled",
                    AgentExecutionApprovalBehavior::ProviderDefault,
                    AgentExecutionWorkspaceAccess::ProviderDefault,
                    AgentExecutionNetworkAccess::ProviderDefault,
                    AgentExecutionRiskLevel::Scoped,
                )
                .disabled("blocked by policy"),
            );

        assert!(disabled.resolve_access_mode(Some("disabled")).is_err());
        assert!(disabled.resolve_access_mode(Some("unknown")).is_err());
    }
}
