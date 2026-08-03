use crate::{
    ids,
    materializer::{
        dematerialize_claude_code_model_configuration,
        materialize_claude_code_model_configuration,
        materialize_claude_code_model_selection, read_claude_code_model_configuration,
    },
};
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentExecutionAccessModeDescriptor,
    AgentExecutionApprovalBehavior, AgentExecutionNetworkAccess, AgentExecutionProviderOption,
    AgentExecutionRiskLevel, AgentExecutionSettingsRequest, AgentExecutionSettingsResolution,
    AgentExecutionSettingsSpec, AgentExecutionWorkspaceAccess, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelError, KernelResult,
    ProviderHealth, ProviderModelConfigurationStatus,
};
use sdkwork_agent_plugin_core::ProcessAdapterConfigurationProvider;

pub const CLAUDE_DEFAULT_ACCESS_MODE_ID: &str = "default";
pub const CLAUDE_ACCEPT_EDITS_ACCESS_MODE_ID: &str = "accept_edits";
pub const CLAUDE_BYPASS_PERMISSIONS_ACCESS_MODE_ID: &str = "bypass_permissions";
const APPROVAL_POLICY_KEY: &str = "sdkwork.code_engine.approval_policy";
const SANDBOX_MODE_KEY: &str = "sdkwork.code_engine.sandbox_mode";

#[derive(Debug, Clone)]
pub struct ClaudeCodeConfigurationProvider {
    base: ProcessAdapterConfigurationProvider,
}

impl ClaudeCodeConfigurationProvider {
    pub fn new() -> Self {
        Self {
            base: ProcessAdapterConfigurationProvider::with_model_configuration_scope(
                ids::AGENT_ID,
                "claude_code",
            ),
        }
    }

    pub fn execution_spec() -> AgentExecutionSettingsSpec {
        AgentExecutionSettingsSpec::new(ids::AGENT_ID, CLAUDE_DEFAULT_ACCESS_MODE_ID)
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                CLAUDE_DEFAULT_ACCESS_MODE_ID,
                "Default permissions",
                "Ask before operations not allowed by Claude Code settings",
                AgentExecutionApprovalBehavior::UserReview,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Scoped,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                CLAUDE_ACCEPT_EDITS_ACCESS_MODE_ID,
                "Accept edits",
                "Allow routine file edits while retaining prompts for other risky operations",
                AgentExecutionApprovalBehavior::ProviderDefault,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Elevated,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                CLAUDE_BYPASS_PERMISSIONS_ACCESS_MODE_ID,
                "Bypass permissions",
                "Run without permission prompts when host policy allows it",
                AgentExecutionApprovalBehavior::Never,
                AgentExecutionWorkspaceAccess::FullAccess,
                AgentExecutionNetworkAccess::Enabled,
                AgentExecutionRiskLevel::Unrestricted,
            ))
    }
}

impl Default for ClaudeCodeConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfigurationProvider for ClaudeCodeConfigurationProvider {
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
        // The base apply builds and validates the profile; the wrapper is
        // responsible for materializing it into the CLI-native config surface
        // (the base's own materialize hook is a no-op).
        let application = self.base.apply_model_configuration(request)?;
        self.materialize_model_configuration(request, &application)?;
        Ok(application)
    }

    fn apply_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
    ) -> KernelResult<AgentModelConfigurationApplication> {
        let application = self.base.apply_model_selection(request)?;
        self.materialize_model_selection(request, &application)?;
        Ok(application)
    }

    fn materialize_model_configuration(
        &self,
        request: &AgentModelConfigurationRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        materialize_claude_code_model_configuration(request, application)
    }

    fn materialize_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        materialize_claude_code_model_selection(request, application)
    }

    fn dematerialize_model_configuration(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<()> {
        dematerialize_claude_code_model_configuration(agent_id, profile_id)
    }

    fn read_model_configuration(
        &self,
        _agent_id: &str,
        _profile_id: &str,
    ) -> KernelResult<ProviderModelConfigurationStatus> {
        read_claude_code_model_configuration()
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
        let mode = spec.resolve_access_mode(request.access_mode_id.as_deref())?;
        let resolution = AgentExecutionSettingsResolution::new(ids::AGENT_ID, &mode.mode_id);
        Ok(match mode.mode_id.as_str() {
            CLAUDE_DEFAULT_ACCESS_MODE_ID => resolution
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    "default",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    SANDBOX_MODE_KEY,
                    "workspace-write",
                )),
            CLAUDE_ACCEPT_EDITS_ACCESS_MODE_ID => resolution
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    "accept-edits",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    SANDBOX_MODE_KEY,
                    "workspace-write",
                )),
            CLAUDE_BYPASS_PERMISSIONS_ACCESS_MODE_ID => resolution
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    "bypass-permissions",
                ))
                .add_provider_option(AgentExecutionProviderOption::string(
                    SANDBOX_MODE_KEY,
                    "danger-full-access",
                )),
            _ => unreachable!("validated Claude Code access mode"),
        })
    }

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}
