use crate::ids;
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentExecutionAccessModeDescriptor,
    AgentExecutionApprovalBehavior, AgentExecutionNetworkAccess, AgentExecutionRiskLevel,
    AgentExecutionSettingsRequest, AgentExecutionSettingsResolution, AgentExecutionSettingsSpec,
    AgentExecutionWorkspaceAccess, KernelError, KernelResult, ProviderHealth,
};
use sdkwork_agent_plugin_core::ProcessAdapterConfigurationProvider;

pub const GEMINI_SDK_DEFAULT_ACCESS_MODE_ID: &str = "sdk_default";

#[derive(Debug, Clone)]
pub struct GeminiCliConfigurationProvider {
    base: ProcessAdapterConfigurationProvider,
}

impl GeminiCliConfigurationProvider {
    pub fn new() -> Self {
        Self {
            base: ProcessAdapterConfigurationProvider::new(ids::AGENT_ID),
        }
    }

    pub fn execution_spec() -> AgentExecutionSettingsSpec {
        AgentExecutionSettingsSpec::new(ids::AGENT_ID, GEMINI_SDK_DEFAULT_ACCESS_MODE_ID)
            .add_access_mode(AgentExecutionAccessModeDescriptor::new(
                GEMINI_SDK_DEFAULT_ACCESS_MODE_ID,
                "SDK default",
                "Use the execution policy implemented by the installed Gemini CLI SDK",
                AgentExecutionApprovalBehavior::ProviderDefault,
                AgentExecutionWorkspaceAccess::ProviderDefault,
                AgentExecutionNetworkAccess::ProviderDefault,
                AgentExecutionRiskLevel::Elevated,
            ))
    }
}

impl Default for GeminiCliConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfigurationProvider for GeminiCliConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        self.base.configuration_spec(agent_id)
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<AgentConfigurationValidation> {
        self.base.validate_configuration(configuration)
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
        Ok(AgentExecutionSettingsResolution::new(
            ids::AGENT_ID,
            &mode.mode_id,
        ))
    }

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}
