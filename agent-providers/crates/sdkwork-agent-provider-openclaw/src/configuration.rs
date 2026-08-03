use crate::{
    ids,
    materializer::{
        dematerialize_openclaw_model_configuration, materialize_openclaw_model_configuration,
        materialize_openclaw_model_selection, read_openclaw_model_configuration,
    },
};
use sdkwork_agent_kernel::{
    AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentModelConfigurationApplication,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, KernelResult, ProviderHealth,
    ProviderModelConfigurationStatus,
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
        materialize_openclaw_model_configuration(request, application)
    }

    fn materialize_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
        application: &AgentModelConfigurationApplication,
    ) -> KernelResult<()> {
        materialize_openclaw_model_selection(request, application)
    }

    fn dematerialize_model_configuration(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> KernelResult<()> {
        dematerialize_openclaw_model_configuration(agent_id, profile_id)
    }

    fn read_model_configuration(
        &self,
        _agent_id: &str,
        _profile_id: &str,
    ) -> KernelResult<ProviderModelConfigurationStatus> {
        read_openclaw_model_configuration()
    }

    fn health(&self) -> ProviderHealth {
        self.base.health()
    }
}
