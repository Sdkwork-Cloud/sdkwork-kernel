use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Barrier, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use sdkwork_agent_kernel::{
    AgentConfiguration, AgentInstallStatus, AgentInstaller, AgentPackageSource,
    AgentUninstallRequest, AgentUpgradeRequest, KernelErrorKind,
};
use sdkwork_agent_plugin_core::{
    ProcessAdapterCommand, ProcessAdapterCommandExecutor, ProcessAdapterCommandOutput,
    ProcessAdapterInstaller, ProcessAdapterPackage, SystemProcessAdapterCommandExecutor,
};

const AGENT_ID: &str = "agent.intelligence.codex";
const INSTALLER_ID: &str = "provider.agent.installer.codex";
const PROVIDER_VERSION: &str = "0.2.0";
const PACKAGE_ID: &str = "@openai/codex-sdk";
const PACKAGE_VERSION: &str = "0.146.0";

#[derive(Clone, Default)]
struct FakeCommandExecutor {
    outputs: Arc<Mutex<VecDeque<ProcessAdapterCommandOutput>>>,
    commands: Arc<Mutex<Vec<ProcessAdapterCommand>>>,
}

impl FakeCommandExecutor {
    fn with_outputs(outputs: Vec<ProcessAdapterCommandOutput>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(outputs.into())),
            commands: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn commands(&self) -> Vec<ProcessAdapterCommand> {
        self.commands.lock().expect("commands lock").clone()
    }
}

impl ProcessAdapterCommandExecutor for FakeCommandExecutor {
    fn execute(
        &self,
        command: &ProcessAdapterCommand,
    ) -> sdkwork_agent_kernel::KernelResult<ProcessAdapterCommandOutput> {
        self.commands
            .lock()
            .expect("commands lock")
            .push(command.clone());
        self.outputs
            .lock()
            .expect("outputs lock")
            .pop_front()
            .ok_or_else(|| sdkwork_agent_kernel::KernelError::Internal {
                message: "fake command output exhausted".to_string(),
            })
    }
}

fn npm_detection(package_version: Option<&str>, openai_version: Option<&str>) -> String {
    let mut dependencies = Vec::new();
    if let Some(version) = package_version {
        dependencies.push(format!(r#""{PACKAGE_ID}":{{"version":"{version}"}}"#));
    }
    if let Some(version) = openai_version {
        dependencies.push(format!(r#""openai":{{"version":"{version}"}}"#));
    }
    format!(r#"{{"dependencies":{{{}}}}}"#, dependencies.join(","))
}

fn python_detection(package_version: Option<&str>) -> String {
    match package_version {
        Some(version) => format!(r#"{{"hermes-agent":"{version}"}}"#),
        None => r#"{"hermes-agent":null}"#.to_string(),
    }
}

fn installer(executor: FakeCommandExecutor) -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        AGENT_ID,
        INSTALLER_ID,
        PROVIDER_VERSION,
        ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION),
    )
    .with_dependency(ProcessAdapterPackage::npm("openai", "7.1.0"))
    .with_install_root(PathBuf::from("provider-runtime"))
    .with_executor(Arc::new(executor))
}

fn install_request() -> sdkwork_agent_kernel::AgentInstallRequest {
    sdkwork_agent_kernel::AgentInstallRequest::new(
        "install.codex.1",
        AGENT_ID,
        PROVIDER_VERSION,
        AgentPackageSource::registry("npm", PACKAGE_ID, PACKAGE_VERSION),
    )
}

#[test]
fn detects_complete_degraded_and_missing_provider_installations() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), Some("7.1.0"))),
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), None)),
        ProcessAdapterCommandOutput::success(npm_detection(None, Some("7.1.0"))),
        ProcessAdapterCommandOutput::success(npm_detection(None, None)),
    ]);
    let installer = installer(executor);

    let installed = installer.detect_installation(AGENT_ID).expect("installed");
    assert!(installed.is_installed());
    assert_eq!(
        installed.installed_version.as_deref(),
        Some(PROVIDER_VERSION)
    );
    assert!(installed
        .dependencies
        .iter()
        .all(|item| item.version_matches()));

    let degraded = installer.detect_installation(AGENT_ID).expect("degraded");
    assert!(degraded.is_degraded());
    assert_eq!(degraded.dependencies[1].installed_version, None);

    let partial = installer.detect_installation(AGENT_ID).expect("partial");
    assert!(partial.is_degraded());
    assert_eq!(partial.installed_version, None);

    let missing = installer.detect_installation(AGENT_ID).expect("missing");
    assert!(!missing.is_installed());
    assert!(!missing.is_degraded());
}

