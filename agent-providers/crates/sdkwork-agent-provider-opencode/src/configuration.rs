use crate::{
    ids,
    materializer::{
        dematerialize_opencode_model_configuration, materialize_opencode_model_configuration,
        materialize_opencode_model_selection,
    },
};
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentExecutionAccessModeDescriptor,
    AgentExecutionApprovalBehavior, AgentExecutionNetworkAccess, AgentExecutionProviderOption,
    AgentExecutionRiskLevel, AgentExecutionSettingsRequest, AgentExecutionSettingsResolution,
    AgentExecutionSettingsSpec, AgentExecutionWorkspaceAccess, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelError, KernelResult,
    ProviderHealth,
};
use sdkwork_agent_plugin_core::ProcessAdapterConfigurationProvider;

pub const OPENCODE_ASK_ACCESS_MODE_ID: &str = "ask";
pub const OPENCODE_ALLOW_EDITS_ACCESS_MODE_ID: &str = "allow_edits";
pub const OPENCODE_ALLOW_ALL_ACCESS_MODE_ID: &str = "allow_all";
const APPROVAL_POLICY_KEY: &str = "sdkwork.code_engine.approval_policy";

#[derive(Debug, Clone)]
pub struct OpenCodeConfigurationProvider {
    base: ProcessAdapterConfigurationProvider,
}

impl OpenCodeConfigurationProvider {
    pub fn new() -> Self {
        Self {
            base: ProcessAdapterConfigurationProvider::with_model_configuration_scope(
                ids::AGENT_ID,
                "opencode",
            ),
        }
    }

    pub fn execution_spec() -> AgentExecutionSettingsSpec {
        AgentExecutionSettingsSpec::new(ids::AGENT_ID, OPENCODE_ASK_ACCESS_MODE_ID)
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                OPENCODE_ASK_ACCESS_MODE_ID,
                "Ask for permission",
                "Ask before tools that require permission",
                AgentExecutionApprovalBehavior::UserReview,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Scoped,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                OPENCODE_ALLOW_EDITS_ACCESS_MODE_ID,
                "Allow edits",
                "Allow workspace edits while keeping other risky tools gated",
                AgentExecutionApprovalBehavior::ProviderDefault,
                AgentExecutionWorkspaceAccess::WorkspaceWrite,
                AgentExecutionNetworkAccess::Restricted,
                AgentExecutionRiskLevel::Elevated,
            ))
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                OPENCODE_ALLOW_ALL_ACCESS_MODE_ID,
                "Allow all",
                "Allow all tools without permission prompts",
                AgentExecutionApprovalBehavior::Never,
                AgentExecutionWorkspaceAccess::FullAccess,
                AgentExecutionNetworkAccess::Enabled,
                AgentExecutionRiskLevel::Unrestricted,
            ))
    }
}

impl Default for OpenCodeConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfigurationProvider for OpenCodeConfigurationProvider {
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

    fn materialize_model_configuration(
        &self,
        request: &AgentModelConfigurationRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        materialize_opencode_model_configuration(request, application)
    }

    fn materialize_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        materialize_opencode_model_selection(request, application)
    }

    fn dematerialize_model_configuration(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<()> {
        dematerialize_opencode_model_configuration(agent_id, profile_id)
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
        Ok(
            AgentExecutionSettingsResolution::new(ids::AGENT_ID, &mode.mode_id)
                .add_provider_option(AgentExecutionProviderOption::string(
                    APPROVAL_POLICY_KEY,
                    match mode.mode_id.as_str() {
                        OPENCODE_ASK_ACCESS_MODE_ID => "ask",
                        OPENCODE_ALLOW_EDITS_ACCESS_MODE_ID => "allow-edits",
                        OPENCODE_ALLOW_ALL_ACCESS_MODE_ID => "allow-all",
                        _ => unreachable!("validated OpenCode access mode"),
                    },
                )),
        )
    }

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}
