use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigSectionKind, AgentConfigValue,
    AgentConfigValueKind, AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentInstallPlan, AgentInstallReport, AgentInstallRequest,
    AgentInstallStep, AgentInstallStepKind, AgentInstaller, AgentUninstallReport,
    AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport, AgentUpgradeRequest, KernelError,
    KernelEventRedaction, KernelResult, ProviderHealth,
};

/// Fail-closed installer for external npm/PyPI process-adapter agents.
#[derive(Debug, Clone)]
pub struct ProcessAdapterInstaller {
    agent_id: String,
    package_label: String,
}

impl ProcessAdapterInstaller {
    pub fn new(agent_id: impl Into<String>, package_label: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            package_label: package_label.into(),
        }
    }

    fn validate_agent_id(&self, agent_id: &str) -> KernelResult<()> {
        if agent_id == self.agent_id {
            return Ok(());
        }

        Err(KernelError::validation(format!(
            "{} installer can only manage {}; received {agent_id}",
            self.package_label, self.agent_id
        )))
    }
}

impl AgentInstaller for ProcessAdapterInstaller {
    fn configuration_spec(
        &self,
        agent_id: &str,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationSpec> {
        ProcessAdapterConfigurationProvider::new(self.agent_id.clone()).configuration_spec(agent_id)
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
        self.validate_agent_id(&request.agent_id)?;

        Ok(AgentInstallPlan::new(
            format!("plan.{}.install", self.package_label),
            self.agent_id.clone(),
            request.target_version.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.verify_external_package",
            AgentInstallStepKind::VerifyPackage,
            format!(
                "verify {} is installed in the host environment (npm/PyPI/registry)",
                self.package_label
            ),
        )))
    }

    fn install(&self, request: AgentInstallRequest) -> KernelResult<AgentInstallReport> {
        self.validate_agent_id(&request.agent_id)?;
        Err(KernelError::ProviderUnavailable {
            provider_id: format!("provider.agent.installer.{}", self.package_label),
        })
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
        self.validate_agent_id(&request.agent_id)?;
        Err(KernelError::validation(
            "external process-adapter packages must be upgraded from the host environment",
        ))
    }

    fn upgrade(&self, request: AgentUpgradeRequest) -> KernelResult<AgentUpgradeReport> {
        self.validate_agent_id(&request.agent_id)?;
        Err(KernelError::validation(
            "external process-adapter packages must be upgraded from the host environment",
        ))
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        self.validate_agent_id(&request.agent_id)?;
        Err(KernelError::validation(
            "external process-adapter packages must be uninstalled from the host environment",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth {
            status: "degraded".to_string(),
        }
    }
}

/// Minimal configuration surface for external process-adapter agents.
#[derive(Debug, Clone)]
pub struct ProcessAdapterConfigurationProvider {
    agent_id: String,
}

impl ProcessAdapterConfigurationProvider {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
        }
    }

    pub fn spec_for(agent_id: &str) -> AgentConfigurationSpec {
        AgentConfigurationSpec::new(agent_id)
            .add_section(
                AgentConfigSection::base("base", "Base").add_field(
                    AgentConfigField::text("agent.display_name", "Display name").required(),
                ),
            )
            .add_section(AgentConfigSection::llm_api_key("llm", "LLM").add_field(
                AgentConfigField::llm_api_key("llm.api_key", "Model provider API key"),
            ))
            .add_section(
                AgentConfigSection::new("runtime", "Runtime", AgentConfigSectionKind::Runtime)
                    .add_field(
                        AgentConfigField::text("runtime.external.backend", "External backend")
                            .required()
                            .with_default(AgentConfigValue::string("process_adapter")),
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

impl AgentConfigurationProvider for ProcessAdapterConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        if agent_id != self.agent_id {
            return Err(KernelError::CapabilityMissing {
                capability_id: agent_id.to_string(),
            });
        }

        Ok(Self::spec_for(agent_id))
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<AgentConfigurationValidation> {
        Ok(Self::spec_for(&self.agent_id).validate(configuration))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
