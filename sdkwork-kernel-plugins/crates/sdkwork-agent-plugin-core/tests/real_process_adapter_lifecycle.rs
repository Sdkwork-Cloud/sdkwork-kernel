use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use sdkwork_agent_kernel::{
    AgentInstallRequest, AgentInstaller, AgentPackageSource, AgentUninstallRequest,
    AgentUpgradeRequest,
};
use sdkwork_agent_plugin_core::{
    ProcessAdapterCommand, ProcessAdapterCommandExecutor, ProcessAdapterCommandOutput,
    ProcessAdapterInstaller, ProcessAdapterPackage, SystemProcessAdapterCommandExecutor,
};

const PROVIDER_VERSION: &str = "0.2.0";

struct NpmProviderCase {
    agent_id: &'static str,
    provider_id: &'static str,
    package_id: &'static str,
    package_version: &'static str,
    dependencies: &'static [(&'static str, &'static str)],
    install_scripts: bool,
}

const NPM_PROVIDERS: &[NpmProviderCase] = &[
    NpmProviderCase {
        agent_id: "agent.intelligence.codex",
        provider_id: "provider.agent.installer.codex",
        package_id: "@openai/codex-sdk",
        package_version: "0.146.0",
        dependencies: &[],
        install_scripts: false,
    },
    NpmProviderCase {
        agent_id: "agent.intelligence.claude-code",
        provider_id: "provider.agent.installer.claude-code",
        package_id: "@anthropic-ai/claude-agent-sdk",
        package_version: "0.3.220",
        dependencies: &[],
        install_scripts: false,
    },
    NpmProviderCase {
        agent_id: "agent.intelligence.gemini-cli",
        provider_id: "provider.agent.installer.gemini-cli",
        package_id: "@google/gemini-cli",
        package_version: "0.53.0",
        dependencies: &[],
        install_scripts: false,
    },
    NpmProviderCase {
        agent_id: "agent.intelligence.opencode",
        provider_id: "provider.agent.installer.opencode",
        package_id: "@opencode-ai/sdk",
        package_version: "1.18.9",
        dependencies: &[],
        install_scripts: false,
    },
    NpmProviderCase {
        agent_id: "agent.intelligence.openclaw",
        provider_id: "provider.agent.installer.openclaw",
        package_id: "openclaw",
        package_version: "2026.7.1-2",
        dependencies: &[("openai", "7.1.0")],
        install_scripts: true,
    },
    NpmProviderCase {
        agent_id: "agent.intelligence.mimo-code",
        provider_id: "provider.agent.installer.mimo-code",
        package_id: "@mimo-ai/sdk",
        package_version: "0.1.9",
        dependencies: &[],
        install_scripts: false,
    },
];

#[test]
#[ignore = "requires npm registry access and installs exact provider packages in temporary roots"]
fn real_npm_provider_lifecycle_reaches_exact_versions() {
    for case in NPM_PROVIDERS {
        let runtime = TemporaryRuntime::new("npm", case.agent_id);
        if case.agent_id == "agent.intelligence.codex" {
            let previous = npm_installer(
                case,
                runtime.path(),
                "0.1.0",
                "0.145.0",
                false,
            );
            previous
                .install(AgentInstallRequest::new(
                    "install.codex.previous",
                    case.agent_id,
                    "0.1.0",
                    AgentPackageSource::registry("npm", case.package_id, "0.145.0"),
                ))
                .expect("previous Codex package installs");
        }

        let installer = npm_installer(
            case,
            runtime.path(),
            PROVIDER_VERSION,
            case.package_version,
            case.install_scripts,
        );
        if case.agent_id == "agent.intelligence.codex" {
            installer
                .upgrade(AgentUpgradeRequest::new(
                    "upgrade.codex.latest",
                    case.agent_id,
                    "0.1.0",
                    PROVIDER_VERSION,
                ))
                .expect("Codex upgrades from the previous registry release");
        } else {
            installer
                .install(AgentInstallRequest::new(
                    format!("install.{}.latest", case.agent_id),
                    case.agent_id,
                    PROVIDER_VERSION,
                    AgentPackageSource::registry(
                        "npm",
                        case.package_id,
                        case.package_version,
                    ),
                ))
                .expect("provider installs from npm");
            installer
                .upgrade(AgentUpgradeRequest::new(
                    format!("upgrade.{}.idempotent", case.agent_id),
                    case.agent_id,
                    PROVIDER_VERSION,
                    PROVIDER_VERSION,
                ))
                .expect("repeat upgrade is idempotent");
        }

        let detected = installer
            .detect_installation(case.agent_id)
            .expect("provider installation is detected");
        assert!(detected.is_installed(), "{} is installed", case.agent_id);
        assert!(runtime.path().join("package-lock.json").is_file());
        installer
            .uninstall(AgentUninstallRequest::new(
                format!("uninstall.{}", case.agent_id),
                case.agent_id,
            ))
            .expect("provider uninstalls from npm");
        assert!(!installer
            .detect_installation(case.agent_id)
            .expect("uninstall is detected")
            .is_installed());
    }
}

