use crate::{
    ids,
    materializer::{
        dematerialize_mimo_code_model_configuration, materialize_mimo_code_model_configuration,
        materialize_mimo_code_model_selection,
    },
};
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelResult, ProviderHealth,
};
use sdkwork_agent_plugin_core::ProcessAdapterConfigurationProvider;

#[derive(Debug, Clone)]
pub struct MiMoCodeConfigurationProvider {
    base: ProcessAdapterConfigurationProvider,
}

impl MiMoCodeConfigurationProvider {
    pub fn new() -> Self {
        Self {
            base: ProcessAdapterConfigurationProvider::with_model_configuration_scope(
                ids::AGENT_ID,
                "mimo_code",
            ),
        }
    }
}

impl Default for MiMoCodeConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfigurationProvider for MiMoCodeConfigurationProvider {
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
        materialize_mimo_code_model_configuration(request, application)
    }

    fn materialize_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        materialize_mimo_code_model_selection(request, application)
    }

    fn dematerialize_model_configuration(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<()> {
        dematerialize_mimo_code_model_configuration(agent_id, profile_id)
    }

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}