#[test]
fn installs_exact_packages_and_verifies_the_result() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_detection(None, None)),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), Some("7.1.0"))),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let report = installer
        .install(install_request())
        .expect("provider installs");
    assert_eq!(report.status, AgentInstallStatus::Installed);
    assert_eq!(report.installed_version.as_deref(), Some(PROVIDER_VERSION));

    let commands = inspector.commands();
    assert_eq!(commands.len(), 3);
    let install = &commands[1];
    assert_eq!(install.program, "npm");
    assert!(install
        .args
        .contains(&format!("{PACKAGE_ID}@{PACKAGE_VERSION}")));
    assert!(install.args.contains(&"openai@7.1.0".to_string()));
    assert!(install.args.contains(&"--ignore-scripts".to_string()));
    assert!(!install.args.iter().any(|arg| arg.contains("&&")));
    assert!(PathBuf::from(&install.args[1]).is_absolute());
    #[cfg(windows)]
    assert!(!install.args[1].starts_with(r"\\?\"));
    assert_eq!(install.timeout, Some(Duration::from_secs(30 * 60)));
}

#[test]
fn upgrade_is_exact_idempotent_and_post_verified() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_detection(Some("0.145.0"), Some("7.0.0"))),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), Some("7.1.0"))),
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), Some("7.1.0"))),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);
    let request = AgentUpgradeRequest::new("upgrade.codex.1", AGENT_ID, "0.1.0", PROVIDER_VERSION)
        .with_rollback_required();

    let report = installer
        .upgrade(request.clone())
        .expect("provider upgrades");
    assert_eq!(report.status, AgentInstallStatus::Upgraded);
    assert!(report.rollback_token.is_none());

    let idempotent = installer
        .upgrade(request)
        .expect("repeat upgrade is idempotent");
    assert_eq!(idempotent.status, AgentInstallStatus::Upgraded);
    assert_eq!(inspector.commands().len(), 4);
}

#[test]
fn uninstall_is_idempotent_and_post_verified() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), Some("7.1.0"))),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_detection(None, None)),
        ProcessAdapterCommandOutput::success(npm_detection(None, None)),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let report = installer
        .uninstall(AgentUninstallRequest::new("uninstall.codex.1", AGENT_ID))
        .expect("provider uninstalls");
    assert_eq!(report.status, AgentInstallStatus::Uninstalled);
    assert_eq!(
        inspector.commands()[1].timeout,
        Some(Duration::from_secs(30 * 60))
    );

    installer
        .uninstall(AgentUninstallRequest::new("uninstall.codex.2", AGENT_ID))
        .expect("repeat uninstall is idempotent");
    assert_eq!(inspector.commands().len(), 4);
}

#[test]
fn dry_run_plans_without_mutating_the_host() {
    let executor = FakeCommandExecutor::default();
    let inspector = executor.clone();
    let installer = installer(executor);

    let install = installer
        .install(install_request().dry_run())
        .expect("install dry run");
    let upgrade = installer
        .upgrade(
            AgentUpgradeRequest::new("upgrade.codex.dry", AGENT_ID, "0.1.0", PROVIDER_VERSION)
                .dry_run(),
        )
        .expect("upgrade dry run");
    let uninstall = installer
        .uninstall(AgentUninstallRequest::new("uninstall.codex.dry", AGENT_ID).dry_run())
        .expect("uninstall dry run");

    assert_eq!(install.status, AgentInstallStatus::Planned);
    assert_eq!(install.target_version, PROVIDER_VERSION);
    assert_eq!(install.installed_version, None);
    assert_eq!(upgrade.status, AgentInstallStatus::Planned);
    assert_eq!(uninstall.status, AgentInstallStatus::Planned);
    assert!(inspector.commands().is_empty());
}

