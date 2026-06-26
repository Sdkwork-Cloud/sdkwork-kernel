use crate::ids;
use sdkwork_agent_plugin_core::{PluginResult, StandardPluginIds};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigProviderBindingSpec {
    pub binding_id: String,
    pub agent_id: String,
    pub provider_id: String,
    pub implementation_kind: String,
    pub configuration_profile_id: String,
    pub capabilities: Vec<String>,
    pub active: bool,
}

impl RigProviderBindingSpec {
    pub fn default_local(
        binding_id: impl Into<String>,
        configuration_profile_id: impl Into<String>,
    ) -> Self {
        Self::try_default_local(binding_id, configuration_profile_id)
            .expect("Rig provider binding ids and capabilities must be standard")
    }

    pub fn try_default_local(
        binding_id: impl Into<String>,
        configuration_profile_id: impl Into<String>,
    ) -> PluginResult<Self> {
        let spec = Self {
            binding_id: binding_id.into(),
            agent_id: ids::AGENT_ID.to_string(),
            provider_id: ids::MODEL_PROVIDER_ID.to_string(),
            implementation_kind: "typed-local-provider".to_string(),
            configuration_profile_id: configuration_profile_id.into(),
            capabilities: vec![
                "model.catalog".to_string(),
                "model.chat".to_string(),
                "tool.invoke".to_string(),
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
                "planning.create".to_string(),
            ],
            active: false,
        };
        spec.validate()
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = provider_id.into();
        self.validate()
            .expect("Rig provider binding provider id must be standard")
    }

    pub fn activate(mut self) -> Self {
        self.active = true;
        self
    }

    pub fn deactivate(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn validate(self) -> PluginResult<Self> {
        StandardPluginIds::validate_binding_id(&self.binding_id)?;
        StandardPluginIds::validate_agent_id(&self.agent_id)?;
        StandardPluginIds::validate_provider_id(&self.provider_id)?;
        StandardPluginIds::validate_profile_id(&self.configuration_profile_id)?;
        StandardPluginIds::validate_capabilities(&self.capabilities)?;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigDeploymentSpec {
    pub deployment_id: String,
    pub tenant_id: String,
    pub agent_id: String,
    pub binding_id: String,
    pub provider_id_snapshot: String,
    pub implementation_kind_snapshot: String,
    pub configuration_profile_id_snapshot: String,
    pub capabilities_snapshot: Vec<String>,
    pub created_at: String,
}

impl RigDeploymentSpec {
    pub fn from_binding(
        deployment_id: impl Into<String>,
        tenant_id: impl Into<String>,
        binding: &RigProviderBindingSpec,
        created_at: impl Into<String>,
    ) -> Self {
        Self::try_from_binding(deployment_id, tenant_id, binding, created_at)
            .expect("Rig deployment snapshot ids and capabilities must be standard")
    }

    pub fn try_from_binding(
        deployment_id: impl Into<String>,
        tenant_id: impl Into<String>,
        binding: &RigProviderBindingSpec,
        created_at: impl Into<String>,
    ) -> PluginResult<Self> {
        let deployment_id = deployment_id.into();
        StandardPluginIds::validate_deployment_id(&deployment_id)?;
        binding.clone().validate()?;

        Ok(Self {
            deployment_id,
            tenant_id: tenant_id.into(),
            agent_id: binding.agent_id.clone(),
            binding_id: binding.binding_id.clone(),
            provider_id_snapshot: binding.provider_id.clone(),
            implementation_kind_snapshot: binding.implementation_kind.clone(),
            configuration_profile_id_snapshot: binding.configuration_profile_id.clone(),
            capabilities_snapshot: binding.capabilities.clone(),
            created_at: created_at.into(),
        })
    }
}
