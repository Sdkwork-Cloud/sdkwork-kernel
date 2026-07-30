use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, OnceLock, RwLock, Weak},
    thread,
    time::{Duration, Instant},
};

use sdkwork_agent_kernel::{
    AgentConfigField, AgentConfigSection, AgentConfigSectionKind, AgentConfigValue,
    AgentConfigValueKind, AgentConfiguration, AgentConfigurationProvider, AgentConfigurationSpec,
    AgentConfigurationValidation, AgentInstallPlan, AgentInstallReport, AgentInstallRequest,
    AgentInstallStep, AgentInstallStepKind, AgentInstallation, AgentInstallationDependency,
    AgentInstaller, AgentPackageSource, AgentUninstallPlan, AgentUninstallReport,
    AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport, AgentUpgradeRequest, KernelError,
    KernelEventRedaction, KernelResult, PolicyCategory, ProviderHealth, ProviderManifest,
};
use semver::Version;
use serde_json::Value;

const PROVIDER_RUNTIME_ROOT_ENV: &str = "SDKWORK_AGENT_PROVIDER_RUNTIME_ROOT";
const PYTHON_BINARY_ENV: &str = "SDKWORK_AGENT_PYTHON_BINARY";
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const DETECTION_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const HEALTH_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const PACKAGE_MUTATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const OPERATION_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_CAPTURE_BYTES: usize = 64 * 1024;
const MAX_MANAGED_PACKAGES: usize = 32;
const PYTHON_METADATA_PROBE: &str = r#"import importlib.metadata as metadata
import json
import sys

versions = {}
for package in sys.argv[1:]:
    try:
        versions[package] = metadata.version(package)
    except metadata.PackageNotFoundError:
        versions[package] = None
print(json.dumps(versions))
"#;

static OPERATION_LOCKS: OnceLock<Mutex<HashMap<String, Weak<RwLock<()>>>>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessAdapterCommand {
    pub program: String,
    pub args: Vec<String>,
    pub timeout: Option<Duration>,
}

impl ProcessAdapterCommand {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProcessAdapterCommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl fmt::Debug for ProcessAdapterCommandOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProcessAdapterCommandOutput")
            .field("exit_code", &self.exit_code)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("timed_out", &self.timed_out)
            .finish()
    }
}

impl ProcessAdapterCommandOutput {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
            timed_out: false,
        }
    }

    pub fn failure(exit_code: i32, stderr: impl Into<String>) -> Self {
        Self {
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
            timed_out: false,
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

pub trait ProcessAdapterCommandExecutor: Send + Sync {
    fn execute(&self, command: &ProcessAdapterCommand)
        -> KernelResult<ProcessAdapterCommandOutput>;
}

#[derive(Debug, Clone)]
pub struct SystemProcessAdapterCommandExecutor {
    timeout: Duration,
}

impl Default for SystemProcessAdapterCommandExecutor {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_COMMAND_TIMEOUT,
        }
    }
}

impl SystemProcessAdapterCommandExecutor {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl ProcessAdapterCommandExecutor for SystemProcessAdapterCommandExecutor {
    fn execute(
        &self,
        command: &ProcessAdapterCommand,
    ) -> KernelResult<ProcessAdapterCommandOutput> {
        let mut process = system_process_command(command)?;
        process
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            process.process_group(0);
        }
        let mut child = process.spawn().map_err(|_| {
            KernelError::provider_error(
                "provider_installer_command_unavailable",
                "provider installer command could not start",
            )
        })?;
        let process_tree = ProcessTreeGuard::attach(&child);

        let stdout = child
            .stdout
            .take()
            .expect("piped provider installer stdout");
        let stderr = child
            .stderr
            .take()
            .expect("piped provider installer stderr");
        let stdout_reader = BoundedReader::spawn(stdout);
        let stderr_reader = BoundedReader::spawn(stderr);
        let timeout = command.timeout.unwrap_or(self.timeout);
        let started = Instant::now();
        let (status, timed_out) = loop {
            if let Some(status) = child.try_wait().map_err(|error| KernelError::Internal {
                message: format!("provider installer command wait failed: {error}"),
            })? {
                break (status, false);
            }
            if started.elapsed() >= timeout {
                process_tree.terminate(&mut child);
                let status = child.wait().map_err(|error| KernelError::Internal {
                    message: format!("provider installer command termination failed: {error}"),
                })?;
                break (status, true);
            }
            thread::sleep(Duration::from_millis(25));
        };

        let drain_started = Instant::now();
        while (!stdout_reader.is_finished() || !stderr_reader.is_finished())
            && drain_started.elapsed() < OUTPUT_DRAIN_TIMEOUT
        {
            thread::sleep(Duration::from_millis(10));
        }
        if !stdout_reader.is_finished() || !stderr_reader.is_finished() {
            process_tree.terminate(&mut child);
        }
        let stdout = stdout_reader.finish()?;
        let stderr = stderr_reader.finish()?;

        Ok(ProcessAdapterCommandOutput {
            exit_code: status.code().unwrap_or(-1),
            stdout,
            stderr,
            timed_out,
        })
    }
}

fn system_process_command(command: &ProcessAdapterCommand) -> KernelResult<Command> {
    #[cfg(windows)]
    if command.program.eq_ignore_ascii_case("npm") {
        let (node_binary, npm_cli) = resolve_windows_npm_runtime()?;
        let mut process = Command::new(node_binary);
        process.arg(npm_cli).args(&command.args);
        return Ok(process);
    }

    let mut process = Command::new(&command.program);
    process.args(&command.args);
    Ok(process)
}

