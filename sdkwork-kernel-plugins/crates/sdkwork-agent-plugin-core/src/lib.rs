use sdkwork_agent_kernel::{
    AgentDefinition, AgentManifest, AgentPackageManifest, ProviderManifest, RuntimeBuilder,
};
use std::collections::HashSet;

pub type PluginResult<T> = Result<T, String>;

pub struct StandardPluginIds;

impl StandardPluginIds {
    pub fn validate_plugin_id(value: &str) -> PluginResult<()> {
        validate_standard_id(value, "pluginId", Some("plugin."))
    }

    pub fn validate_agent_id(value: &str) -> PluginResult<()> {
        validate_standard_id(value, "agentId", Some("agent."))
    }

    pub fn validate_provider_id(value: &str) -> PluginResult<()> {
        validate_standard_id(value, "providerId", Some("provider."))
    }

    pub fn validate_provider_or_adapter_id(value: &str) -> PluginResult<()> {
        if value.starts_with("adapter.") {
            return validate_standard_id(value, "providerId", Some("adapter."));
        }
        Self::validate_provider_id(value)
    }

    pub fn validate_binding_id(value: &str) -> PluginResult<()> {
        validate_standard_id(value, "bindingId", Some("binding."))
    }

    pub fn validate_profile_id(value: &str) -> PluginResult<()> {
        validate_standard_id(value, "configurationProfileId", Some("profile."))
    }

    pub fn validate_deployment_id(value: &str) -> PluginResult<()> {
        validate_standard_id(value, "deploymentId", Some("deployment."))
    }

    pub fn validate_capability_id(value: &str) -> PluginResult<()> {
        if value.trim().is_empty() {
            return Err("capabilities must not contain empty capability ids".to_string());
        }
        if value.chars().count() > 128 {
            return Err("capabilities capability ids must be at most 128 characters".to_string());
        }
        if value.trim() != value
            || !value.chars().all(is_standard_id_character)
            || !has_non_empty_dot_segments(value)
        {
            return Err("capabilities must use lowercase namespaced capability ids".to_string());
        }
        Ok(())
    }

