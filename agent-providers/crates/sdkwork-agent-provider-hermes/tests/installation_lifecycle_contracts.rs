//! Installation lifecycle contract tests for Hermes.
//!
//! Drives the crate's real `ProcessAdapterInstaller` descriptor through
//! detection, planning, dry-run, install, upgrade with rollback metadata,
//! rollback, uninstall, verification, and inventory using a scripted command
//! executor. No registry or filesystem access is required.

use std::sync::Arc;

use sdkwork_agent_kernel::{
    AgentInstallStatus, AgentInstallationState, AgentInstaller, AgentPackageSource,
    AgentRollbackRequest, AgentRollbackStatus, AgentUninstallRequest, AgentUpgradeRequest,
    AgentVerifyRequest, AgentVerifyStatus, PolicyCategory,
};
use sdkwork_agent_plugin_core::{
    pypi_metadata_payload, ProcessAdapterCommandOutput, ProcessAdapterInstaller,
    ScriptedCommandExecutor,
};
use sdkwork_agent_provider_hermes::{
    hermes_agent_installer, HERMES_PACKAGE, HERMES_PACKAGE_VERSION,
};

const AGENT_ID: &str = "agent.hermes";
const PROVIDER_VERSION: &str = env!("CARGO_PKG_VERSION");

fn installer(executor: ScriptedCommandExecutor) -> ProcessAdapterInstaller {
    hermes_agent_installer().with_executor(Arc::new(executor))
}

fn default_python_binary() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

fn install_request() -> sdkwork_agent_kernel::AgentInstallRequest {
    sdkwork_agent_kernel::AgentInstallRequest::new(
        "install.lifecycle.1",
        AGENT_ID,
        PROVIDER_VERSION,
        AgentPackageSource::registry("pypi", HERMES_PACKAGE, HERMES_PACKAGE_VERSION),
    )
}

fn installed_payload() -> String {
    pypi_metadata_payload(&[(HERMES_PACKAGE, Some(HERMES_PACKAGE_VERSION))])
}

fn old_payload() -> String {
    pypi_metadata_payload(&[(HERMES_PACKAGE, Some("0.0.1"))])
}

fn absent_payload() -> String {
    pypi_metadata_payload(&[(HERMES_PACKAGE, None)])
}

#[test]
fn detects_missing_and_installed_states() {
    let missing = installer(ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(absent_payload()),
    ]));
    let detection = missing
        .detect_installation(AGENT_ID)
        .expect("provider installation is detected");
    assert_eq!(detection.state, AgentInstallationState::NotInstalled);

    let installed = installer(ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(installed_payload()),
    ]));
    let detection = installed
        .detect_installation(AGENT_ID)
        .expect("provider installation is detected");
    assert_eq!(detection.state, AgentInstallationState::Installed);
    assert_eq!(
        detection.installed_version.as_deref(),
        Some(PROVIDER_VERSION)
    );
    assert!(detection
        .dependencies
        .iter()
        .all(|dependency| dependency.version_matches()));
}

#[test]
fn plans_with_policy_and_dry_runs_without_mutating() {
    let executor = ScriptedCommandExecutor::default();
    let inspector = executor.clone();
    let installer = installer(executor);

    let plan = installer
        .plan_install(&install_request())
        .expect("install plan is generated");
    assert!(plan.requires_policy());
    assert_eq!(
        plan.required_policy_categories,
        [PolicyCategory::AgentInstall.as_str().to_string()]
    );

    let install = installer
        .install(install_request().dry_run())
        .expect("install dry run");
    assert_eq!(install.status, AgentInstallStatus::Planned);

    let upgrade = installer
        .upgrade(
            AgentUpgradeRequest::new("upgrade.lifecycle.dry", AGENT_ID, "0.1.0", PROVIDER_VERSION)
                .dry_run(),
        )
        .expect("upgrade dry run");
    assert_eq!(upgrade.status, AgentInstallStatus::Planned);

    let uninstall = installer
        .uninstall(AgentUninstallRequest::new("uninstall.lifecycle.dry", AGENT_ID).dry_run())
        .expect("uninstall dry run");
    assert_eq!(uninstall.status, AgentInstallStatus::Planned);

    assert!(inspector.commands().is_empty());
}