#[test]
fn invalid_install_and_upgrade_configuration_fail_before_execution() {
    let executor = FakeCommandExecutor::default();
    let inspector = executor.clone();
    let installer = installer(executor);
    let invalid = AgentConfiguration::new(AGENT_ID, "profile.invalid");

    let install_error = installer
        .plan_install(&install_request().with_configuration(invalid.clone()))
        .expect_err("invalid install configuration is rejected");
    assert_eq!(install_error.kind(), KernelErrorKind::ValidationError);
    let upgrade_error = installer
        .plan_upgrade(
            &AgentUpgradeRequest::new(
                "upgrade.codex.invalid-config",
                AGENT_ID,
                "0.1.0",
                PROVIDER_VERSION,
            )
            .with_configuration(invalid),
        )
        .expect_err("invalid upgrade configuration is rejected");
    assert_eq!(upgrade_error.kind(), KernelErrorKind::ValidationError);
    assert!(inspector.commands().is_empty());
}

#[test]
fn installation_detection_timeout_fails_closed() {
    let timed_out = ProcessAdapterCommandOutput {
        exit_code: -1,
        stdout: String::new(),
        stderr: String::new(),
        timed_out: true,
    };
    let executor = FakeCommandExecutor::with_outputs(vec![timed_out.clone()]);
    let installer = installer(executor);

    let error = installer
        .detect_installation(AGENT_ID)
        .expect_err("detection timeout must not be reported as not installed");
    assert_eq!(error.kind(), KernelErrorKind::Timeout);

    let python_installer = ProcessAdapterInstaller::new(
        "agent.intelligence.hermes",
        "provider.agent.installer.hermes",
        PROVIDER_VERSION,
        ProcessAdapterPackage::pypi("hermes-agent", "0.19.0"),
    )
    .with_executor(Arc::new(FakeCommandExecutor::with_outputs(vec![timed_out])));
    let error = python_installer
        .detect_installation("agent.intelligence.hermes")
        .expect_err("Python detection timeout must fail closed");
    assert_eq!(error.kind(), KernelErrorKind::Timeout);
}

#[test]
fn package_installer_does_not_claim_host_configuration_removal() {
    let executor = FakeCommandExecutor::default();
    let inspector = executor.clone();
    let installer = installer(executor);

    let error = installer
        .uninstall(
            AgentUninstallRequest::new("uninstall.codex.configuration", AGENT_ID)
                .remove_configuration(),
        )
        .expect_err("configuration removal requires the host configuration store");
    assert_eq!(error.kind(), KernelErrorKind::ProviderError);
    assert_eq!(error.code(), "provider_configuration_removal_requires_host");
    assert!(inspector.commands().is_empty());
}

#[test]
fn package_manager_failures_do_not_expose_stderr_credentials() {
    let secret = "registry-token-must-not-leak";
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_detection(None, None)),
        ProcessAdapterCommandOutput::failure(
            1,
            format!("authentication failed for https://user:{secret}@registry.example"),
        ),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_detection(None, None)),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let error = installer
        .install(install_request())
        .expect_err("package manager failure must fail the installation");

    assert_eq!(error.code(), "provider_package_install_failed");
    assert_eq!(
        error.message(),
        "provider package manager command failed with exit code 1"
    );
    assert!(!error.message().contains(secret));
    assert!(!error.safe_message().contains(secret));
    let commands = inspector.commands();
    assert_eq!(commands.len(), 4);
    assert!(commands[2]
        .args
        .iter()
        .any(|argument| argument == "uninstall"));
}