    pub fn validate_capabilities(values: &[String]) -> PluginResult<()> {
        let mut seen = HashSet::new();
        for capability in values {
            Self::validate_capability_id(capability)?;
            if !seen.insert(capability.as_str()) {
                return Err(format!(
                    "capabilities must not contain duplicate capability id: {capability}"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelPluginManifest {
    pub plugin_id: String,
    pub display_name: String,
    pub version: String,
    pub implementation_kind: String,
    pub source_reference: Option<String>,
    pub agent_id: Option<String>,
    pub provider_ids: Vec<String>,
    pub supported_profiles: Vec<String>,
}

impl KernelPluginManifest {
    pub fn new(
        plugin_id: impl Into<String>,
        display_name: impl Into<String>,
        version: impl Into<String>,
        implementation_kind: impl Into<String>,
    ) -> Self {
        Self::try_new(plugin_id, display_name, version, implementation_kind)
            .expect("plugin id must be standard")
    }

    pub fn try_new(
        plugin_id: impl Into<String>,
        display_name: impl Into<String>,
        version: impl Into<String>,
        implementation_kind: impl Into<String>,
    ) -> PluginResult<Self> {
        let manifest = Self {
            plugin_id: plugin_id.into(),
            display_name: display_name.into(),
            version: version.into(),
            implementation_kind: implementation_kind.into(),
            source_reference: None,
            agent_id: None,
            provider_ids: Vec::new(),
            supported_profiles: Vec::new(),
        };
        manifest.validate()
    }

    pub fn with_source_reference(mut self, source_reference: impl Into<String>) -> Self {
        self.source_reference = Some(source_reference.into());
        self
    }

    pub fn with_agent_id(self, agent_id: impl Into<String>) -> Self {
        self.try_with_agent_id(agent_id)
            .expect("plugin agent id must be standard")
    }

    pub fn try_with_agent_id(mut self, agent_id: impl Into<String>) -> PluginResult<Self> {
        self.agent_id = Some(agent_id.into());
        self.validate()
    }

    pub fn with_provider_id(self, provider_id: impl Into<String>) -> Self {
        self.try_with_provider_id(provider_id)
            .expect("plugin provider id must be standard")
    }

    pub fn try_with_provider_id(mut self, provider_id: impl Into<String>) -> PluginResult<Self> {
        self.provider_ids.push(provider_id.into());
        self.validate()
    }

    pub fn with_supported_profile(mut self, supported_profile: impl Into<String>) -> Self {
        self.supported_profiles.push(supported_profile.into());
        self
    }

    pub fn supports_profile(&self, profile: &str) -> bool {
        self.supported_profiles
            .iter()
            .any(|supported| supported == profile)
    }

    pub fn validate(self) -> PluginResult<Self> {
        StandardPluginIds::validate_plugin_id(&self.plugin_id)?;
        if let Some(agent_id) = self.agent_id.as_deref() {
            StandardPluginIds::validate_agent_id(agent_id)?;
        }

        let mut seen_provider_ids = HashSet::new();
        for provider_id in &self.provider_ids {
            StandardPluginIds::validate_provider_or_adapter_id(provider_id)?;
            if !seen_provider_ids.insert(provider_id.as_str()) {
                return Err(format!("duplicate providerId: {provider_id}"));
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelProviderBinding {
    pub binding_id: String,
    pub agent_id: String,
    pub provider_id: String,
    pub implementation_kind: String,
    pub configuration_profile_id: String,
    pub capabilities: Vec<String>,
    pub active: bool,
}

impl KernelProviderBinding {
    pub fn new(
        binding_id: impl Into<String>,
        agent_id: impl Into<String>,
        provider_id: impl Into<String>,
        implementation_kind: impl Into<String>,
        configuration_profile_id: impl Into<String>,
    ) -> Self {
        Self::try_new(
            binding_id,
            agent_id,
            provider_id,
            implementation_kind,
            configuration_profile_id,
        )
        .expect("plugin provider binding ids must be standard")
    }

    pub fn try_new(
        binding_id: impl Into<String>,
        agent_id: impl Into<String>,
        provider_id: impl Into<String>,
        implementation_kind: impl Into<String>,
        configuration_profile_id: impl Into<String>,
    ) -> PluginResult<Self> {
        let record = Self {
            binding_id: binding_id.into(),
            agent_id: agent_id.into(),
            provider_id: provider_id.into(),
            implementation_kind: implementation_kind.into(),
            configuration_profile_id: configuration_profile_id.into(),
            capabilities: Vec::new(),
            active: false,
        };
        record.validate()
    }

    pub fn with_provider_id(self, provider_id: impl Into<String>) -> Self {
        self.try_with_provider_id(provider_id)
            .expect("plugin provider id must be standard")
    }

    pub fn try_with_provider_id(mut self, provider_id: impl Into<String>) -> PluginResult<Self> {
        self.provider_id = provider_id.into();
        self.validate()
    }

    pub fn with_capability(self, capability: impl Into<String>) -> Self {
        self.try_with_capability(capability)
            .expect("plugin capability id must be standard")
    }

    pub fn try_with_capability(mut self, capability: impl Into<String>) -> PluginResult<Self> {
        self.capabilities.push(capability.into());
        self.validate()
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
pub struct KernelPluginDeploymentSnapshot {
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

impl KernelPluginDeploymentSnapshot {
    pub fn from_binding(
        deployment_id: impl Into<String>,
        tenant_id: impl Into<String>,
        binding: &KernelProviderBinding,
        created_at: impl Into<String>,
    ) -> Self {
        Self::try_from_binding(deployment_id, tenant_id, binding, created_at)
            .expect("plugin deployment snapshot ids must be standard")
    }

    pub fn try_from_binding(
        deployment_id: impl Into<String>,
        tenant_id: impl Into<String>,
        binding: &KernelProviderBinding,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelPluginConformanceProfile {
    pub profile_id: String,
    pub required_profiles: Vec<String>,
}

impl KernelPluginConformanceProfile {
    pub fn new(profile_id: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            required_profiles: Vec::new(),
        }
    }

    pub fn require_profile(mut self, profile: impl Into<String>) -> Self {
        self.required_profiles.push(profile.into());
        self
    }

    pub fn requires(&self, profile: &str) -> bool {
        self.required_profiles
            .iter()
            .any(|required| required == profile)
    }
}

pub trait SdkworkKernelPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest;

    fn agent_manifest(&self) -> AgentManifest;

    fn agent_definition(&self) -> AgentDefinition {
        let manifest = self.agent_manifest();
        let definition_id = format!(
            "definition.{}",
            manifest.agent_id.trim_start_matches("agent.")
        );
        AgentDefinition::new(definition_id, manifest)
    }

    fn package_manifest(&self) -> AgentPackageManifest;

    fn provider_manifests(&self) -> Vec<ProviderManifest>;

    fn configure_runtime(&self, builder: RuntimeBuilder) -> RuntimeBuilder;

    fn conformance_profile(&self) -> KernelPluginConformanceProfile;
}

pub trait SdkworkKernelFoundationPlugin {
    fn plugin_manifest(&self) -> KernelPluginManifest;

    fn provider_manifests(&self) -> Vec<ProviderManifest>;

    fn conformance_profile(&self) -> KernelPluginConformanceProfile;
}

mod policy;
mod process_adapter;
mod local_plugin;
pub use policy::SdkStandardPolicyProvider;
pub use process_adapter::{ProcessAdapterConfigurationProvider, ProcessAdapterInstaller};
pub use local_plugin::{
    LocalPluginCatalog, LocalPluginDescriptor, LocalPluginDiscoveryRequest,
    LocalPluginLoadError, LocalPluginLoadErrorKind, LocalPluginProvider,
    LocalPluginSkillDescriptor, LocalPluginSource, LocalPluginStatus,
};

fn validate_standard_id(
    value: &str,
    field_name: &str,
    required_prefix: Option<&str>,
) -> PluginResult<()> {
    if value.trim().is_empty() {
        return Err(format!("{field_name} is required"));
    }
    if value.trim() != value {
        return Err(format!(
            "{field_name} must not contain leading or trailing whitespace"
        ));
    }
    if value.chars().count() > 128 {
        return Err(format!("{field_name} must be at most 128 characters"));
    }
    if !value.chars().all(is_standard_id_character) {
        return Err(format!(
            "{field_name} must use lowercase standard id characters"
        ));
    }
    if !has_non_empty_dot_segments(value) {
        return Err(format!(
            "{field_name} must use non-empty dot-delimited segments"
        ));
    }
    if let Some(prefix) = required_prefix {
        if !value.starts_with(prefix) {
            return Err(format!("{field_name} must start with {prefix}"));
        }
    }
    Ok(())
}

fn has_non_empty_dot_segments(value: &str) -> bool {
    let mut segment_count = 0;
    for segment in value.split('.') {
        segment_count += 1;
        if segment.is_empty() {
            return false;
        }
    }
    segment_count >= 2
}

fn is_standard_id_character(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_id_validation_rejects_empty_segments() {
        assert!(StandardPluginIds::validate_provider_id("provider.model.rig-rust").is_ok());
        assert!(StandardPluginIds::validate_provider_id("provider.").is_err());
        assert!(StandardPluginIds::validate_provider_id("provider..rig").is_err());
        assert!(StandardPluginIds::validate_provider_id("Provider.Model").is_err());
    }

    #[test]
    fn capability_validation_rejects_duplicates_and_empty_segments() {
        assert!(StandardPluginIds::validate_capabilities(&[
            "model.chat".to_string(),
            "tool.invoke".to_string(),
        ])
        .is_ok());
        assert!(StandardPluginIds::validate_capability_id("model.").is_err());
        assert!(
            StandardPluginIds::validate_capability_id(&format!("model.{}", "a".repeat(123)))
                .is_err()
        );
        assert!(StandardPluginIds::validate_capabilities(&[
            "model.chat".to_string(),
            "model.chat".to_string(),
        ])
        .is_err());
    }
}