#[cfg(windows)]
fn resolve_windows_npm_runtime() -> KernelResult<(PathBuf, PathBuf)> {
    let path = std::env::var_os("PATH").ok_or_else(|| {
        KernelError::provider_error(
            "provider_installer_command_unavailable",
            "Node.js and the npm CLI could not be resolved",
        )
    })?;
    let mut node_binary = None;
    let mut npm_cli = None;
    for directory in std::env::split_paths(&path) {
        let node_candidate = directory.join("node.exe");
        let npm_candidate = directory
            .join("node_modules")
            .join("npm")
            .join("bin")
            .join("npm-cli.js");
        if node_candidate.is_file() && npm_candidate.is_file() {
            return Ok((node_candidate, npm_candidate));
        }
        if node_binary.is_none() && node_candidate.is_file() {
            node_binary = Some(node_candidate);
        }
        if npm_cli.is_none() && npm_candidate.is_file() {
            npm_cli = Some(npm_candidate);
        }
    }
    match (node_binary, npm_cli) {
        (Some(node_binary), Some(npm_cli)) => Ok((node_binary, npm_cli)),
        _ => Err(KernelError::provider_error(
            "provider_installer_command_unavailable",
            "Node.js and the npm CLI could not be resolved",
        )),
    }
}

struct BoundedReader {
    captured: Arc<Mutex<Vec<u8>>>,
    handle: thread::JoinHandle<()>,
}

impl BoundedReader {
    fn spawn(mut reader: impl Read + Send + 'static) -> Self {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::clone(&captured);
        let handle = thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                let Ok(count) = reader.read(&mut buffer) else {
                    break;
                };
                if count == 0 {
                    break;
                }
                let Ok(mut captured) = writer.lock() else {
                    break;
                };
                let remaining = MAX_CAPTURE_BYTES.saturating_sub(captured.len());
                captured.extend_from_slice(&buffer[..count.min(remaining)]);
            }
        });
        Self { captured, handle }
    }

    fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    fn finish(self) -> KernelResult<String> {
        if self.handle.is_finished() {
            self.handle.join().map_err(|_| KernelError::Internal {
                message: "provider installer output reader failed".to_string(),
            })?;
        }
        let captured = self.captured.lock().map_err(|_| KernelError::Internal {
            message: "provider installer output capture failed".to_string(),
        })?;
        Ok(String::from_utf8_lossy(&captured).into_owned())
    }
}

struct ProcessTreeGuard {
    #[cfg(windows)]
    job: Option<WindowsProcessJob>,
}

impl ProcessTreeGuard {
    fn attach(child: &Child) -> Self {
        Self {
            #[cfg(windows)]
            job: WindowsProcessJob::attach(child),
        }
    }