#[test]
fn command_output_debug_is_content_redacted() {
    let secret = "command-output-secret";
    let output = ProcessAdapterCommandOutput {
        exit_code: 1,
        stdout: secret.to_string(),
        stderr: secret.to_string(),
        timed_out: false,
    };

    let debug = format!("{output:?}");
    assert!(!debug.contains(secret));
    assert!(debug.contains("stdout_bytes"));
    assert!(debug.contains("stderr_bytes"));
}

#[test]
fn failed_upgrade_verification_restores_the_previous_versions() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(npm_detection(Some("0.145.0"), Some("7.0.0"))),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_detection(Some(PACKAGE_VERSION), None)),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(npm_detection(Some("0.145.0"), Some("7.0.0"))),
    ]);
    let inspector = executor.clone();
    let installer = installer(executor);

    let error = installer
        .upgrade(AgentUpgradeRequest::new(
            "upgrade.codex.rollback",
            AGENT_ID,
            "0.1.0",
            PROVIDER_VERSION,
        ))
        .expect_err("failed verification must fail the upgrade");

    assert_eq!(error.code(), "provider_installation_verification_failed");
    let commands = inspector.commands();
    assert_eq!(commands.len(), 5);
    assert!(commands[3]
        .args
        .contains(&format!("{PACKAGE_ID}@0.145.0")));
    assert!(commands[3].args.contains(&"openai@7.0.0".to_string()));
}

#[test]
fn descriptors_reject_tags_ranges_invalid_names_and_duplicates() {
    let invalid_descriptors = [
        ProcessAdapterInstaller::new(
            AGENT_ID,
            INSTALLER_ID,
            PROVIDER_VERSION,
            ProcessAdapterPackage::npm(PACKAGE_ID, "latest"),
        ),
        ProcessAdapterInstaller::new(
            AGENT_ID,
            INSTALLER_ID,
            PROVIDER_VERSION,
            ProcessAdapterPackage::npm(PACKAGE_ID, "^0.146.0"),
        ),
        ProcessAdapterInstaller::new(
            AGENT_ID,
            INSTALLER_ID,
            PROVIDER_VERSION,
            ProcessAdapterPackage::npm("https://registry.example/package.tgz", PACKAGE_VERSION),
        ),
        ProcessAdapterInstaller::new(
            AGENT_ID,
            INSTALLER_ID,
            PROVIDER_VERSION,
            ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION),
        )
        .with_dependency(ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION)),
    ];

    for descriptor in invalid_descriptors {
        let package = &descriptor.packages()[0];
        let request = sdkwork_agent_kernel::AgentInstallRequest::new(
            "install.invalid-descriptor",
            AGENT_ID,
            PROVIDER_VERSION,
            AgentPackageSource::registry(
                "npm",
                package.package_id.clone(),
                package.version.clone(),
            ),
        );
        let error = descriptor
            .plan_install(&request)
            .expect_err("unsafe descriptor must be rejected before execution");
        assert_eq!(error.kind(), KernelErrorKind::ValidationError);
    }

    let invalid_python = ProcessAdapterInstaller::new(
        "agent.intelligence.hermes",
        "provider.agent.installer.hermes",
        PROVIDER_VERSION,
        ProcessAdapterPackage::pypi("hermes-agent", "latest"),
    );
    let error = invalid_python
        .detect_installation("agent.intelligence.hermes")
        .expect_err("PyPI tags are not exact versions");
    assert_eq!(error.kind(), KernelErrorKind::ValidationError);
}