#[test]
#[ignore = "requires PyPI access and installs Hermes into a temporary virtual environment"]
fn real_python_provider_lifecycle_isolated_in_virtual_environment() {
    let runtime = TemporaryRuntime::new("python", "agent.intelligence.hermes");
    let venv = runtime.path().join("venv");
    let status = Command::new(default_python_binary())
        .args(["-m", "venv"])
        .arg(&venv)
        .status()
        .expect("Python can create a virtual environment");
    assert!(status.success(), "Python virtual environment creation succeeds");
    let managed_python = if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    };
    let _python_guard = EnvironmentGuard::set(
        "SDKWORK_AGENT_PYTHON_BINARY",
        managed_python.as_os_str().to_owned(),
    );
    let installer = ProcessAdapterInstaller::new(
        "agent.intelligence.hermes",
        "provider.agent.installer.hermes",
        PROVIDER_VERSION,
        ProcessAdapterPackage::pypi("hermes-agent", "0.19.0"),
    )
    .with_executor(Arc::new(LifecycleEvidenceExecutor));

    installer
        .install(AgentInstallRequest::new(
            "install.hermes.latest",
            "agent.intelligence.hermes",
            PROVIDER_VERSION,
            AgentPackageSource::registry("pypi", "hermes-agent", "0.19.0"),
        ))
        .expect("Hermes installs into the managed virtual environment");
    assert!(installer
        .detect_installation("agent.intelligence.hermes")
        .expect("Hermes installation is detected")
        .is_installed());
    installer
        .upgrade(AgentUpgradeRequest::new(
            "upgrade.hermes.idempotent",
            "agent.intelligence.hermes",
            PROVIDER_VERSION,
            PROVIDER_VERSION,
        ))
        .expect("Hermes exact-version upgrade is idempotent");
    installer
        .uninstall(AgentUninstallRequest::new(
            "uninstall.hermes",
            "agent.intelligence.hermes",
        ))
        .expect("Hermes uninstalls from the managed virtual environment");
    assert!(!installer
        .detect_installation("agent.intelligence.hermes")
        .expect("Hermes uninstall is detected")
        .is_installed());
}

fn npm_installer(
    case: &NpmProviderCase,
    root: &Path,
    provider_version: &str,
    package_version: &str,
    install_scripts: bool,
) -> ProcessAdapterInstaller {
    let mut installer = ProcessAdapterInstaller::new(
        case.agent_id,
        case.provider_id,
        provider_version,
        ProcessAdapterPackage::npm(case.package_id, package_version),
    )
    .with_install_root(root)
    .with_executor(Arc::new(LifecycleEvidenceExecutor));
    for (package_id, version) in case.dependencies {
        installer = installer.with_dependency(ProcessAdapterPackage::npm(*package_id, *version));
    }
    if install_scripts {
        installer = installer.with_install_scripts();
    }
    installer
}

struct LifecycleEvidenceExecutor;

impl ProcessAdapterCommandExecutor for LifecycleEvidenceExecutor {
    fn execute(
        &self,
        command: &ProcessAdapterCommand,
    ) -> sdkwork_agent_kernel::KernelResult<ProcessAdapterCommandOutput> {
        let output = SystemProcessAdapterCommandExecutor::default().execute(command)?;
        eprintln!(
            "lifecycle command={} exit={} timed_out={} stdout_bytes={} stderr_bytes={}",
            command.program,
            output.exit_code,
            output.timed_out,
            output.stdout.len(),
            output.stderr.len()
        );
        Ok(output)
    }
}

fn default_python_binary() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

struct TemporaryRuntime {
    path: PathBuf,
}

impl TemporaryRuntime {
    fn new(manager: &str, agent_id: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after Unix epoch")
            .as_nanos();
        let safe_agent_id = agent_id.replace('.', "-");
        let path = std::env::temp_dir().join(format!(
            "sdkwork-provider-lifecycle-{manager}-{safe_agent_id}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("temporary provider runtime is created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryRuntime {
    fn drop(&mut self) {
        let temp = std::env::temp_dir();
        let safe_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("sdkwork-provider-lifecycle-"));
        if self.path.starts_with(&temp) && safe_name {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

struct EnvironmentGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvironmentGuard {
    fn set(key: &'static str, value: std::ffi::OsString) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