    fn terminate(&self, child: &mut Child) {
        let process_id = child.id().to_string();
        #[cfg(windows)]
        if let Some(job) = &self.job {
            job.terminate();
        } else {
            // taskkill can block for several seconds, so keep the fallback asynchronous.
            let _ = Command::new("taskkill")
                .args(["/PID", &process_id, "/T", "/F"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(unix)]
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{process_id}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.kill();
    }
}

#[cfg(windows)]
struct WindowsProcessJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl WindowsProcessJob {
    fn attach(child: &Child) -> Option<Self> {
        use std::{mem::size_of, os::windows::io::AsRawHandle, ptr};
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
        };

        // The handle is owned by this guard and closed in Drop. Every failure path closes it here.
        unsafe {
            let handle = CreateJobObjectW(ptr::null(), ptr::null());
            if handle.is_null() {
                return None;
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            let assigned = configured != 0
                && AssignProcessToJobObject(handle, child.as_raw_handle().cast()) != 0;
            if !assigned {
                CloseHandle(handle);
                return None;
            }
            Some(Self { handle })
        }
    }

    fn terminate(&self) {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;

        // The job handle remains valid for the guard lifetime.
        unsafe {
            TerminateJobObject(self.handle, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsProcessJob {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;

        // Closing a KILL_ON_JOB_CLOSE job also cleans up children left after normal parent exit.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessAdapterPackageManager {
    Npm,
    PythonPip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessAdapterPackage {
    pub registry_id: String,
    pub package_id: String,
    pub version: String,
    pub package_manager: ProcessAdapterPackageManager,
}

impl ProcessAdapterPackage {
    pub fn npm(package_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            registry_id: "npm".to_string(),
            package_id: package_id.into(),
            version: version.into(),
            package_manager: ProcessAdapterPackageManager::Npm,
        }
    }

    pub fn pypi(package_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            registry_id: "pypi".to_string(),
            package_id: package_id.into(),
            version: version.into(),
            package_manager: ProcessAdapterPackageManager::PythonPip,
        }
    }

    fn exact_spec(&self) -> String {
        match self.package_manager {
            ProcessAdapterPackageManager::Npm => {
                format!("{}@{}", self.package_id, self.version)
            }
            ProcessAdapterPackageManager::PythonPip => {
                format!("{}=={}", self.package_id, self.version)
            }
        }
    }

    fn validate(&self) -> KernelResult<()> {
        match self.package_manager {
            ProcessAdapterPackageManager::Npm => {
                if !is_valid_npm_package_id(&self.package_id) {
                    return Err(KernelError::validation(
                        "npm package id must be a canonical registry package name",
                    ));
                }
                if Version::parse(&self.version).is_err() {
                    return Err(KernelError::validation(
                        "npm package version must be an exact semantic version",
                    ));
                }
            }
            ProcessAdapterPackageManager::PythonPip => {
                if !is_valid_pypi_package_id(&self.package_id) {
                    return Err(KernelError::validation(
                        "PyPI package id must be a canonical distribution name",
                    ));
                }
                if !is_exact_python_version(&self.version) {
                    return Err(KernelError::validation(
                        "PyPI package version must be an exact PEP 440-style version",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn is_valid_npm_package_id(package_id: &str) -> bool {
    if package_id.is_empty() || package_id.len() > 214 || package_id.trim() != package_id {
        return false;
    }
    if let Some(scoped) = package_id.strip_prefix('@') {
        let Some((scope, name)) = scoped.split_once('/') else {
            return false;
        };
        return !name.contains('/') && is_valid_npm_package_segment(scope) && is_valid_npm_package_segment(name);
    }
    !package_id.contains('/') && is_valid_npm_package_segment(package_id)
}

fn is_valid_npm_package_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_' | '.' | '~')
        })
}

fn is_valid_pypi_package_id(package_id: &str) -> bool {
    if package_id.is_empty() || package_id.len() > 200 || package_id.trim() != package_id {
        return false;
    }
    let mut characters = package_id.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let Some(last) = package_id.chars().last() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && last.is_ascii_alphanumeric()
        && package_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn is_exact_python_version(version: &str) -> bool {
    if version.is_empty()
        || version.len() > 128
        || version.trim() != version
        || !version.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '-' | '_' | '+' | '!')
        })
    {
        return false;
    }
    let without_prefix = version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix('V'))
        .unwrap_or(version);
    let release = match without_prefix.split_once('!') {
        Some((epoch, release)) if !epoch.is_empty() && epoch.chars().all(|c| c.is_ascii_digit()) => {
            release
        }
        Some(_) => return false,
        None => without_prefix,
    };
    release
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
}

fn normalized_package_id(package: &ProcessAdapterPackage) -> String {
    match package.package_manager {
        ProcessAdapterPackageManager::Npm => package.package_id.clone(),
        ProcessAdapterPackageManager::PythonPip => package
            .package_id
            .chars()
            .map(|character| match character {
                '.' | '_' => '-',
                _ => character.to_ascii_lowercase(),
            })
            .collect(),
    }
}

/// Installer for npm and PyPI-backed process-adapter providers.
#[derive(Clone)]
pub struct ProcessAdapterInstaller {
    agent_id: String,
    provider_id: String,
    provider_version: String,
    packages: Vec<ProcessAdapterPackage>,
    install_root: Option<PathBuf>,
    install_scripts_enabled: bool,
    executor: Arc<dyn ProcessAdapterCommandExecutor>,
}

impl fmt::Debug for ProcessAdapterInstaller {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProcessAdapterInstaller")
            .field("agent_id", &self.agent_id)
            .field("provider_id", &self.provider_id)
            .field("provider_version", &self.provider_version)
            .field("packages", &self.packages)
            .field("install_root_configured", &self.install_root.is_some())
            .field("install_scripts_enabled", &self.install_scripts_enabled)
            .finish_non_exhaustive()
    }
}

impl ProcessAdapterInstaller {
    pub fn new(
        agent_id: impl Into<String>,
        provider_id: impl Into<String>,
        provider_version: impl Into<String>,
        primary_package: ProcessAdapterPackage,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            provider_id: provider_id.into(),
            provider_version: provider_version.into(),
            packages: vec![primary_package],
            install_root: None,
            install_scripts_enabled: false,
            executor: Arc::new(SystemProcessAdapterCommandExecutor::default()),
        }
    }

    pub fn with_dependency(mut self, package: ProcessAdapterPackage) -> Self {
        self.packages.push(package);
        self
    }

    pub fn with_install_root(mut self, install_root: impl Into<PathBuf>) -> Self {
        self.install_root = Some(install_root.into());
        self
    }

    pub fn with_install_scripts(mut self) -> Self {
        self.install_scripts_enabled = true;
        self
    }

    pub fn with_executor(mut self, executor: Arc<dyn ProcessAdapterCommandExecutor>) -> Self {
        self.executor = executor;
        self
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn provider_version(&self) -> &str {
        &self.provider_version
    }

    pub fn packages(&self) -> &[ProcessAdapterPackage] {
        &self.packages
    }

    pub fn install_scripts_enabled(&self) -> bool {
        self.install_scripts_enabled
    }

    fn validate_agent_id(&self, agent_id: &str) -> KernelResult<()> {
        if agent_id == self.agent_id {
            return Ok(());
        }
        Err(KernelError::validation(format!(
            "{} can only manage {}; received {agent_id}",
            self.provider_id, self.agent_id
        )))
    }

    fn validate_descriptor(&self) -> KernelResult<ProcessAdapterPackageManager> {
        if self.agent_id.trim().is_empty()
            || self.provider_id.trim().is_empty()
            || Version::parse(&self.provider_version).is_err()
        {
            return Err(KernelError::validation(
                "provider installer agent id, provider id, and exact semantic version are required",
            ));
        }
        if self.packages.len() > MAX_MANAGED_PACKAGES {
            return Err(KernelError::validation(format!(
                "provider installer cannot manage more than {MAX_MANAGED_PACKAGES} packages"
            )));
        }
        let manager = self
            .packages
            .first()
            .ok_or_else(|| KernelError::validation("provider installer requires a package"))?
            .package_manager;
        let mut package_ids = HashSet::new();
        for package in &self.packages {
            package.validate()?;
            if package.package_manager != manager {
                return Err(KernelError::validation(
                    "one provider installer cannot mix package managers",
                ));
            }
            let normalized_id = normalized_package_id(package);
            if !package_ids.insert(normalized_id) {
                return Err(KernelError::validation(
                    "provider installer cannot manage a package more than once",
                ));
            }
        }
        Ok(manager)
    }

    fn operation_lock_key(&self) -> KernelResult<String> {
        match self.validate_descriptor()? {
            ProcessAdapterPackageManager::Npm => {
                let root = self.npm_install_root()?;
                let mut key = root.to_string_lossy().into_owned();
                if cfg!(windows) {
                    key.make_ascii_lowercase();
                }
                Ok(format!("npm:{key}"))
            }
            ProcessAdapterPackageManager::PythonPip => {
                let mut binary = self.python_binary();
                if cfg!(windows) {
                    binary.make_ascii_lowercase();
                }
                Ok(format!("pypi:{binary}"))
            }
        }
    }

    fn shared_operation_lock(&self) -> KernelResult<Arc<RwLock<()>>> {
        let key = self.operation_lock_key()?;
        let registry = OPERATION_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut registry = registry.lock().map_err(|_| {
            KernelError::provider_error(
                "provider_installer_lock_unavailable",
                "provider lifecycle coordination is unavailable",
            )
        })?;
        registry.retain(|_, lock| lock.strong_count() > 0);
        Ok(match registry.get(&key).and_then(Weak::upgrade) {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(RwLock::new(()));
                registry.insert(key, Arc::downgrade(&lock));
                lock
            }
        })
    }

    fn with_detection_lock<T>(
        &self,
        operation: impl FnOnce() -> KernelResult<T>,
    ) -> KernelResult<T> {
        let lock = self.shared_operation_lock()?;
        let started = Instant::now();
        loop {
            match lock.try_read() {
                Ok(_guard) => return operation(),
                Err(std::sync::TryLockError::WouldBlock)
                    if started.elapsed() < OPERATION_LOCK_TIMEOUT =>
                {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    return Err(KernelError::timeout(
                        "provider lifecycle coordination timed out",
                    ));
                }
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(KernelError::provider_error(
                        "provider_installer_lock_poisoned",
                        "provider lifecycle state requires host recovery",
                    ));
                }
            }
        }
    }

    fn with_mutation_lock<T>(
        &self,
        operation: impl FnOnce() -> KernelResult<T>,
    ) -> KernelResult<T> {
        let lock = self.shared_operation_lock()?;
        let started = Instant::now();
        loop {
            match lock.try_write() {
                Ok(_guard) => return operation(),
                Err(std::sync::TryLockError::WouldBlock)
                    if started.elapsed() < OPERATION_LOCK_TIMEOUT =>
                {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    return Err(KernelError::timeout(
                        "provider lifecycle coordination timed out",
                    ));
                }
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(KernelError::provider_error(
                        "provider_installer_lock_poisoned",
                        "provider lifecycle state requires host recovery",
                    ));
                }
            }
        }
    }

    fn validate_install_request(&self, request: &AgentInstallRequest) -> KernelResult<()> {
        self.validate_agent_id(&request.agent_id)?;
        self.validate_descriptor()?;
        if request.target_version != self.provider_version {
            return Err(KernelError::validation(format!(
                "provider package target version must be {}; received {}",
                self.provider_version, request.target_version
            )));
        }
        let primary = &self.packages[0];
        match &request.source {
            AgentPackageSource::Registry {
                registry_id,
                package_id,
                version,
            } if registry_id == &primary.registry_id
                && package_id == &primary.package_id
                && version == &primary.version =>
            {
                self.validate_request_configuration(request.configuration.as_ref())
            }
            _ => Err(KernelError::validation(
                "provider package source does not match the installer descriptor",
            )),
        }
    }

    fn validate_upgrade_request(&self, request: &AgentUpgradeRequest) -> KernelResult<()> {
        self.validate_agent_id(&request.agent_id)?;
        self.validate_descriptor()?;
        if request.to_version != self.provider_version {
            return Err(KernelError::validation(format!(
                "provider upgrade target version must be {}; received {}",
                self.provider_version, request.to_version
            )));
        }
        if request.from_version.trim().is_empty() {
            return Err(KernelError::validation(
                "provider upgrade source version is required",
            ));
        }
        self.validate_request_configuration(request.configuration.as_ref())
    }

    fn npm_install_root(&self) -> KernelResult<PathBuf> {
        let root = if let Some(root) = &self.install_root {
            root.clone()
        } else if let Some(root) = std::env::var_os(PROVIDER_RUNTIME_ROOT_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
        {
            root
        } else {
            find_packaged_provider_runtime_root().ok_or_else(|| {
                KernelError::provider_error(
                    "provider_install_root_missing",
                    format!("{PROVIDER_RUNTIME_ROOT_ENV} is not configured"),
                )
            })?
        };
        resolve_install_root(&root)
    }

    fn python_binary(&self) -> String {
        std::env::var(PYTHON_BINARY_ENV).unwrap_or_else(|_| {
            if cfg!(windows) {
                "python".to_string()
            } else {
                "python3".to_string()
            }
        })
    }

    fn validate_request_configuration(
        &self,
        configuration: Option<&AgentConfiguration>,
    ) -> KernelResult<()> {
        let Some(configuration) = configuration else {
            return Ok(());
        };
        if ProcessAdapterConfigurationProvider::spec_for(&self.agent_id)
            .validate(configuration)
            .is_valid()
        {
            Ok(())
        } else {
            Err(KernelError::validation(
                "provider configuration does not satisfy the installation schema",
            ))
        }
    }

    fn detect_dependencies(&self) -> KernelResult<Vec<AgentInstallationDependency>> {
        match self.validate_descriptor()? {
            ProcessAdapterPackageManager::Npm => self.detect_npm_dependencies(),
            ProcessAdapterPackageManager::PythonPip => self.detect_pypi_dependencies(),
        }
    }

    fn detect_npm_dependencies(&self) -> KernelResult<Vec<AgentInstallationDependency>> {
        let root = self.npm_install_root()?;
        let mut args = vec![
            "--prefix".to_string(),
            root.to_string_lossy().into_owned(),
            "list".to_string(),
            "--depth=0".to_string(),
            "--json".to_string(),
        ];
        args.extend(
            self.packages
                .iter()
                .map(|package| package.package_id.clone()),
        );
        let output = self.executor.execute(
            &ProcessAdapterCommand::new("npm", args).with_timeout(DETECTION_COMMAND_TIMEOUT),
        )?;
        if output.timed_out {
            return Err(KernelError::timeout(
                "provider installation detection timed out",
            ));
        }
        let payload: Value = serde_json::from_str(&output.stdout).map_err(|error| {
            KernelError::provider_error(
                "provider_installation_detection_failed",
                format!("npm installation detection returned invalid JSON: {error}"),
            )
        })?;
        let error_code = payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str);
        let expected_absence = matches!(error_code, Some("ENOENT" | "ELSPROBLEMS"));
        let empty_absence = payload
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
            && output.stderr.trim().is_empty();
        if (!output.is_success() && !expected_absence && !empty_absence)
            || error_code.is_some_and(|code| !matches!(code, "ENOENT" | "ELSPROBLEMS"))
        {
            return Err(KernelError::provider_error(
                "provider_installation_detection_failed",
                "npm installation detection failed",
            ));
        }
        let dependencies = payload.get("dependencies").and_then(Value::as_object);
        Ok(self
            .packages
            .iter()
            .map(|package| {
                let installed_version = dependencies
                    .and_then(|items| items.get(&package.package_id))
                    .and_then(|item| item.get("version"))
                    .and_then(Value::as_str);
                dependency_detection(package, installed_version)
            })
            .collect())
    }

    fn detect_pypi_dependencies(&self) -> KernelResult<Vec<AgentInstallationDependency>> {
        let mut args = vec!["-c".to_string(), PYTHON_METADATA_PROBE.to_string()];
        args.extend(
            self.packages
                .iter()
                .map(|package| package.package_id.clone()),
        );
        let output = self.executor.execute(
            &ProcessAdapterCommand::new(self.python_binary(), args)
                .with_timeout(DETECTION_COMMAND_TIMEOUT),
        )?;
        if output.timed_out {
            return Err(KernelError::timeout(
                "provider installation detection timed out",
            ));
        }
        if !output.is_success() {
            return Err(KernelError::provider_error(
                "provider_installation_detection_failed",
                format!(
                    "Python installation detection failed with exit code {}",
                    output.exit_code
                ),
            ));
        }
        let payload: Value = serde_json::from_str(&output.stdout).map_err(|error| {
            KernelError::provider_error(
                "provider_installation_detection_failed",
                format!("Python installation detection returned invalid JSON: {error}"),
            )
        })?;
        let versions = payload.as_object().ok_or_else(|| {
            KernelError::provider_error(
                "provider_installation_detection_failed",
                "Python installation detection returned an invalid payload",
            )
        })?;
        self.packages
            .iter()
            .map(|package| match versions.get(&package.package_id) {
                Some(Value::String(version)) if !version.is_empty() => {
                    Ok(dependency_detection(package, Some(version)))
                }
                Some(Value::Null) => Ok(dependency_detection(package, None)),
                _ => Err(KernelError::provider_error(
                    "provider_installation_detection_failed",
                    "Python installation detection omitted package metadata",
                )),
            })
            .collect()
    }

    fn run_package_install(&self, code: &str) -> KernelResult<()> {
        let exact_packages: Vec<String> = self
            .packages
            .iter()
            .map(ProcessAdapterPackage::exact_spec)
            .collect();
        self.run_exact_package_install(&exact_packages, code)
    }

    fn run_exact_package_install(&self, exact_packages: &[String], code: &str) -> KernelResult<()> {
        let manager = self.validate_descriptor()?;
        let command = match manager {
            ProcessAdapterPackageManager::Npm => {
                let root = self.npm_install_root()?;
                let mut args = vec![
                    "--prefix".to_string(),
                    root.to_string_lossy().into_owned(),
                    "install".to_string(),
                    "--no-audit".to_string(),
                    "--no-fund".to_string(),
                    "--omit=dev".to_string(),
                    "--save-exact".to_string(),
                ];
                if !self.install_scripts_enabled {
                    args.push("--ignore-scripts".to_string());
                }
                args.extend(exact_packages.iter().cloned());
                ProcessAdapterCommand::new("npm", args)
            }
            ProcessAdapterPackageManager::PythonPip => {
                let mut args = vec![
                    "-m".to_string(),
                    "pip".to_string(),
                    "install".to_string(),
                    "--disable-pip-version-check".to_string(),
                    "--no-input".to_string(),
                    "--only-binary=:all:".to_string(),
                ];
                args.extend(exact_packages.iter().cloned());
                ProcessAdapterCommand::new(self.python_binary(), args)
            }
        };
        self.run_checked(&command.with_timeout(PACKAGE_MUTATION_TIMEOUT), code)
    }

    fn run_package_uninstall(&self, code: &str) -> KernelResult<()> {
        let package_ids: Vec<String> = self
            .packages
            .iter()
            .rev()
            .map(|package| package.package_id.clone())
            .collect();
        self.run_package_uninstall_ids(&package_ids, code)
    }

    fn run_package_uninstall_ids(&self, package_ids: &[String], code: &str) -> KernelResult<()> {
        let manager = self.validate_descriptor()?;
        let command = match manager {
            ProcessAdapterPackageManager::Npm => {
                let root = self.npm_install_root()?;
                let mut args = vec![
                    "--prefix".to_string(),
                    root.to_string_lossy().into_owned(),
                    "uninstall".to_string(),
                    "--no-audit".to_string(),
                    "--no-fund".to_string(),
                ];
                args.extend(package_ids.iter().cloned());
                ProcessAdapterCommand::new("npm", args)
            }
            ProcessAdapterPackageManager::PythonPip => {
                let mut args = vec![
                    "-m".to_string(),
                    "pip".to_string(),
                    "uninstall".to_string(),
                    "--yes".to_string(),
                ];
                args.extend(package_ids.iter().cloned());
                ProcessAdapterCommand::new(self.python_binary(), args)
            }
        };
        self.run_checked(&command.with_timeout(PACKAGE_MUTATION_TIMEOUT), code)
    }

    fn run_checked(&self, command: &ProcessAdapterCommand, code: &str) -> KernelResult<()> {
        let output = self.executor.execute(command)?;
        if output.timed_out {
            return Err(KernelError::timeout(
                "provider package manager command timed out",
            ));
        }
        if output.is_success() {
            return Ok(());
        }
        Err(KernelError::provider_error(
            code,
            format!(
                "provider package manager command failed with exit code {}",
                output.exit_code
            ),
        ))
    }

    fn detect_installation_unlocked(&self, agent_id: &str) -> KernelResult<AgentInstallation> {
        self.validate_agent_id(agent_id)?;
        let dependencies = self.detect_dependencies()?;
        let primary_present = dependencies
            .first()
            .and_then(|dependency| dependency.installed_version.as_ref())
            .is_some();
        let any_present = dependencies
            .iter()
            .any(|dependency| dependency.installed_version.is_some());
        let complete = dependencies
            .iter()
            .all(AgentInstallationDependency::version_matches);
        let mut installation = if !any_present {
            AgentInstallation::not_installed(agent_id)
        } else if complete {
            AgentInstallation::installed(agent_id, &self.provider_version)
        } else if primary_present {
            AgentInstallation::degraded(agent_id, &self.provider_version)
        } else {
            AgentInstallation::partially_installed(agent_id)
        };
        installation.dependencies = dependencies;
        Ok(installation)
    }

    fn verify_installed_unlocked(&self) -> KernelResult<AgentInstallation> {
        let detection = self.detect_installation_unlocked(&self.agent_id)?;
        if detection.is_installed() {
            Ok(detection)
        } else {
            Err(KernelError::provider_error(
                "provider_installation_verification_failed",
                "provider dependencies did not match the exact installation descriptor",
            ))
        }
    }

    fn verify_uninstalled_unlocked(&self) -> KernelResult<()> {
        let remaining = self.detect_dependencies()?;
        if remaining
            .iter()
            .any(|dependency| dependency.installed_version.is_some())
        {
            return Err(KernelError::provider_error(
                "provider_uninstall_verification_failed",
                "one or more provider packages remain installed",
            ));
        }
        Ok(())
    }

    fn restore_dependency_snapshot(
        &self,
        snapshot: &[AgentInstallationDependency],
    ) -> KernelResult<()> {
        let manager = self.validate_descriptor()?;
        let mut restore_specs = Vec::new();
        let mut remove_ids = Vec::new();
        for dependency in snapshot {
            match &dependency.installed_version {
                Some(version) => {
                    validate_detected_version(manager, version)?;
                    let separator = match manager {
                        ProcessAdapterPackageManager::Npm => "@",
                        ProcessAdapterPackageManager::PythonPip => "==",
                    };
                    restore_specs.push(format!(
                        "{}{}{}",
                        dependency.package_id, separator, version
                    ));
                }
                None => remove_ids.push(dependency.package_id.clone()),
            }
        }
        if !restore_specs.is_empty() {
            self.run_exact_package_install(
                &restore_specs,
                "provider_package_rollback_install_failed",
            )?;
        }
        if !remove_ids.is_empty() {
            self.run_package_uninstall_ids(
                &remove_ids,
                "provider_package_rollback_uninstall_failed",
            )?;
        }
        let restored = self.detect_dependencies()?;
        if dependency_versions_match(snapshot, &restored) {
            Ok(())
        } else {
            Err(KernelError::provider_error(
                "provider_package_rollback_verification_failed",
                "provider package rollback did not restore the previous dependency state",
            ))
        }
    }

    fn compensate_failure(
        &self,
        snapshot: &[AgentInstallationDependency],
        original: KernelError,
    ) -> KernelError {
        match self.restore_dependency_snapshot(snapshot) {
            Ok(()) => original,
            Err(_) => KernelError::provider_error(
                "provider_package_rollback_failed",
                "provider package mutation failed and the previous state could not be restored",
            ),
        }
    }
}

fn resolve_install_root(root: &Path) -> KernelResult<PathBuf> {
    if root.as_os_str().is_empty() {
        return Err(KernelError::validation(
            "provider install root must not be empty",
        ));
    }
    let absolute = std::path::absolute(root).map_err(|_| {
        KernelError::provider_error(
            "provider_install_root_unavailable",
            "provider install root could not be resolved",
        )
    })?;
    let normalized = lexical_normalize(&absolute);
    if normalized.parent().is_none() {
        return Err(KernelError::validation(
            "provider install root must not be a filesystem root",
        ));
    }
    if normalized.exists() && !normalized.is_dir() {
        return Err(KernelError::validation(
            "provider install root must be a directory",
        ));
    }
    Ok(dunce::canonicalize(&normalized).unwrap_or(normalized))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

impl AgentInstaller for ProcessAdapterInstaller {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "agent_installer",
            format!("{} package lifecycle", self.agent_id),
            self.provider_version.clone(),
            vec![
                "agent.install".to_string(),
                "agent.uninstall".to_string(),
                "agent.upgrade".to_string(),
            ],
        )
    }

    fn detect_installation(&self, agent_id: &str) -> KernelResult<AgentInstallation> {
        self.validate_agent_id(agent_id)?;
        self.with_detection_lock(|| self.detect_installation_unlocked(agent_id))
    }

    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        ProcessAdapterConfigurationProvider::new(self.agent_id.clone()).configuration_spec(agent_id)
    }

