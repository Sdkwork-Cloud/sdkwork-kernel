use crate::{
    ids,
    materializer::{
        dematerialize_gemini_cli_model_configuration,
        materialize_gemini_cli_model_configuration, read_gemini_cli_model_configuration,
    },
};
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentExecutionAccessModeDescriptor,
    AgentExecutionApprovalBehavior, AgentExecutionNetworkAccess, AgentExecutionRiskLevel,
    AgentExecutionSettingsRequest, AgentExecutionSettingsResolution, AgentExecutionSettingsSpec,
    AgentExecutionWorkspaceAccess, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelError, KernelResult,
    ProviderHealth, ProviderModelConfigurationStatus,
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
            base: ProcessAdapterConfigurationProvider::with_model_configuration_scope(
                ids::AGENT_ID,
                "gemini_cli",
            ),
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
        self.base.apply_model_selection(request)
    }

    fn materialize_model_configuration(
        &self,
        request: &AgentModelConfigurationRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        // The Gemini CLI receives the model id per turn, so only the
        // configuration (relay base URL + API key) is materialized.
        materialize_gemini_cli_model_configuration(request, application)
    }

    fn dematerialize_model_configuration(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<()> {
        dematerialize_gemini_cli_model_configuration(agent_id, profile_id)
    }

    fn read_model_configuration(
        &self,
        _agent_id: &str,
        _profile_id: &str,
    ) -> KernelResult<ProviderModelConfigurationStatus> {
        read_gemini_cli_model_configuration()
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
