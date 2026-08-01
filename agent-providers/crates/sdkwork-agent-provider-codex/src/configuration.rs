use crate::ids;
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentExecutionAccessModeDescriptor,
    AgentExecutionApprovalBehavior, AgentExecutionNetworkAccess, AgentExecutionProviderOption,
    AgentExecutionRiskLevel, AgentExecutionSettingsRequest, AgentExecutionSettingsResolution,
    AgentExecutionSettingsSpec, AgentExecutionWorkspaceAccess, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelError, KernelResult,
    ProviderHealth, APPROVE_FOR_ME_ACCESS_MODE_ID, ASK_FOR_APPROVAL_ACCESS_MODE_ID,
    FULL_ACCESS_MODE_ID,
};
use sdkwork_agent_plugin_core::ProcessAdapterConfigurationProvider;

const APPROVAL_POLICY_KEY: &str = "sdkwork.code_engine.approval_policy";
const APPROVALS_REVIEWER_KEY: &str = "sdkwork.code_engine.approvals_reviewer";
const SANDBOX_MODE_KEY: &str = "sdkwork.code_engine.sandbox_mode";

#[derive(Debug, Clone)]
pub struct CodexConfigurationProvider {
    base: ProcessAdapterConfigurationProvider,
}

impl CodexConfigurationProvider {
    pub fn new() -> Self {
        Self {
            base: ProcessAdapterConfigurationProvider::with_model_configuration_scope(
                ids::AGENT_ID,
                "codex",
            ),
        }
    }

    pub fn execution_spec() -> AgentExecutionSettingsSpec {
        AgentExecutionSettingsSpec::new(ids::AGENT_ID, ASK_FOR_APPROVAL_ACCESS_MODE_ID)
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                ASK_FOR_APPROVAL_ACCESS_MODE_ID,
                "Ask for approval",
                "Ask before editing outside the workspace or using the network",
                AgentExecutionApprovalBehavior::UserReview,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Scoped,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                APPROVE_FOR_ME_ACCESS_MODE_ID,
                "Approve for me",
                "Automatically review risky operations without expanding access boundaries",
                AgentExecutionApprovalBehavior::AutomaticReview,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Elevated,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                FULL_ACCESS_MODE_ID,
                "Full access",
                "Access any file and the network without approval prompts",
                AgentExecutionApprovalBehavior::Never,
                AgentExecutionWorkspaceAccess::FullAccess,
                AgentExecutionNetworkAccess::Enabled,
                AgentExecutionRiskLevel::Unrestricted,
            ))
    }
}

impl Default for CodexConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfigurationProvider for CodexConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        self.base.configuration_spec(agent_id)
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<AgentConfigurationValidation> {
        self.base.validate_configuration(configuration)
    }

    fn apply_model_configuration(
        &self,
        request: &AgentModelConfigurationRequest,
    ) -> KernelResult<AgentModelConfigurationApplication> {
        self.base.apply_model_configuration(request)
    }

    fn apply_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
    ) -> KernelResult<AgentModelConfigurationApplication> {
        self.base.apply_model_selection(request)
    }

    fn execution_settings_spec(&self, agent_id: &str) -> KernelResult<AgentExecutionSettingsSpec> {
        if agent_id != ids::AGENT_ID {
            return Err(KernelError::CapabilityMissing {
                capability_id: agent_id.to_string(),
            });
        }
        let spec = Self::execution_spec();
        spec.validate()?;
        Ok(spec)
    }

    fn resolve_execution_settings(
        &self,
        request: &AgentExecutionSettingsRequest,
    ) -> KernelResult<AgentExecutionSettingsResolution> {
        let spec = self.execution_settings_spec(&request.agent_id)?;
        let access_mode = spec.resolve_access_mode(request.access_mode_id.as_deref())?;
        let resolution =
            AgentExecutionSettingsResolution::new(ids::AGENT_ID, access_mode.mode_id.as_str());
        Ok(match access_mode.mode_id.as_str() {
            ASK_FOR_APPROVAL_ACCESS_MODE_ID => resolution
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    "on-request",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVALS_REVIEWER_KEY,
                    "user",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    SANDBOX_MODE_KEY,
                    "workspace-write",
                )),
            APPROVE_FOR_ME_ACCESS_MODE_ID => resolution
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    "on-request",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVALS_REVIEWER_KEY,
                    "auto_review",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    SANDBOX_MODE_KEY,
                    "workspace-write",
                )),
            FULL_ACCESS_MODE_ID => resolution
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    "never",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    SANDBOX_MODE_KEY,
                    "danger-full-access",
                )),
            _ => unreachable!("validated Codex access mode"),
        })
    }

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::AgentExecutionProviderOptionValue;

    #[test]
    fn approve_for_me_changes_reviewer_without_expanding_boundaries() {
        let resolved = CodexConfigurationProvider::new()
            .resolve_execution_settings(
                &AgentExecutionSettingsRequest::new(ids::AGENT_ID)
                    .with_access_mode(APPROVE_FOR_ME_ACCESS_MODE_ID),
            )
            .expect("resolve mode");

        assert_eq!(
            resolved.provider_option(APPROVALS_REVIEWER_KEY),
            Some(&AgentExecutionProviderOptionValue::String(
                "auto_review".to_string()
            ))
        );
        assert_eq!(
            resolved.provider_option(SANDBOX_MODE_KEY),
            Some(&AgentExecutionProviderOptionValue::String(
                "workspace-write".to_string()
            ))
        );
    }

    #[test]
    fn full_access_disables_approvals_and_sandboxing() {
        let resolved = CodexConfigurationProvider::new()
            .resolve_execution_settings(
                &AgentExecutionSettingsRequest::new(ids::AGENT_ID)
                    .with_access_mode(FULL_ACCESS_MODE_ID),
            )
            .expect("resolve mode");

        assert_eq!(
            resolved.provider_option(APPROVAL_POLICY_KEY),
            Some(&AgentExecutionProviderOptionValue::String(
                "never".to_string()
            ))
        );
        assert_eq!(
            resolved.provider_option(SANDBOX_MODE_KEY),
            Some(&AgentExecutionProviderOptionValue::String(
                "danger-full-access".to_string()
            ))
        );
    }
}
