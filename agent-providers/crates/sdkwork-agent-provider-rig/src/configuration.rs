use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigSectionKind, AgentConfigValue,
    AgentConfigValueKind, AgentConfiguration, AgentConfigurationProfile,
    AgentConfigurationProvider, AgentConfigurationSpec, AgentConfigurationValidation,
    AgentModelConfigurationApplication, AgentModelConfigurationRequest, AgentModelSelectionRequest,
    AgentSecretBinding, KernelError, KernelEventRedaction, KernelResult, ProviderHealth,
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
                    .add_field(AgentConfigField::text("llm.rig.provider_id", "Provider"))
                    .add_field(AgentConfigField::llm_api_key(
                        "llm.rig.api_key",
                        "Rig API key",
                    ))
                    .add_field(AgentConfigField::text(
                        "llm.rig.base_url",
                        "Model API base URL",
                    ))
                    .add_field(AgentConfigField::text(
                        "llm.rig.default_model",
                        "Default model",
                    ))
                    .add_field(AgentConfigField::new(
                        "llm.rig.supported_models",
                        "Supported models",
                        AgentConfigValueKind::StringList,
                    ))
                    .add_field(AgentConfigField::new(
                        "llm.rig.input_context_tokens",
                        "Input context tokens",
                        AgentConfigValueKind::Integer,
                    ))
                    .add_field(AgentConfigField::new(
                        "llm.rig.output_context_tokens",
                        "Output context tokens",
                        AgentConfigValueKind::Integer,
                    ))
                    .add_field(AgentConfigField::new(
                        "llm.rig.tool_call_rounds",
                        "Tool call rounds",
                        AgentConfigValueKind::Integer,
                    ))
                    .add_field(
                        AgentConfigField::new(
                            "llm.rig.supports_multimodal",
                            "Supports multimodal input",
                            AgentConfigValueKind::Boolean,
                        )
                        .with_default(AgentConfigValue::boolean(false)),
                    ),
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

    fn apply_model_configuration(
        &self,
        request: &AgentModelConfigurationRequest,
    ) -> KernelResult<AgentModelConfigurationApplication> {
        if request.agent_id != ids::AGENT_ID {
            return Err(KernelError::CapabilityMissing {
                capability_id: request.agent_id.clone(),
            });
        }
        request.validate()?;

        let mut configuration = AgentConfiguration::new(&request.agent_id, &request.profile_id)
            .set("agent.display_name", AgentConfigValue::string("Rig"))
            .set(
                "llm.rig.provider_id",
                AgentConfigValue::string(request.vendor_code.trim()),
            )
            .set(
                "llm.rig.api_key",
                AgentConfigValue::secret_ref(request.api_key_secret_ref.trim()),
            )
            .set(
                "llm.rig.base_url",
                AgentConfigValue::string(request.base_url.trim()),
            )
            .set(
                "llm.rig.default_model",
                AgentConfigValue::string(request.default_model_id.trim()),
            )
            .set(
                "llm.rig.supported_models",
                AgentConfigValue::string_list(
                    request
                        .supported_model_ids
                        .iter()
                        .map(|model_id| model_id.trim().to_string())
                        .collect(),
                ),
            )
            .set(
                "llm.rig.supports_multimodal",
                AgentConfigValue::boolean(request.supports_multimodal),
            )
            .set("runtime.rig.backend_mode", AgentConfigValue::string("live"))
            .set("security.fail_closed", AgentConfigValue::string("true"));
        for (key, value) in [
            ("llm.rig.input_context_tokens", request.input_context_tokens),
            (
                "llm.rig.output_context_tokens",
                request.output_context_tokens,
            ),
            ("llm.rig.tool_call_rounds", request.tool_call_rounds),
        ] {
            if let Some(value) = value {
                configuration = configuration.set(key, AgentConfigValue::integer(value));
            }
        }

        let profile = AgentConfigurationProfile::new(
            &request.profile_id,
            &request.agent_id,
            "0.2.0",
            configuration,
        )
        .add_secret_binding(AgentSecretBinding::llm_api_key(
            "llm.rig.api_key",
            request.vendor_code.trim(),
            request.api_key_secret_ref.trim(),
        ))
        .activate();
        if !profile.validate_against(&Self::spec()).is_valid() {
            return Err(KernelError::validation(
                "Rig model configuration does not satisfy the configuration schema",
            ));
        }
        Ok(AgentModelConfigurationApplication::new(
            &request.request_id,
            "rig",
            profile,
        ))
    }

    fn apply_model_selection(
        &self,
        request: &AgentModelSelectionRequest,
    ) -> KernelResult<AgentModelConfigurationApplication> {
        if request.agent_id != ids::AGENT_ID {
            return Err(KernelError::CapabilityMissing {
                capability_id: request.agent_id.clone(),
            });
        }
        request.validate()?;

        let (mut configuration, configuration_version, secret_bindings) =
            if let Some(profile) = &request.current_profile {
                (
                    profile.configuration.clone(),
                    profile.configuration_version.clone(),
                    profile.secret_bindings.clone(),
                )
            } else {
                let provider_default_secret_ref = "provider-default.rig.model.api_key";
                (
                    AgentConfiguration::new(&request.agent_id, &request.profile_id)
                        .set("agent.display_name", AgentConfigValue::string("Rig"))
                        .set(
                            "llm.rig.api_key",
                            AgentConfigValue::secret_ref(provider_default_secret_ref),
                        )
                        .set("runtime.rig.backend_mode", AgentConfigValue::string("live"))
                        .set("security.fail_closed", AgentConfigValue::string("true")),
                    "0.2.0".to_string(),
                    vec![AgentSecretBinding::llm_api_key(
                        "llm.rig.api_key",
                        "rig",
                        provider_default_secret_ref,
                    )],
                )
            };
        let mut supported_model_ids = match configuration.value("llm.rig.supported_models") {
            Some(AgentConfigValue::StringList(model_ids)) => model_ids.clone(),
            Some(_) => {
                return Err(KernelError::validation(
                    "Rig supported model configuration has an invalid value kind",
                ))
            }
            None => Vec::new(),
        };
        let selected_model_id = request.model_id.trim();
        let model_is_supported = supported_model_ids
            .iter()
            .any(|model_id| model_id.trim() == selected_model_id);
        if request.enforce_supported_models && !model_is_supported {
            return Err(KernelError::validation(
                "selected model is not included in the configured supported models",
            ));
        }
        if !model_is_supported {
            supported_model_ids.push(selected_model_id.to_string());
        }
        configuration = configuration
            .set(
                "llm.rig.default_model",
                AgentConfigValue::string(selected_model_id),
            )
            .set(
                "llm.rig.supported_models",
                AgentConfigValue::string_list(supported_model_ids),
            );
        let mut profile = AgentConfigurationProfile::new(
            &request.profile_id,
            &request.agent_id,
            configuration_version,
            configuration,
        )
        .activate();
        for binding in secret_bindings {
            profile = profile.add_secret_binding(binding);
        }
        if !profile.validate_against(&Self::spec()).is_valid() {
            return Err(KernelError::validation(
                "Rig model selection does not satisfy the configuration schema",
            ));
        }
        Ok(AgentModelConfigurationApplication::new(
            &request.request_id,
            "rig",
            profile,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
