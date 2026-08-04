//! Installation lifecycle contract tests for Claude Code.
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
    npm_absent_payload, npm_list_payload, ProcessAdapterCommandOutput, ProcessAdapterInstaller,
    ScriptedCommandExecutor, TemporaryInstallRoot,
};
use sdkwork_agent_provider_claude_code::{
    claude_code_agent_installer, ANTHROPIC_SDK_PACKAGE, ANTHROPIC_SDK_VERSION,
    CLAUDE_AGENT_SDK_PACKAGE, CLAUDE_AGENT_SDK_VERSION, MCP_SDK_PACKAGE, MCP_SDK_VERSION,
    ZOD_PACKAGE, ZOD_VERSION,
};

const AGENT_ID: &str = "agent.claude-code";
const PROVIDER_VERSION: &str = env!("CARGO_PKG_VERSION");

fn installer(root: &std::path::Path, executor: ScriptedCommandExecutor) -> ProcessAdapterInstaller {
    claude_code_agent_installer()
        .with_install_root(root)
        .with_executor(Arc::new(executor))
}

fn install_request() -> sdkwork_agent_kernel::AgentInstallRequest {
    sdkwork_agent_kernel::AgentInstallRequest::new(
        "install.lifecycle.1",
        AGENT_ID,
        PROVIDER_VERSION,
        AgentPackageSource::registry("npm", CLAUDE_AGENT_SDK_PACKAGE, CLAUDE_AGENT_SDK_VERSION),
    )
}

fn installed_payload() -> String {
    npm_list_payload(&[
        (CLAUDE_AGENT_SDK_PACKAGE, Some(CLAUDE_AGENT_SDK_VERSION)),
        (ANTHROPIC_SDK_PACKAGE, Some(ANTHROPIC_SDK_VERSION)),
        (MCP_SDK_PACKAGE, Some(MCP_SDK_VERSION)),
        (ZOD_PACKAGE, Some(ZOD_VERSION)),
    ])
}

#[test]
fn detects_missing_and_installed_states() {
    let root = TemporaryInstallRoot::new("claude-code-detect");
    let missing = installer(
        root.path(),
        ScriptedCommandExecutor::with_outputs(vec![ProcessAdapterCommandOutput::success(
            npm_absent_payload(),
        )]),
    );
    let detection = missing
        .detect_installation(AGENT_ID)
        .expect("provider installation is detected");
    assert_eq!(detection.state, AgentInstallationState::NotInstalled);
    assert!(!detection.is_installed());

    let installed = installer(
        root.path(),
        ScriptedCommandExecutor::with_outputs(vec![ProcessAdapterCommandOutput::success(
            installed_payload(),
        )]),
    );
    let detection = installed
        .detect_installation(AGENT_ID)
        .expect("provider installation is detected");
    assert_eq!(detection.state, AgentInstallationState::Installed);
    assert!(detection.is_installed());
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
    let root = TemporaryInstallRoot::new("claude-code-plan");
    let executor = ScriptedCommandExecutor::default();
    let inspector = executor.clone();
    let installer = installer(root.path(), executor);

    let plan = installer
        .plan_install(&install_request())
        .expect("install plan is generated");
    assert!(plan.requires_policy());
    assert_eq!(
        plan.required_policy_categories,
        [PolicyCategory::AgentInstall.as_str().to_string()]
    );
    assert!(plan
        .steps
        .iter()
        .any(|step| step.kind == sdkwork_agent_kernel::AgentInstallStepKind::VerifyPackage));

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
    let root = TemporaryInstallRoot::new("claude-code-install");
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_absent_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(installed_payload()),
    ]);
    let inspector = executor.clone();
    let installer = installer(root.path(), executor);

    let report = installer
        .install(install_request())
        .expect("provider installs");
    assert_eq!(report.status, AgentInstallStatus::Installed);
    assert_eq!(report.installed_version.as_deref(), Some(PROVIDER_VERSION));

    let commands = inspector.commands();
    assert_eq!(commands.len(), 3);
    let detection = &commands[0];
    assert_eq!(detection.program, "npm");
    assert!(detection.args.iter().any(|argument| argument == "list"));
    assert!(detection
        .args
        .iter()
        .any(|argument| argument == "--depth=0"));
    let install = &commands[1];
    assert_eq!(install.program, "npm");
    assert!(install
        .args
        .iter()
        .any(|argument| argument
            == &format!("{CLAUDE_AGENT_SDK_PACKAGE}@{CLAUDE_AGENT_SDK_VERSION}")));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == &format!("{ANTHROPIC_SDK_PACKAGE}@{ANTHROPIC_SDK_VERSION}")));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == &format!("{MCP_SDK_PACKAGE}@{MCP_SDK_VERSION}")));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == &format!("{ZOD_PACKAGE}@{ZOD_VERSION}")));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == "--save-exact"));
    assert!(install
        .args
        .iter()
        .any(|argument| argument == "--ignore-scripts"));
    assert!(!install.args.iter().any(|argument| argument.contains("&&")));
    assert!(std::path::PathBuf::from(&install.args[1]).is_absolute());
}

