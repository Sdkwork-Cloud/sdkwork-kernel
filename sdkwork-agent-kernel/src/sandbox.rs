//! Sandbox Provider for secure tool execution.
//!
//! This module provides sandbox isolation for tool execution across platforms:
//! - Linux: Landlock + namespaces (seccomp)
//! - Windows: Restricted token
//! - macOS: Seatbelt (future)
//!
//! Reference: Codex CLI sandboxing implementation

use std::collections::HashMap;
use std::path::PathBuf;

/// Sandbox type for different platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxType {
    None,
    LinuxSeccomp,
    WindowsRestrictedToken,
    MacosSeatbelt,
}

impl SandboxType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LinuxSeccomp => "linux_seccomp",
            Self::WindowsRestrictedToken => "windows_restricted_token",
            Self::MacosSeatbelt => "macos_seatbelt",
        }
    }

    pub fn is_available() -> Option<SandboxType> {
        #[cfg(target_os = "linux")]
        {
            Some(SandboxType::LinuxSeccomp)
        }
        #[cfg(target_os = "windows")]
        {
            Some(SandboxType::WindowsRestrictedToken)
        }
        #[cfg(target_os = "macos")]
        {
            Some(SandboxType::MacosSeatbelt)
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            None
        }
    }
}

/// File system access permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemPermission {
    /// No access allowed.
    None,
    /// Read-only access.
    ReadOnly,
    /// Read-write access.
    ReadWrite,
    /// Full access (read/write/execute).
    Full,
}

/// File system sandbox policy.
#[derive(Debug, Clone)]
pub struct FileSystemSandboxPolicy {
    /// Root directory for sandbox.
    pub root: PathBuf,
    /// Path permissions (relative to root).
    pub paths: HashMap<PathBuf, FileSystemPermission>,
    /// Allow network filesystem access.
    pub allow_network_fs: bool,
    /// Allow temporary directory access.
    pub allow_temp: bool,
}

impl FileSystemSandboxPolicy {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            paths: HashMap::new(),
            allow_network_fs: false,
            allow_temp: true,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>, perm: FileSystemPermission) -> Self {
        self.paths.insert(path.into(), perm);
        self
    }

    pub fn allow_network_fs(mut self, allow: bool) -> Self {
        self.allow_network_fs = allow;
        self
    }

    pub fn allow_temp(mut self, allow: bool) -> Self {
        self.allow_temp = allow;
        self
    }

    /// Create a permissive policy (no restrictions).
    pub fn permissive() -> Self {
        Self {
            root: PathBuf::from("/"),
            paths: HashMap::new(),
            allow_network_fs: true,
            allow_temp: true,
        }
    }

    /// Create a restrictive policy (minimal access).
    pub fn restrictive(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            paths: HashMap::new(),
            allow_network_fs: false,
            allow_temp: false,
        }
    }
}

/// Network access permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkPermission {
    /// No network access.
    None,
    /// Outbound connections only.
    Outbound,
    /// Full network access.
    Full,
}

/// Network sandbox policy.
#[derive(Debug, Clone)]
pub struct NetworkSandboxPolicy {
    /// Network permission level.
    pub permission: NetworkPermission,
    /// Allowed hosts (for outbound).
    pub allowed_hosts: Vec<String>,
    /// Allowed ports.
    pub allowed_ports: Vec<u16>,
}

impl NetworkSandboxPolicy {
    pub fn new(permission: NetworkPermission) -> Self {
        Self {
            permission,
            allowed_hosts: Vec::new(),
            allowed_ports: Vec::new(),
        }
    }

    pub fn with_allowed_host(mut self, host: impl Into<String>) -> Self {
        self.allowed_hosts.push(host.into());
        self
    }

    pub fn with_allowed_port(mut self, port: u16) -> Self {
        self.allowed_ports.push(port);
        self
    }

    /// Create a no-network policy.
    pub fn no_network() -> Self {
        Self::new(NetworkPermission::None)
    }

    /// Create an outbound-only policy.
    pub fn outbound_only() -> Self {
        Self::new(NetworkPermission::Outbound)
    }

    /// Create a full network policy.
    pub fn full_network() -> Self {
        Self::new(NetworkPermission::Full)
    }
}