    fn plan_install(&self, request: &AgentInstallRequest) -> KernelResult<AgentInstallPlan> {
        self.validate_install_request(request)?;
        Ok(AgentInstallPlan::new(
            format!("plan.{}.install", self.agent_id),
            self.agent_id.clone(),
            request.target_version.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.verify_provider_packages",
            AgentInstallStepKind::VerifyPackage,
            "detect exact provider package versions",
        ))
        .add_step(AgentInstallStep::new(
            "step.install_provider_packages",
            AgentInstallStepKind::WriteFiles,
            "install exact provider package versions",
        ))
        .add_step(AgentInstallStep::new(
            "step.verify_installed_provider_packages",
            AgentInstallStepKind::VerifyPackage,
            "verify installed provider package versions",
        ))
        .require_policy(PolicyCategory::AgentInstall))
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
        self.with_mutation_lock(|| {
            let snapshot = self.detect_dependencies()?;
            if snapshot
                .iter()
                .all(AgentInstallationDependency::version_matches)
            {
                return Ok(AgentInstallReport::installed(
                    request.request_id,
                    request.agent_id,
                    request.target_version,
                ));
            }
            if let Err(error) =
                self.run_package_install("provider_package_install_failed")
            {
                return Err(self.compensate_failure(&snapshot, error));
            }
            if let Err(error) = self.verify_installed_unlocked() {
                return Err(self.compensate_failure(&snapshot, error));
            }
            Ok(AgentInstallReport::installed(
                request.request_id,
                request.agent_id,
                request.target_version,
            ))
        })
    }

    fn plan_upgrade(&self, request: &AgentUpgradeRequest) -> KernelResult<AgentUpgradePlan> {
        self.validate_upgrade_request(request)?;
        let mut plan = AgentUpgradePlan::new(
            format!("plan.{}.upgrade", self.agent_id),
            self.agent_id.clone(),
            request.from_version.clone(),
            request.to_version.clone(),
        )
        .with_rollback_required(request.rollback_required)
        .add_step(AgentInstallStep::new(
            "step.detect_provider_version",
            AgentInstallStepKind::VerifyPackage,
            "detect current provider package versions",
        ));
        if request.rollback_required {
            plan = plan.add_step(AgentInstallStep::new(
                "step.backup_provider_version",
                AgentInstallStepKind::BackupCurrentVersion,
                "record current provider package versions for automatic rollback",
            ));
        }
        Ok(plan
        .add_step(AgentInstallStep::new(
            "step.replace_provider_packages",
            AgentInstallStepKind::ReplaceVersion,
            "replace provider packages with exact target versions",
        ))
        .add_step(AgentInstallStep::new(
            "step.verify_upgraded_provider_packages",
            AgentInstallStepKind::VerifyPackage,
            "verify upgraded provider package versions",
        ))
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
        self.with_mutation_lock(|| {
            let snapshot = self.detect_dependencies()?;
            if snapshot
                .iter()
                .all(AgentInstallationDependency::version_matches)
            {
                return Ok(AgentUpgradeReport::upgraded(
                    request.request_id,
                    request.agent_id,
                    request.from_version,
                    request.to_version,
                ));
            }
            if let Err(error) =
                self.run_package_install("provider_package_upgrade_failed")
            {
                return Err(self.compensate_failure(&snapshot, error));
            }
            if let Err(error) = self.verify_installed_unlocked() {
                return Err(self.compensate_failure(&snapshot, error));
            }
            Ok(AgentUpgradeReport::upgraded(
                request.request_id,
                request.agent_id,
                request.from_version,
                request.to_version,
            ))
        })
    }

    fn plan_uninstall(&self, request: &AgentUninstallRequest) -> KernelResult<AgentUninstallPlan> {
        self.validate_agent_id(&request.agent_id)?;
        self.validate_descriptor()?;
        if !request.preserve_data {
            return Err(KernelError::validation(
                "provider package installers do not own agent data removal",
            ));
        }
        if request.remove_configuration {
            return Err(KernelError::provider_error(
                "provider_configuration_removal_requires_host",
                "provider configuration removal must be completed by the host configuration store before package uninstall",
            ));
        }
        let plan = AgentUninstallPlan::new(
            format!("plan.{}.uninstall", self.agent_id),
            self.agent_id.clone(),
        )
        .add_step(AgentInstallStep::new(
            "step.remove_provider_packages",
            AgentInstallStepKind::RemoveFiles,
            "remove provider packages from the managed runtime",
        ))
        .add_step(AgentInstallStep::new(
            "step.verify_removed_provider_packages",
            AgentInstallStepKind::VerifyPackage,
            "verify provider packages are absent",
        ))
        .require_policy(PolicyCategory::AgentUninstall);
        Ok(plan)
    }

    fn uninstall(&self, request: AgentUninstallRequest) -> KernelResult<AgentUninstallReport> {
        self.plan_uninstall(&request)?;
        if request.dry_run {
            return Ok(AgentUninstallReport::planned(
                request.request_id,
                request.agent_id,
            ));
        }
        self.with_mutation_lock(|| {
            let snapshot = self.detect_dependencies()?;
            if snapshot
                .iter()
                .all(|dependency| dependency.installed_version.is_none())
            {
                return Ok(
                    AgentUninstallReport::uninstalled(request.request_id, request.agent_id)
                        .with_configuration_removed(false),
                );
            }
            if let Err(error) =
                self.run_package_uninstall("provider_package_uninstall_failed")
            {
                return Err(self.compensate_failure(&snapshot, error));
            }
            if let Err(error) = self.verify_uninstalled_unlocked() {
                return Err(self.compensate_failure(&snapshot, error));
            }
            Ok(
                AgentUninstallReport::uninstalled(request.request_id, request.agent_id)
                    .with_configuration_removed(false),
            )
        })
    }

    fn health(&self) -> ProviderHealth {
        let manager = match self.validate_descriptor() {
            Ok(manager) => manager,
            Err(_) => {
                return ProviderHealth {
                    status: "unhealthy".to_string(),
                }
            }
        };
        let command = match manager {
            ProcessAdapterPackageManager::Npm => {
                ProcessAdapterCommand::new("npm", vec!["--version".to_string()])
            }
            ProcessAdapterPackageManager::PythonPip => ProcessAdapterCommand::new(
                self.python_binary(),
                vec!["-m".to_string(), "pip".to_string(), "--version".to_string()],
            ),
        };
        match self
            .executor
            .execute(&command.with_timeout(HEALTH_COMMAND_TIMEOUT))
        {
            Ok(output) if output.is_success() => ProviderHealth::available(),
            _ => ProviderHealth {
                status: "degraded".to_string(),
            },
        }
    }
}