#[test]
fn upgrades_with_rollback_metadata_and_verifies() {
    let root = TemporaryInstallRoot::new("claude-code-upgrade");
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(old_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(installed_payload()),
        ProcessAdapterCommandOutput::success(installed_payload()),
    ]);
    let installer = installer(root.path(), executor);

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
    assert!(!report
        .to_event("event.upgrade.lifecycle.1")
        .payload
        .contains(token));

    let verify = installer
        .verify_installation(&AgentVerifyRequest::new("verify.lifecycle.1", AGENT_ID))
        .expect("installed provider verifies");
    assert_eq!(verify.status, AgentVerifyStatus::Valid);
    assert_eq!(verify.checksum_valid, Some(true));
}

#[test]
fn rollback_restores_the_snapshotted_dependency_state() {
    let root = TemporaryInstallRoot::new("claude-code-rollback");
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(old_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(installed_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(old_payload()),
    ]);
    let inspector = executor.clone();
    let installer = installer(root.path(), executor);

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
    assert_eq!(restore.program, "npm");
    assert!(restore
        .args
        .iter()
        .any(|argument| argument == &format!("{ANTHROPIC_SDK_PACKAGE}@0.0.1")));
    assert!(restore
        .args
        .iter()
        .any(|argument| argument == &format!("{MCP_SDK_PACKAGE}@0.0.1")));
    assert!(restore
        .args
        .iter()
        .any(|argument| argument == &format!("{ZOD_PACKAGE}@0.0.1")));
}

#[test]
fn uninstall_removes_the_provider_and_verifies_absence() {
    let root = TemporaryInstallRoot::new("claude-code-uninstall");
    let executor = ScriptedCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(installed_payload()),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_absent_payload()),
        ProcessAdapterCommandOutput::success(npm_absent_payload()),
    ]);
    let inspector = executor.clone();
    let installer = installer(root.path(), executor);

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
    assert_eq!(uninstall.program, "npm");
    assert!(uninstall
        .args
        .iter()
        .any(|argument| argument == CLAUDE_AGENT_SDK_PACKAGE));
    assert!(uninstall
        .args
        .iter()
        .any(|argument| argument == ANTHROPIC_SDK_PACKAGE));
    assert!(uninstall
        .args
        .iter()
        .any(|argument| argument == MCP_SDK_PACKAGE));
    assert!(uninstall
        .args
        .iter()
        .any(|argument| argument == ZOD_PACKAGE));
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
    let root = TemporaryInstallRoot::new("claude-code-inventory");
    let installed = installer(
        root.path(),
        ScriptedCommandExecutor::with_outputs(vec![ProcessAdapterCommandOutput::success(
            installed_payload(),
        )]),
    );
    let records = installed.list_installed().expect("installed inventory");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].agent_id, AGENT_ID);
    assert_eq!(
        records[0].source,
        sdkwork_agent_kernel::AgentPackageSourceInfo::Registry {
            registry_id: "npm".to_string(),
            package_id: CLAUDE_AGENT_SDK_PACKAGE.to_string(),
        }
    );

    let missing = installer(
        root.path(),
        ScriptedCommandExecutor::with_outputs(vec![ProcessAdapterCommandOutput::success(
            npm_absent_payload(),
        )]),
    );
    assert!(missing
        .list_installed()
        .expect("empty inventory")
        .is_empty());
}

fn old_payload() -> String {
    npm_list_payload(&[
        (CLAUDE_AGENT_SDK_PACKAGE, Some("0.0.1")),
        (ANTHROPIC_SDK_PACKAGE, Some("0.0.1")),
        (MCP_SDK_PACKAGE, Some("0.0.1")),
        (ZOD_PACKAGE, Some("0.0.1")),
    ])
}
