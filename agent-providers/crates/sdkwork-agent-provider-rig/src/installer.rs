use sdkwork_agent_kernel::{
    AgentConfigurationProvider, AgentInstallPlan, AgentInstallReport, AgentInstallRequest,
    AgentInstallStep, AgentInstallStepKind, AgentInstallation, AgentInstallationDependency,
    AgentInstaller, AgentPackageSource, AgentUninstallPlan, AgentUninstallReport,
    AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport, AgentUpgradeRequest, KernelError,
    KernelResult, PolicyCategory, ProviderHealth, ProviderManifest,
};

use crate::{configuration::RigConfigurationProvider, ids};

#[derive(Debug, Clone, Default)]
pub struct RigAgentInstaller;

impl RigAgentInstaller {
    pub fn new() -> Self {
        Self
    }
}

const RIG_PROVIDER_VERSION: &str = env!("CARGO_PKG_VERSION");
const RIG_AGENT_VERSION: &str = "0.1.0";

fn validate_rig_agent_id(agent_id: &str) -> KernelResult<()> {
    if agent_id == ids::AGENT_ID {
        return Ok(());
    }

    Err(KernelError::validation(format!(
        "Rig installer can only manage {}; received {agent_id}",
        ids::AGENT_ID
    )))
}

fn validate_rig_package_source(
    source: &AgentPackageSource,
    target_version: &str,
) -> KernelResult<()> {
    let AgentPackageSource::Registry {
        package_id,
        version,
        ..
    } = source
    else {
        return Ok(());
    };

    if package_id != ids::AGENT_ID {
        return Err(KernelError::validation(format!(
            "Rig registry source must use package {}; received {package_id}",
            ids::AGENT_ID
        )));
    }

    if version != target_version {
        return Err(KernelError::validation(format!(
            "Rig registry source version {version} does not match target version {target_version}"
        )));
    }

    Ok(())
}

impl AgentInstaller for RigAgentInstaller {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::INSTALLER_PROVIDER_ID,
            "agent_installer",
            "rig-rust-host-lifecycle",
            RIG_PROVIDER_VERSION,
            vec![
                "agent.install".to_string(),
                "agent.uninstall".to_string(),
                "agent.upgrade".to_string(),
            ],
        )
    }

    fn detect_installation(&self, agent_id: &str) -> KernelResult<AgentInstallation> {
        validate_rig_agent_id(agent_id)?;
        Ok(
            AgentInstallation::installed(agent_id, RIG_AGENT_VERSION).with_dependency(
                AgentInstallationDependency::installed(
                    "sdkwork-host",
                    ids::AGENT_ID,
                    RIG_AGENT_VERSION,
                    RIG_AGENT_VERSION,
                ),
            ),
        )
    }

    fn configuration_spec(
        &self,
        agent_id: &str,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationSpec> {
        RigConfigurationProvider::new().configuration_spec(agent_id)
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
        validate_rig_agent_id(&request.agent_id)?;
        validate_rig_package_source(&request.source, &request.target_version)?;

        let step = if request.target_version == RIG_AGENT_VERSION {
            AgentInstallStep::new(
                "step.verify_embedded_provider",
                AgentInstallStepKind::VerifyPackage,
                "verify the Rig provider embedded in the running host",
            )
        } else {
            AgentInstallStep::new(
                "step.replace_host",
                AgentInstallStepKind::ReplaceVersion,
                "replace the host with a build embedding the requested Rig provider version",
            )
        };
        Ok(AgentInstallPlan::new(
            "plan.rig.install",
            ids::AGENT_ID,
            request.target_version.clone(),
        )
        .add_step(step)
        .require_policy(PolicyCategory::AgentInstall)
        .with_configuration_spec(RigConfigurationProvider::spec()))
    }

    fn install(&self, request: AgentInstallRequest) -> KernelResult<AgentInstallReport> {
        self.plan_install(&request)?;

        if request.dry_run {
            return Ok(AgentInstallReport::planned(
                request.request_id,
                request.agent_id,
                request.target_version,
            ));
        }
        validate_host_version(&request.target_version, "install")?;

        Ok(AgentInstallReport::installed(
            request.request_id,
            ids::AGENT_ID,
            request.target_version,
        ))
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
        validate_rig_agent_id(&request.agent_id)?;

        let step = if request.to_version == RIG_AGENT_VERSION {
            AgentInstallStep::new(
                "step.verify_embedded_provider",
                AgentInstallStepKind::VerifyPackage,
                "verify the Rig provider embedded in the running host",
            )
        } else {
            AgentInstallStep::new(
                "step.replace_host",
                AgentInstallStepKind::ReplaceVersion,
                "replace the host with a build embedding the requested Rig provider version",
            )
        };
        Ok(AgentUpgradePlan::new(
            "plan.rig.upgrade",
            ids::AGENT_ID,
            request.from_version.clone(),
            request.to_version.clone(),
        )
        .add_step(step)
        .with_rollback_required(request.rollback_required)
        .require_policy(PolicyCategory::AgentUpgrade))
    }

    fn upgrade(&self, request: AgentUpgradeRequest) -> KernelResult<AgentUpgradeReport> {
        self.plan_upgrade(&request)?;

        if request.dry_run {
            return Ok(AgentUpgradeReport::planned(
                request.request_id,
                request.agent_id,
                request.from_version,
                request.to_version,
            ));
        }
        validate_host_version(&request.to_version, "upgrade")?;

        Ok(AgentUpgradeReport::upgraded(
            request.request_id,
            ids::AGENT_ID,
            request.from_version,
            request.to_version,
        ))
    }

    fn plan_uninstall(&self, request: &AgentUninstallRequest) -> KernelResult<AgentUninstallPlan> {
        validate_rig_agent_id(&request.agent_id)?;

        Ok(
            AgentUninstallPlan::new("plan.rig.host-uninstall", ids::AGENT_ID)
                .add_step(AgentInstallStep::new(
                    "step.replace_host",
                    AgentInstallStepKind::ReplaceVersion,
                    "replace the host build without the embedded Rig provider",
                ))
                .require_policy(PolicyCategory::AgentUninstall),
        )
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        self.plan_uninstall(&request)?;

        if request.dry_run {
            return Ok(AgentUninstallReport::planned(
                request.request_id,
                request.agent_id,
            ));
        }

        Err(KernelError::provider_error(
            "embedded_provider_requires_host_update",
            "Rig is embedded in the running host; install a host build without the Rig provider to uninstall it",
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn validate_host_version(target_version: &str, operation: &str) -> KernelResult<()> {
    if target_version == RIG_AGENT_VERSION {
        return Ok(());
    }

    Err(KernelError::provider_error(
        "embedded_provider_requires_host_update",
        format!(
            "Rig {operation} target {target_version} requires replacing the host; running host embeds agent version {RIG_AGENT_VERSION}"
        )
    ))
}