fn validate_detected_version(
    manager: ProcessAdapterPackageManager,
    version: &str,
) -> KernelResult<()> {
    let valid = match manager {
        ProcessAdapterPackageManager::Npm => Version::parse(version).is_ok(),
        ProcessAdapterPackageManager::PythonPip => is_exact_python_version(version),
    };
    if valid {
        Ok(())
    } else {
        Err(KernelError::provider_error(
            "provider_package_rollback_version_invalid",
            "detected provider package version cannot be restored safely",
        ))
    }
}

fn dependency_versions_match(
    expected: &[AgentInstallationDependency],
    actual: &[AgentInstallationDependency],
) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            expected.registry_id == actual.registry_id
                && expected.package_id == actual.package_id
                && expected.installed_version == actual.installed_version
        })
}

fn dependency_detection(
    package: &ProcessAdapterPackage,
    installed_version: Option<&str>,
) -> AgentInstallationDependency {
    match installed_version {
        Some(version) => AgentInstallationDependency::installed(
            &package.registry_id,
            &package.package_id,
            &package.version,
            version,
        ),
        None => AgentInstallationDependency::missing(
            &package.registry_id,
            &package.package_id,
            &package.version,
        ),
    }
}

fn find_packaged_provider_runtime_root() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let mut ancestors = executable.parent();
    while let Some(directory) = ancestors {
        let candidates = [
            directory.join("provider-runtime"),
            directory.join("resources").join("provider-runtime"),
            directory.join("Resources").join("provider-runtime"),
        ];
        if let Some(candidate) = candidates.into_iter().find(|path| {
            path.join("workers")
                .join("generic-ts-sdk-worker.mjs")
                .is_file()
        }) {
            return Some(candidate);
        }
        ancestors = directory.parent();
    }
    None
}