#[test]
fn python_detection_is_single_process_structured_and_fails_closed() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(python_detection(Some("0.19.0"))),
        ProcessAdapterCommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "credential=must-not-leak".to_string(),
            timed_out: false,
        },
    ]);
    let inspector = executor.clone();
    let installer = ProcessAdapterInstaller::new(
        "agent.intelligence.hermes",
        "provider.agent.installer.hermes",
        PROVIDER_VERSION,
        ProcessAdapterPackage::pypi("hermes-agent", "0.19.0"),
    )
    .with_executor(Arc::new(executor));

    let installed = installer
        .detect_installation("agent.intelligence.hermes")
        .expect("Python metadata is detected");
    assert!(installed.is_installed());
    let command = &inspector.commands()[0];
    assert!(command.args.contains(&"-c".to_string()));
    assert_eq!(command.timeout, Some(Duration::from_secs(30)));

    let error = installer
        .detect_installation("agent.intelligence.hermes")
        .expect_err("Python probe failures must not become missing packages");
    assert_eq!(error.code(), "provider_installation_detection_failed");
    assert!(!error.message().contains("must-not-leak"));
}

#[test]
fn python_installs_are_non_interactive_and_wheel_only() {
    let executor = FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput::success(python_detection(None)),
        ProcessAdapterCommandOutput::success(""),
        ProcessAdapterCommandOutput::success(python_detection(Some("0.19.0"))),
    ]);
    let inspector = executor.clone();
    let installer = ProcessAdapterInstaller::new(
        "agent.intelligence.hermes",
        "provider.agent.installer.hermes",
        PROVIDER_VERSION,
        ProcessAdapterPackage::pypi("hermes-agent", "0.19.0"),
    )
    .with_executor(Arc::new(executor));
    installer
        .install(sdkwork_agent_kernel::AgentInstallRequest::new(
            "install.hermes.secure",
            "agent.intelligence.hermes",
            PROVIDER_VERSION,
            AgentPackageSource::registry("pypi", "hermes-agent", "0.19.0"),
        ))
        .expect("wheel-backed Python provider installs");

    let command = &inspector.commands()[1];
    assert!(command.args.contains(&"--no-input".to_string()));
    assert!(command.args.contains(&"--only-binary=:all:".to_string()));
}

#[test]
fn npm_detection_fails_closed_on_infrastructure_errors() {
    let installer = installer(FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput {
            exit_code: 1,
            stdout: r#"{"error":{"code":"EACCES","summary":"credential=must-not-leak"}}"#
                .to_string(),
            stderr: "credential=must-not-leak".to_string(),
            timed_out: false,
        },
        ProcessAdapterCommandOutput {
            exit_code: 1,
            stdout: "{}".to_string(),
            stderr: "credential=must-not-leak".to_string(),
            timed_out: false,
        },
    ]));

    let coded_error = installer
        .detect_installation(AGENT_ID)
        .expect_err("npm infrastructure errors must fail closed");
    assert_eq!(coded_error.code(), "provider_installation_detection_failed");
    assert!(!coded_error.message().contains("must-not-leak"));
    let uncoded_error = installer
        .detect_installation(AGENT_ID)
        .expect_err("uncoded npm failures must fail closed");
    assert_eq!(uncoded_error.code(), "provider_installation_detection_failed");
    assert!(!uncoded_error.message().contains("must-not-leak"));
}

#[test]
fn npm_eleven_empty_absence_payload_is_not_an_infrastructure_failure() {
    let installer = installer(FakeCommandExecutor::with_outputs(vec![
        ProcessAdapterCommandOutput {
            exit_code: 1,
            stdout: "{}".to_string(),
            stderr: String::new(),
            timed_out: false,
        },
    ]));

    let detection = installer
        .detect_installation(AGENT_ID)
        .expect("npm 11 empty package query means not installed");
    assert!(!detection.is_installed());
    assert!(!detection.is_degraded());
}

#[test]
fn rejects_unsafe_npm_install_roots() {
    let empty = ProcessAdapterInstaller::new(
        AGENT_ID,
        INSTALLER_ID,
        PROVIDER_VERSION,
        ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION),
    )
    .with_install_root(PathBuf::new());
    assert_eq!(
        empty
            .detect_installation(AGENT_ID)
            .expect_err("empty install roots are unsafe")
            .kind(),
        KernelErrorKind::ValidationError
    );

    let filesystem_root = std::path::absolute(".")
        .expect("current directory")
        .ancestors()
        .last()
        .expect("filesystem root")
        .to_path_buf();
    let root = ProcessAdapterInstaller::new(
        AGENT_ID,
        INSTALLER_ID,
        PROVIDER_VERSION,
        ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION),
    )
    .with_install_root(filesystem_root);
    assert_eq!(
        root.detect_installation(AGENT_ID)
            .expect_err("filesystem roots are unsafe")
            .kind(),
        KernelErrorKind::ValidationError
    );
}

