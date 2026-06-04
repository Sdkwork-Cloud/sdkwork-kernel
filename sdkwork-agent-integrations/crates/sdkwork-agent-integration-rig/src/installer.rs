use sdkwork_agent_kernel::{
    AgentConfigurationProvider, AgentInstallPlan, AgentInstallReport, AgentInstallRequest,
    AgentInstallStep, AgentInstallStepKind, AgentInstaller, AgentUninstallReport,
    AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport, AgentUpgradeRequest, KernelResult,
    PolicyCategory, ProviderHealth,
};

use crate::{configuration::RigConfigurationProvider, ids};

#[derive(Debug, Clone, Default)]
pub struct RigAgentInstaller;

impl RigAgentInstaller {
    pub fn new() -> Self {
        Self
    }
}

impl AgentInstaller for RigAgentInstaller {
    fn configuration_spec(
        &self,
        agent_id: &str,
    ) -> KernelResult<sdkwork_agent_kernel::AgentConfigurationSpec> {
        RigConfigurationProvider::new().configuration_spec(agent_id)
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
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
        Ok(AgentInstallReport::installed(
            request.request_id,
            ids::AGENT_ID,
            request.target_version,
        ))
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
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
        Ok(AgentUpgradeReport::upgraded(
            request.request_id,
            ids::AGENT_ID,
            request.from_version,
            request.to_version,
        ))
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        Ok(
            AgentUninstallReport::uninstalled(request.request_id, ids::AGENT_ID)
                .with_configuration_removed(request.remove_configuration),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
