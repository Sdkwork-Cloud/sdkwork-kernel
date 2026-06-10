use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigSectionKind, AgentConfigValue,
    AgentConfigValueKind, AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, KernelEventRedaction, KernelResult, ProviderHealth,
};

use crate::ids;

#[derive(Debug, Clone, Default)]
pub struct RigConfigurationProvider;

impl RigConfigurationProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn spec() -> AgentConfigurationSpec {
        AgentConfigurationSpec::new(ids::AGENT_ID)
            .add_section(
                AgentConfigSection::base("base", "Base").add_field(
                    AgentConfigField::text("agent.display_name", "Display name").required(),
                ),
            )
            .add_section(
                AgentConfigSection::llm_api_key("llm", "LLM")
                    .add_field(AgentConfigField::text("llm.rig.provider_id", "Provider").required())
                    .add_field(AgentConfigField::llm_api_key(
                        "llm.rig.api_key",
                        "Rig API key",
                    )),
            )
            .add_section(
                AgentConfigSection::new("runtime", "Runtime", AgentConfigSectionKind::Runtime)
                    .add_field(
                        AgentConfigField::text("runtime.rig.backend_mode", "Backend mode")
                            .required()
                            .with_default(AgentConfigValue::string("fail_closed")),
                    ),
            )
            .add_section(
                AgentConfigSection::new("security", "Security", AgentConfigSectionKind::Security)
                    .add_field(
                        AgentConfigField::new(
                            "security.fail_closed",
                            "Fail closed",
                            AgentConfigValueKind::String,
                        )
                        .required()
                        .with_default(AgentConfigValue::string("true"))
                        .with_redaction(KernelEventRedaction::Internal),
                    ),
            )
    }
}

impl AgentConfigurationProvider for RigConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        if agent_id != ids::AGENT_ID {
            return Err(sdkwork_agent_kernel::KernelError::CapabilityMissing {
                capability_id: agent_id.to_string(),
            });
        }
        Ok(Self::spec())
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<AgentConfigurationValidation> {
        Ok(Self::spec().validate(configuration))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