/// Complete sandbox policy.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    /// Sandbox type.
    pub sandbox_type: SandboxType,
    /// File system policy.
    pub file_system: FileSystemSandboxPolicy,
    /// Network policy.
    pub network: NetworkSandboxPolicy,
    /// Environment variables to set.
    pub env: HashMap<String, String>,
    /// Working directory (inside sandbox).
    pub working_dir: Option<PathBuf>,
}

impl SandboxPolicy {
    pub fn new(sandbox_type: SandboxType) -> Self {
        Self {
            sandbox_type,
            file_system: FileSystemSandboxPolicy::permissive(),
            network: NetworkSandboxPolicy::no_network(),
            env: HashMap::new(),
            working_dir: None,
        }
    }

    pub fn with_file_system(mut self, policy: FileSystemSandboxPolicy) -> Self {
        self.file_system = policy;
        self
    }

    pub fn with_network(mut self, policy: NetworkSandboxPolicy) -> Self {
        self.network = policy;
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

/// Command to execute in sandbox.
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    /// Program to execute.
    pub program: String,
    /// Arguments.
    pub args: Vec<String>,
    /// Current working directory.
    pub cwd: PathBuf,
    /// Environment variables.
    pub env: HashMap<String, String>,
}

impl SandboxCommand {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            env: HashMap::new(),
        }
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args.extend(args);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// Execution result from sandbox.
#[derive(Debug, Clone)]
pub struct SandboxExecutionResult {
    /// Exit code.
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Whether execution was killed.
    pub killed: bool,
}

impl SandboxExecutionResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.killed
    }
}

/// Sandbox provider for secure tool execution.
pub trait SandboxProvider: Send + Sync {
    /// Get sandbox type for this provider.
    fn sandbox_type(&self) -> SandboxType;

    /// Check if sandbox is available on this platform.
    fn is_available(&self) -> bool;

    /// Execute a command in sandbox.
    fn execute(
        &self,
        command: SandboxCommand,
        policy: SandboxPolicy,
    ) -> Result<SandboxExecutionResult, SandboxError>;

    /// Validate sandbox policy.
    fn validate_policy(&self, policy: &SandboxPolicy) -> Result<(), SandboxError>;

    /// Get sandbox info string.
    fn info(&self) -> String {
        format!("SandboxProvider[type={}]", self.sandbox_type().as_str())
    }
}

/// Sandbox execution error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    /// Sandbox not available on this platform.
    NotAvailable,
    /// Invalid policy.
    InvalidPolicy(String),
    /// Execution failed.
    ExecutionFailed(String),
    /// Permission denied.
    PermissionDenied(String),
    /// Timeout.
    Timeout,
    /// Unsupported feature.
    Unsupported(String),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAvailable => write!(f, "Sandbox not available on this platform"),
            Self::InvalidPolicy(msg) => write!(f, "Invalid sandbox policy: {}", msg),
            Self::ExecutionFailed(msg) => write!(f, "Sandbox execution failed: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::Timeout => write!(f, "Sandbox execution timeout"),
            Self::Unsupported(feature) => write!(f, "Unsupported feature: {}", feature),
        }
    }
}

impl std::error::Error for SandboxError {}

/// No-op sandbox provider (for testing).
#[derive(Debug, Clone)]
pub struct NoOpSandboxProvider;

impl SandboxProvider for NoOpSandboxProvider {
    fn sandbox_type(&self) -> SandboxType {
        SandboxType::None
    }

    fn is_available(&self) -> bool {
        true
    }