/// Minimal configuration surface for external process-adapter agents.
#[derive(Debug, Clone)]
pub struct ProcessAdapterConfigurationProvider {
    agent_id: String,
}

impl ProcessAdapterConfigurationProvider {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
        }
    }

    pub fn spec_for(agent_id: &str) -> AgentConfigurationSpec {
        AgentConfigurationSpec::new(agent_id)
            .add_section(
                AgentConfigSection::base("base", "Base").add_field(
                    AgentConfigField::text("agent.display_name", "Display name").required(),
                ),
            )
            .add_section(AgentConfigSection::llm_api_key("llm", "LLM").add_field(
                AgentConfigField::llm_api_key("llm.api_key", "Model provider API key"),
            ))
            .add_section(
                AgentConfigSection::new("runtime", "Runtime", AgentConfigSectionKind::Runtime)
                    .add_field(
                        AgentConfigField::text("runtime.external.backend", "External backend")
                            .required()
                            .with_default(AgentConfigValue::string("process_adapter")),
                    ),
            )
            .add_section(
                AgentConfigSection::new("security", "Security", AgentConfigSectionKind::Security)
                    .add_field(
                        AgentConfigField::new(
                            "security.fail_closed",
                            "Fail closed",
                            AgentConfigValueKind::String,
                        )
                        .required()
                        .with_default(AgentConfigValue::string("true"))
                        .with_redaction(KernelEventRedaction::Internal),
                    ),
            )
    }
}

impl AgentConfigurationProvider for ProcessAdapterConfigurationProvider {
    fn configuration_spec(&self, agent_id: &str) -> KernelResult<AgentConfigurationSpec> {
        if agent_id != self.agent_id {
            return Err(KernelError::CapabilityMissing {
                capability_id: agent_id.to_string(),
            });
        }

        Ok(Self::spec_for(agent_id))
    }

    fn validate_configuration(
        &self,
        configuration: &AgentConfiguration,
    ) -> KernelResult<AgentConfigurationValidation> {
        Ok(Self::spec_for(&self.agent_id).validate(configuration))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