#[derive(Clone, Default)]
struct ConcurrentExecutor {
    installed: Arc<AtomicBool>,
    mutation_calls: Arc<AtomicUsize>,
    active_mutations: Arc<AtomicUsize>,
    max_active_mutations: Arc<AtomicUsize>,
}

impl ProcessAdapterCommandExecutor for ConcurrentExecutor {
    fn execute(
        &self,
        command: &ProcessAdapterCommand,
    ) -> sdkwork_agent_kernel::KernelResult<ProcessAdapterCommandOutput> {
        if command.args.iter().any(|argument| argument == "list") {
            let output = if self.installed.load(Ordering::SeqCst) {
                npm_detection(Some(PACKAGE_VERSION), Some("7.1.0"))
            } else {
                npm_detection(None, None)
            };
            return Ok(ProcessAdapterCommandOutput::success(output));
        }

        if command.args.iter().any(|argument| argument == "install") {
            self.mutation_calls.fetch_add(1, Ordering::SeqCst);
            let active = self.active_mutations.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_active_mutations.fetch_max(active, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(100));
            self.installed.store(true, Ordering::SeqCst);
            self.active_mutations.fetch_sub(1, Ordering::SeqCst);
            return Ok(ProcessAdapterCommandOutput::success(""));
        }

        Ok(ProcessAdapterCommandOutput::success(""))
    }
}

fn concurrent_installer(executor: ConcurrentExecutor) -> ProcessAdapterInstaller {
    ProcessAdapterInstaller::new(
        AGENT_ID,
        INSTALLER_ID,
        PROVIDER_VERSION,
        ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION),
    )
    .with_dependency(ProcessAdapterPackage::npm("openai", "7.1.0"))
    .with_install_root(PathBuf::from("provider-runtime-concurrency-contract"))
    .with_executor(Arc::new(executor))
}

#[test]
fn concurrent_installs_are_serialized_and_idempotent() {
    let executor = ConcurrentExecutor::default();
    let first = concurrent_installer(executor.clone());
    let second = concurrent_installer(executor.clone());
    let first_thread = thread::spawn(move || first.install(install_request()));
    let second_thread = thread::spawn(move || second.install(install_request()));

    first_thread
        .join()
        .expect("first installer thread")
        .expect("first install succeeds");
    second_thread
        .join()
        .expect("second installer thread")
        .expect("second install succeeds");

    assert_eq!(executor.mutation_calls.load(Ordering::SeqCst), 1);
    assert_eq!(executor.max_active_mutations.load(Ordering::SeqCst), 1);
}

#[derive(Clone)]
struct ConcurrentDetectionExecutor {
    start: Arc<Barrier>,
    active_detections: Arc<AtomicUsize>,
    max_active_detections: Arc<AtomicUsize>,
}

impl ProcessAdapterCommandExecutor for ConcurrentDetectionExecutor {
    fn execute(
        &self,
        command: &ProcessAdapterCommand,
    ) -> sdkwork_agent_kernel::KernelResult<ProcessAdapterCommandOutput> {
        assert!(command.args.iter().any(|argument| argument == "list"));
        self.start.wait();
        let active = self.active_detections.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active_detections
            .fetch_max(active, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(100));
        self.active_detections.fetch_sub(1, Ordering::SeqCst);
        Ok(ProcessAdapterCommandOutput::success(npm_detection(
            Some(PACKAGE_VERSION),
            Some("7.1.0"),
        )))
    }
}

