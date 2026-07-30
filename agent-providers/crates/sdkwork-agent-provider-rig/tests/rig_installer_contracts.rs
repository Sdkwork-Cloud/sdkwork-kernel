use sdkwork_agent_kernel::{
    AgentInstallStatus, AgentInstallStepKind, AgentInstaller, AgentPackageSource,
    AgentUninstallRequest, AgentUpgradeRequest, KernelErrorKind, ProviderHealth,
};
use sdkwork_agent_provider_rig::{ids, rig_package_manifest, RigAgentInstaller};

#[test]
fn rig_installer_plans_before_installing_and_requires_policy() {
    let installer = RigAgentInstaller::new();
    let request = rig_package_manifest().install_request("install.rig.1");

    let plan = installer
        .plan_install(&request)
        .expect("rig install plan is produced");

    assert_eq!(plan.agent_id, ids::AGENT_ID);
    assert!(!plan.steps.is_empty());
    assert!(plan
        .steps
        .iter()
        .all(|step| step.kind == AgentInstallStepKind::VerifyPackage));
    assert!(plan
        .required_policy_categories
        .contains(&"agent.install".to_string()));
    assert_eq!(installer.health(), ProviderHealth::available());
    let installation = installer
        .detect_installation(ids::AGENT_ID)
        .expect("embedded Rig installation is detected");
    assert!(installation.is_installed());
    assert_eq!(installation.installed_version.as_deref(), Some("0.1.0"));
}

#[test]
fn rig_installer_reports_idempotent_install_and_upgrade_for_the_host_version() {
    let installer = RigAgentInstaller::new();
    let source = AgentPackageSource::registry("sdkwork", ids::AGENT_ID, "0.1.0");
    let install = installer
        .install(sdkwork_agent_kernel::AgentInstallRequest::new(
            "install.rig.1",
            ids::AGENT_ID,
            "0.1.0",
            source,
        ))
        .expect("rig install report is produced");
    assert_eq!(install.agent_id, ids::AGENT_ID);

    let upgrade_request =
        AgentUpgradeRequest::new("upgrade.rig.1", ids::AGENT_ID, "0.0.9", "0.1.0");
    let upgrade = installer
        .plan_upgrade(&upgrade_request)
        .expect("rig upgrade plan is produced");
    assert!(upgrade
        .steps
        .iter()
        .all(|step| step.kind == AgentInstallStepKind::VerifyPackage));
    assert!(upgrade
        .required_policy_categories
        .contains(&"agent.upgrade".to_string()));

    let upgraded = installer
        .upgrade(upgrade_request)
        .expect("host version is already current");
    assert_eq!(upgraded.status, AgentInstallStatus::Upgraded);
}

#[test]
fn rig_installer_plans_host_managed_uninstall_and_fails_closed_on_execution() {
    let installer = RigAgentInstaller::new();
    let request = AgentUninstallRequest::new("uninstall.rig.1", ids::AGENT_ID);
    let plan = installer
        .plan_uninstall(&request)
        .expect("host uninstall plan is produced");
    assert_eq!(plan.steps.len(), 1);

    let dry_run = installer
        .uninstall(request.clone().dry_run())
        .expect("dry-run host uninstall is planned");
    assert_eq!(dry_run.status, AgentInstallStatus::Planned);

    let error = installer
        .uninstall(request)
        .expect_err("embedded provider cannot remove itself");
    assert_eq!(error.kind(), KernelErrorKind::ProviderError);
    assert!(error.to_string().contains("host"));
}

#[test]
fn rig_installer_rejects_upgrade_to_a_version_not_embedded_in_the_host() {
    let installer = RigAgentInstaller::new();
    let request = AgentUpgradeRequest::new(
        "upgrade.rig.future",
        ids::AGENT_ID,
        "0.1.0",
        "0.1.1",
    );
    let plan = installer
        .plan_upgrade(&request)
        .expect("host replacement can be planned");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].kind, AgentInstallStepKind::ReplaceVersion);
    assert!(plan.steps[0].description.contains("host"));

    let dry_run = installer
        .upgrade(request.clone().dry_run())
        .expect("host replacement dry-run is planned");
    assert_eq!(dry_run.status, AgentInstallStatus::Planned);

    let error = installer
        .upgrade(request)
        .expect_err("host replacement is required");
    assert_eq!(error.kind(), KernelErrorKind::ProviderError);
}

#[test]
fn rig_future_install_dry_run_plans_host_replacement_without_false_registration_steps() {
    let installer = RigAgentInstaller::new();
    let request = sdkwork_agent_kernel::AgentInstallRequest::new(
        "install.rig.future",
        ids::AGENT_ID,
        "0.1.1",
        AgentPackageSource::registry("sdkwork", ids::AGENT_ID, "0.1.1"),
    );
    let plan = installer
        .plan_install(&request)
        .expect("future host replacement can be planned");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].kind, AgentInstallStepKind::ReplaceVersion);
    assert!(plan.steps[0].description.contains("host"));

    let report = installer
        .install(request.dry_run())
        .expect("future install dry-run is planned");
    assert_eq!(report.status, AgentInstallStatus::Planned);
}

#[test]
fn rig_installer_rejects_requests_for_other_agents() {
    let installer = RigAgentInstaller::new();
    let source = AgentPackageSource::registry("sdkwork", "agent.other", "0.1.0");

    let install_error = installer
        .plan_install(&sdkwork_agent_kernel::AgentInstallRequest::new(
            "install.other.1",
            "agent.other",
            "0.1.0",
            source,
        ))
        .expect_err("Rig installer must not plan installs for a different agent");
    assert_eq!(install_error.kind(), KernelErrorKind::ValidationError);

    let upgrade_error = installer
        .plan_upgrade(&sdkwork_agent_kernel::AgentUpgradeRequest::new(
            "upgrade.other.1",
            "agent.other",
            "0.1.0",
            "0.1.1",
        ))
        .expect_err("Rig installer must not plan upgrades for a different agent");
    assert_eq!(upgrade_error.kind(), KernelErrorKind::ValidationError);

    let uninstall_error = installer
        .uninstall(sdkwork_agent_kernel::AgentUninstallRequest::new(
            "uninstall.other.1",
            "agent.other",
        ))
        .expect_err("Rig installer must not uninstall a different agent");
    assert_eq!(uninstall_error.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn rig_installer_rejects_registry_sources_for_other_packages() {
    let installer = RigAgentInstaller::new();
    let source = AgentPackageSource::registry("sdkwork", "agent.other", "0.1.0");

    let install_error = installer
        .install(sdkwork_agent_kernel::AgentInstallRequest::new(
            "install.rig.bad-source",
            ids::AGENT_ID,
            "0.1.0",
            source,
        ))
        .expect_err("Rig installer must reject registry sources for a different package");

    assert_eq!(install_error.kind(), KernelErrorKind::ValidationError);
}