    fn execute(
        &self,
        command: SandboxCommand,
        _policy: SandboxPolicy,
    ) -> Result<SandboxExecutionResult, SandboxError> {
        // Execute without sandbox (for testing only!)
        let output = std::process::Command::new(&command.program)
            .args(&command.args)
            .current_dir(&command.cwd)
            .envs(&command.env)
            .output()
            .map_err(|e| SandboxError::ExecutionFailed(e.to_string()))?;

        Ok(SandboxExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            killed: false,
        })
    }

    fn validate_policy(&self, _policy: &SandboxPolicy) -> Result<(), SandboxError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_type_is_available() {
        let sandbox_type = SandboxType::is_available();
        #[cfg(target_os = "linux")]
        assert_eq!(sandbox_type, Some(SandboxType::LinuxSeccomp));
        #[cfg(target_os = "windows")]
        assert_eq!(sandbox_type, Some(SandboxType::WindowsRestrictedToken));
        #[cfg(target_os = "macos")]
        assert_eq!(sandbox_type, Some(SandboxType::MacosSeatbelt));
    }

    #[test]
    fn test_sandbox_type_as_str() {
        assert_eq!(SandboxType::None.as_str(), "none");
        assert_eq!(SandboxType::LinuxSeccomp.as_str(), "linux_seccomp");
        assert_eq!(SandboxType::WindowsRestrictedToken.as_str(), "windows_restricted_token");
    }

    #[test]
    fn test_file_system_sandbox_policy_new() {
        let policy = FileSystemSandboxPolicy::new("/tmp/sandbox");
        assert_eq!(policy.root, PathBuf::from("/tmp/sandbox"));
        assert!(policy.paths.is_empty());
        assert!(!policy.allow_network_fs);
        assert!(policy.allow_temp);
    }

    #[test]
    fn test_file_system_sandbox_policy_with_path() {
        let policy = FileSystemSandboxPolicy::new("/tmp/sandbox")
            .with_path("workspace", FileSystemPermission::ReadWrite);

        assert_eq!(policy.paths.len(), 1);
        assert_eq!(
            policy.paths.get(&PathBuf::from("workspace")),
            Some(&FileSystemPermission::ReadWrite)
        );
    }

    #[test]
    fn test_file_system_sandbox_policy_permissive() {
        let policy = FileSystemSandboxPolicy::permissive();
        assert!(policy.allow_network_fs);
        assert!(policy.allow_temp);
    }

    #[test]
    fn test_file_system_sandbox_policy_restrictive() {
        let policy = FileSystemSandboxPolicy::restrictive("/tmp/sandbox");
        assert!(!policy.allow_network_fs);
        assert!(!policy.allow_temp);
    }

    #[test]
    fn test_network_sandbox_policy_new() {
        let policy = NetworkSandboxPolicy::new(NetworkPermission::Outbound);
        assert_eq!(policy.permission, NetworkPermission::Outbound);
        assert!(policy.allowed_hosts.is_empty());
        assert!(policy.allowed_ports.is_empty());
    }

    #[test]
    fn test_network_sandbox_policy_with_allowed() {
        let policy = NetworkSandboxPolicy::outbound_only()
            .with_allowed_host("api.example.com")
            .with_allowed_port(443);

        assert_eq!(policy.allowed_hosts, vec!["api.example.com"]);
        assert_eq!(policy.allowed_ports, vec![443]);
    }

    #[test]
    fn test_sandbox_policy_new() {
        let policy = SandboxPolicy::new(SandboxType::LinuxSeccomp);
        assert_eq!(policy.sandbox_type, SandboxType::LinuxSeccomp);
        assert!(policy.env.is_empty());
        assert!(policy.working_dir.is_none());
    }

    #[test]
    fn test_sandbox_policy_with_env() {
        let policy = SandboxPolicy::new(SandboxType::None)
            .with_env("PATH", "/usr/bin")
            .with_env("HOME", "/home/user");

        assert_eq!(policy.env.get("PATH"), Some(&"/usr/bin".to_string()));
        assert_eq!(policy.env.get("HOME"), Some(&"/home/user".to_string()));
    }

    #[test]
    fn test_sandbox_command_new() {
        let cmd = SandboxCommand::new("ls");
        assert_eq!(cmd.program, "ls");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_sandbox_command_with_args() {
        let cmd = SandboxCommand::new("ls")
            .with_arg("-l")
            .with_arg("-a");

        assert_eq!(cmd.args, vec!["-l", "-a"]);
    }

    #[test]
    fn test_sandbox_execution_result_success() {
        let result = SandboxExecutionResult {
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: String::new(),
            killed: false,
        };
        assert!(result.success());
    }

    #[test]
    fn test_sandbox_execution_result_failure() {
        let result = SandboxExecutionResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            killed: false,
        };
        assert!(!result.success());
    }

    #[test]
    fn test_no_op_sandbox_provider() {
        let provider = NoOpSandboxProvider;
        assert_eq!(provider.sandbox_type(), SandboxType::None);
        assert!(provider.is_available());
    }

    #[test]
    fn test_sandbox_error_display() {
        assert_eq!(
            SandboxError::NotAvailable.to_string(),
            "Sandbox not available on this platform"
        );
        assert_eq!(
            SandboxError::InvalidPolicy("test".to_string()).to_string(),
            "Invalid sandbox policy: test"
        );
    }
}