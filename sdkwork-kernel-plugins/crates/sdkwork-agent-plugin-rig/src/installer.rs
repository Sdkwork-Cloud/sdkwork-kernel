use sdkwork_agent_kernel::{
    AgentConfigurationProvider, AgentInstallPlan, AgentInstallReport, AgentInstallRequest,
    AgentInstallStep, AgentInstallStepKind, AgentInstaller, AgentPackageSource,
    AgentUninstallReport, AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport,
    AgentUpgradeRequest, KernelError, KernelResult, PolicyCategory, ProviderHealth,
};

use crate::{configuration::RigConfigurationProvider, ids};

#[derive(Debug, Clone, Default)]
pub struct RigAgentInstaller;

impl RigAgentInstaller {
    pub fn new() -> Self {
        Self
    }
}

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
    fn configuration_spec(
        &self,
        agent_id: &str,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationSpec> {
        RigConfigurationProvider::new().configuration_spec(agent_id)
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
        validate_rig_agent_id(&request.agent_id)?;
        validate_rig_package_source(&request.source, &request.target_version)?;

        Ok(AgentInstallPlan::new(
            "plan.rig.install",
            ids::AGENT_ID,
            request.target_version.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.verify",
            AgentInstallStepKind::VerifyPackage,
            "verify Rig package manifest",
        ))
        .add_step(AgentInstallStep::new(
            "step.register",
            AgentInstallStepKind::RegisterAgent,
            "register Rig agent manifest",
        ))
        .add_step(AgentInstallStep::new(
            "step.configure",
            AgentInstallStepKind::ConfigureAgent,
            "bind Rig configuration profile",
        ))
        .require_policy(PolicyCategory::AgentInstall)
        .with_configuration_spec(RigConfigurationProvider::spec()))
    }

    fn install(&self, request: AgentInstallRequest) -> KernelResult<AgentInstallReport> {
        validate_rig_agent_id(&request.agent_id)?;
        validate_rig_package_source(&request.source, &request.target_version)?;

        Ok(AgentInstallReport::installed(
            request.request_id,
            ids::AGENT_ID,
            request.target_version,
        ))
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
        validate_rig_agent_id(&request.agent_id)?;

        Ok(AgentUpgradePlan::new(
            "plan.rig.upgrade",
            ids::AGENT_ID,
            request.from_version.clone(),
            request.to_version.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.backup",
            AgentInstallStepKind::BackupCurrentVersion,
            "record current Rig package state",
        ))
        .add_step(AgentInstallStep::new(
            "step.replace",
            AgentInstallStepKind::ReplaceVersion,
            "replace Rig package version",
        ))
        .with_rollback_required(request.rollback_required)
        .require_policy(PolicyCategory::AgentUpgrade))
    }

    fn upgrade(&self, request: AgentUpgradeRequest) -> KernelResult<AgentUpgradeReport> {
        validate_rig_agent_id(&request.agent_id)?;

        Ok(AgentUpgradeReport::upgraded(
            request.request_id,
            ids::AGENT_ID,
            request.from_version,
            request.to_version,
        ))
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        validate_rig_agent_id(&request.agent_id)?;

        Ok(
            AgentUninstallReport::uninstalled(request.request_id, ids::AGENT_ID)
                .with_configuration_removed(request.remove_configuration),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
