use sdkwork_agent_integration_rig::{ids, rig_package_manifest, RigAgentInstaller};
use sdkwork_agent_kernel::{AgentInstaller, AgentPackageSource, ProviderHealth};

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
