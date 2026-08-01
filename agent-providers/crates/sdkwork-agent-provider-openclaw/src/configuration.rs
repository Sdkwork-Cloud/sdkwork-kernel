use crate::ids;
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelResult, ProviderHealth,
};
use sdkwork_agent_plugin_core::ProcessAdapterConfigurationProvider;

#[derive(Debug, Clone)]
pub struct OpenClawConfigurationProvider {
    base: ProcessAdapterConfigurationProvider,
}

impl OpenClawConfigurationProvider {
    pub fn new() -> Self {
        Self {
            base: ProcessAdapterConfigurationProvider::with_model_configuration_scope(
                ids::AGENT_ID,
                "openclaw",
            ),
        }
    }
}

impl Default for OpenClawConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfigurationProvider for OpenClawConfigurationProvider {
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

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}