#[test]
fn concurrent_detections_share_the_runtime_read_lock() {
    let executor = ConcurrentDetectionExecutor {
        start: Arc::new(Barrier::new(2)),
        active_detections: Arc::new(AtomicUsize::new(0)),
        max_active_detections: Arc::new(AtomicUsize::new(0)),
    };
    let first = ProcessAdapterInstaller::new(
        AGENT_ID,
        INSTALLER_ID,
        PROVIDER_VERSION,
        ProcessAdapterPackage::npm(PACKAGE_ID, PACKAGE_VERSION),
    )
    .with_dependency(ProcessAdapterPackage::npm("openai", "7.1.0"))
    .with_install_root(PathBuf::from("provider-runtime-detection-concurrency-contract"))
    .with_executor(Arc::new(executor.clone()));
    let second = first.clone();
    let first_thread = thread::spawn(move || first.detect_installation(AGENT_ID));
    let second_thread = thread::spawn(move || second.detect_installation(AGENT_ID));

    first_thread
        .join()
        .expect("first detection thread")
        .expect("first detection succeeds");
    second_thread
        .join()
        .expect("second detection thread")
        .expect("second detection succeeds");

    assert_eq!(executor.max_active_detections.load(Ordering::SeqCst), 2);
}

#[test]
fn install_and_upgrade_plans_only_claim_steps_the_installer_executes() {
    let installer = installer(FakeCommandExecutor::default());
    let install = installer
        .plan_install(&install_request())
        .expect("install plan");
    assert!(!install
        .steps
        .iter()
        .any(|step| step.step_id == "step.register_provider"));
    assert_eq!(install.steps.last().expect("post verification").kind,
        sdkwork_agent_kernel::AgentInstallStepKind::VerifyPackage);

    let upgrade_request =
        AgentUpgradeRequest::new("upgrade.codex.plan", AGENT_ID, "0.1.0", PROVIDER_VERSION);
    let upgrade = installer
        .plan_upgrade(&upgrade_request)
        .expect("upgrade plan");
    assert!(!upgrade
        .steps
        .iter()
        .any(|step| step.kind == sdkwork_agent_kernel::AgentInstallStepKind::BackupCurrentVersion));

    let rollback_upgrade = installer
        .plan_upgrade(&upgrade_request.with_rollback_required())
        .expect("rollback-aware upgrade plan");
    assert!(rollback_upgrade
        .steps
        .iter()
        .any(|step| step.kind == sdkwork_agent_kernel::AgentInstallStepKind::BackupCurrentVersion));
}

#[test]
fn system_executor_enforces_timeout_across_child_processes() {
    #[cfg(windows)]
    let command = ProcessAdapterCommand::new(
        "cmd",
        vec![
            "/D".to_string(),
            "/S".to_string(),
            "/C".to_string(),
            "ping -n 30 127.0.0.1 >NUL".to_string(),
        ],
    )
    .with_timeout(Duration::from_millis(100));
    #[cfg(unix)]
    let command = ProcessAdapterCommand::new(
        "/bin/sh",
        vec!["-c".to_string(), "sleep 30 & wait".to_string()],
    )
    .with_timeout(Duration::from_millis(100));

    let started = Instant::now();
    let output = SystemProcessAdapterCommandExecutor::default()
        .execute(&command)
        .expect("timed command returns a structured output");

    assert!(output.timed_out);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn system_executor_bounds_captured_output() {
    #[cfg(windows)]
    let command = ProcessAdapterCommand::new(
        "powershell",
        vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-Command".to_string(),
            "[Console]::Out.Write('x' * 70000)".to_string(),
        ],
    )
    .with_timeout(Duration::from_secs(10));
    #[cfg(unix)]
    let command = ProcessAdapterCommand::new(
        "/bin/sh",
        vec![
            "-c".to_string(),
            "head -c 70000 /dev/zero | tr '\\0' x".to_string(),
        ],
    )
    .with_timeout(Duration::from_secs(10));

    let output = SystemProcessAdapterCommandExecutor::default()
        .execute(&command)
        .expect("large output command completes");

    assert!(output.is_success());
    assert_eq!(output.stdout.len(), 64 * 1024);
}
