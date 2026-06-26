use sdkwork_agent_kernel::{AgentInstaller, AgentPackageSource, KernelErrorKind, ProviderHealth};
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
        .required_policy_categories
        .contains(&"agent.install".to_string()));
    assert_eq!(installer.health(), ProviderHealth::available());
}

#[test]
fn rig_installer_reports_install_upgrade_and_uninstall_without_raw_secrets() {
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

    let upgrade = installer
        .plan_upgrade(&sdkwork_agent_kernel::AgentUpgradeRequest::new(
            "upgrade.rig.1",
            ids::AGENT_ID,
            "0.1.0",
            "0.1.1",
        ))
        .expect("rig upgrade plan is produced");
    assert!(upgrade
        .required_policy_categories
        .contains(&"agent.upgrade".to_string()));

    let uninstall = installer
        .uninstall(sdkwork_agent_kernel::AgentUninstallRequest::new(
            "uninstall.rig.1",
            ids::AGENT_ID,
        ))
        .expect("rig uninstall report is produced");
    assert_eq!(uninstall.agent_id, ids::AGENT_ID);
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