#[test]
fn installs_exact_packages_and_verifies_the_result() {
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(absent_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(installed_payload()),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let report = installer
        .install(install_request())
        .expect("provider installs");
    assert_eq!(report.status, AgentInstallStatus::Installed);

    let commands = inspector.commands();
    assert_eq!(commands.len(), 3);
    let detection = &commands[0];
    assert_eq!(detection.program, default_python_binary());
    assert!(detection.args.iter().any(|argument| argument == "-c"));
    let install = &commands[1];
    assert_eq!(install.program, default_python_binary());
    assert!(install.args.iter().any(|argument| argument == "-m"));
    assert!(install.args.iter().any(|argument| argument == "pip"));
    assert!(install.args.iter().any(|argument| argument == "install"));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == "--only-binary=:all:"));
    assert!(install.args.iter().any(|argument| argument == "--no-input"));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == &format!("{HERMES_PACKAGE}=={HERMES_PACKAGE_VERSION}")));
    assert!(!install.args.iter().any(|argument| argument.contains("&&")));
}

#[test]
fn upgrades_with_rollback_metadata_and_verifies() {
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(old_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(installed_payload()),
        ProcessAdapterCommandOutput::success(installed_payload()),
    ]);
    let installer = installer(executor);

    let report = installer
        .upgrade(
            AgentUpgradeRequest::new("upgrade.lifecycle.1", AGENT_ID, "0.1.0", PROVIDER_VERSION)
                .with_rollback_required(),
        )
        .expect("provider upgrades");
    assert_eq!(report.status, AgentInstallStatus::Upgraded);
    let token = report
        .rollback_token
        .as_ref()
        .expect("rollback-required upgrade returns an opaque rollback token");
    assert!(token.chars().all(|character| character.is_ascii_hexdigit()));

    let verify = installer
        .verify_installation(&AgentVerifyRequest::new("verify.lifecycle.1", AGENT_ID))
        .expect("installed provider verifies");
    assert_eq!(verify.status, AgentVerifyStatus::Valid);
}

#[test]
fn rollback_restores_the_snapshotted_dependency_state() {
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(old_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(installed_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(old_payload()),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let upgrade = installer
        .upgrade(
            AgentUpgradeRequest::new("upgrade.lifecycle.rb", AGENT_ID, "0.1.0", PROVIDER_VERSION)
                .with_rollback_required(),
        )
        .expect("provider upgrades with rollback metadata");
    let token = upgrade
        .rollback_token
        .expect("rollback-required upgrade returns an opaque rollback token");

    let rollback = installer
        .rollback(
            AgentRollbackRequest::new("rollback.lifecycle.1", AGENT_ID)
                .with_rollback_token(token)
                .to_version("0.1.0"),
        )
        .expect("provider rolls back to the snapshotted state");
    assert_eq!(rollback.status, AgentRollbackStatus::Success);
    assert_eq!(rollback.to_version, "0.1.0");

    let commands = inspector.commands();
    assert_eq!(commands.len(), 5);
    let restore = &commands[3];
    assert_eq!(restore.program, default_python_binary());
    assert!(restore
        .args
        .iter()
        .any(|argument| argument == &format!("{HERMES_PACKAGE}==0.0.1")));
}

#[test]
fn uninstall_removes_the_provider_and_verifies_absence() {
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(installed_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(absent_payload()),
        ProcessAdapterCommandOutput::success(absent_payload()),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let report = installer
        .uninstall(AgentUninstallRequest::new(
            "uninstall.lifecycle.1",
            AGENT_ID,
        ))
        .expect("provider uninstalls");
    assert_eq!(report.status, AgentInstallStatus::Uninstalled);

    let commands = inspector.commands();
    assert_eq!(commands.len(), 3);
    let uninstall = &commands[1];
    assert_eq!(uninstall.program, default_python_binary());
    assert!(uninstall
        .args
        .iter()
        .any(|argument| argument == "uninstall"));
    assert!(uninstall.args.iter().any(|argument| argument == "--yes"));
    assert!(uninstall
        .args
        .iter()
        .any(|argument| argument == HERMES_PACKAGE));

    assert!(
        installer
            .verify_installation(&AgentVerifyRequest::new("verify.lifecycle.2", AGENT_ID))
            .expect("absent provider verification")
            .status
            == AgentVerifyStatus::NotFound
    );
}

#[test]
fn inventory_reports_only_proven_records() {
    let installed = installer(ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(installed_payload()),
    ]));
    let records = installed.list_installed().expect("installed inventory");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].agent_id, AGENT_ID);
    assert_eq!(
        records[0].source,
        sdkwork_agent_kernel::AgentPackageSourceInfo::Registry {
            registry_id: "pypi".to_string(),
            package_id: HERMES_PACKAGE.to_string(),
        }
    );

    let missing = installer(ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(absent_payload()),
    ]));
    assert!(missing
        .list_installed()
        .expect("empty inventory")
        .is_empty());
}
