use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest, SideEffectLevel};
use std::fmt::{Debug, Formatter};

pub trait HostProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn filesystem(&self, request: FilesystemRequest) -> KernelResult<FilesystemResult>;

    fn process(&self, request: ProcessRequest) -> KernelResult<ProcessResult>;

    fn network(&self, request: NetworkRequest) -> KernelResult<NetworkResult>;

    fn resolve_secret(&self, secret_ref: SecretRef) -> KernelResult<SecretValue>;

    fn storage(&self, _request: StorageRequest) -> KernelResult<StorageResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.storage".to_string(),
        })
    }

    fn time(&self, _request: TimeRequest) -> KernelResult<TimeResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.time".to_string(),
        })
    }

    fn environment(&self, _request: EnvironmentRequest) -> KernelResult<EnvironmentResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.environment".to_string(),
        })
    }

    fn executor(&self, _request: ExecutorRequest) -> KernelResult<ExecutorResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "host.executor".to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPathPolicy {
    allowed_roots: Vec<String>,
}

impl HostPathPolicy {
    pub fn new(allowed_roots: Vec<String>) -> Self {
        Self {
            allowed_roots: allowed_roots
                .into_iter()
                .map(|root| normalize_path(&root))
                .collect(),
        }
    }

    pub fn is_path_allowed(&self, path: &str) -> bool {
        let Some(normalized_path) = normalize_candidate_path(path) else {
            return false;
        };

        self.allowed_roots.iter().any(|root| {
            normalized_path == *root
                || normalized_path
                    .strip_prefix(root)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEnvPolicy {
    None,
    Inherit,
    AllowList(Vec<String>),
    Explicit(Vec<(String, String)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemOperation {
    Read,
    Write,
    List,
    Stat,
    Delete,
    Watch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemRequest {
    pub operation_id: String,
    pub operation: FilesystemOperation,
    pub path: String,
    pub content: Option<String>,
    pub policy_categories: Vec<String>,
}

impl FilesystemRequest {
    pub fn read(operation_id: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(operation_id, FilesystemOperation::Read, path, None)
    }

    pub fn write(
        operation_id: impl Into<String>,
        path: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self::new(
            operation_id,
            FilesystemOperation::Write,
            path,
            Some(content.into()),
        )
    }

    pub fn delete(operation_id: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(operation_id, FilesystemOperation::Delete, path, None)
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn side_effect_level(&self) -> SideEffectLevel {
        match self.operation {
            FilesystemOperation::Read
            | FilesystemOperation::List
            | FilesystemOperation::Stat
            | FilesystemOperation::Watch => SideEffectLevel::ReadOnly,
            FilesystemOperation::Write => SideEffectLevel::SideEffectful,
            FilesystemOperation::Delete => SideEffectLevel::Destructive,
        }
    }

    pub fn requires_policy(&self) -> bool {
        self.side_effect_level() != SideEffectLevel::ReadOnly || !self.policy_categories.is_empty()
    }

    fn new(
        operation_id: impl Into<String>,
        operation: FilesystemOperation,
        path: impl Into<String>,
        content: Option<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            operation,
            path: path.into(),
            content,
            policy_categories: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemResult {
    pub operation_id: String,
    pub content: Option<String>,
    pub entries: Vec<String>,
    pub metadata: Option<String>,
}

impl FilesystemResult {
    pub fn read(operation_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            content: Some(content.into()),
            entries: Vec::new(),
            metadata: None,
        }
    }

    pub fn completed(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            content: None,
            entries: Vec::new(),
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRequest {
    pub operation_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_directory: String,
    pub env_policy: HostEnvPolicy,
    pub timeout_ms: Option<u64>,
    pub policy_categories: Vec<String>,
}

impl ProcessRequest {
    pub fn spawn(
        operation_id: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        working_directory: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            command: command.into(),
            args,
            working_directory: working_directory.into(),
            env_policy: HostEnvPolicy::None,
            timeout_ms: None,
            policy_categories: Vec::new(),
        }
    }

    pub fn with_env_policy(mut self, env_policy: HostEnvPolicy) -> Self {
        self.env_policy = env_policy;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn requires_policy(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub operation_id: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub cancelled: bool,
    pub timed_out: bool,
}

impl ProcessResult {
    pub fn exited(
        operation_id: impl Into<String>,
        exit_code: i32,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            exit_code: Some(exit_code),
            stdout: stdout.into(),
            stderr: stderr.into(),
            cancelled: false,
            timed_out: false,
        }
    }

    pub fn cancelled(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            cancelled: true,
            timed_out: false,
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0) && !self.cancelled && !self.timed_out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRequest {
    pub operation_id: String,
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub timeout_ms: Option<u64>,
    pub policy_categories: Vec<String>,
}

impl NetworkRequest {
    pub fn get(operation_id: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            method: "GET".to_string(),
            url: url.into(),
            body: None,
            timeout_ms: None,
            policy_categories: Vec::new(),
        }
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn requires_policy(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkResult {
    pub operation_id: String,
    pub status_code: u16,
    pub body: String,
}

impl NetworkResult {
    pub fn response(
        operation_id: impl Into<String>,
        status_code: u16,
        body: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            status_code,
            body: body.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRef {
    pub secret_ref_id: String,
    pub display_name: String,
}

impl SecretRef {
    pub fn new(secret_ref_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            secret_ref_id: secret_ref_id.into(),
            display_name: display_name.into(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue {
    pub secret_ref_id: String,
    value: String,
}

impl SecretValue {
    pub fn new(secret_ref_id: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            secret_ref_id: secret_ref_id.into(),
            value: value.into(),
        }
    }

    pub fn expose_value(&self) -> &str {
        &self.value
    }

    pub fn redacted(&self) -> &'static str {
        "[REDACTED]"
    }
}

impl Debug for SecretValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SecretValue")
            .field("secret_ref_id", &self.secret_ref_id)
            .field("value", &self.redacted())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRequest {
    pub operation_id: String,
    pub scope: String,
    pub key: String,
    pub value: Option<String>,
    pub retention_days: Option<u32>,
}

impl StorageRequest {
    pub fn put(
        operation_id: impl Into<String>,
        scope: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            scope: scope.into(),
            key: key.into(),
            value: Some(value.into()),
            retention_days: None,
        }
    }

    pub fn with_retention_days(mut self, retention_days: u32) -> Self {
        self.retention_days = Some(retention_days);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeRequest {
    pub operation_id: String,
}

impl TimeRequest {
    pub fn now(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentRequest {
    pub operation_id: String,
    pub variable_name: String,
}

impl EnvironmentRequest {
    pub fn get(operation_id: impl Into<String>, variable_name: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            variable_name: variable_name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorRequest {
    pub operation_id: String,
    pub action_id: String,
    pub timeout_ms: Option<u64>,
    pub policy_categories: Vec<String>,
}

impl ExecutorRequest {
    pub fn run(operation_id: impl Into<String>, action_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            action_id: action_id.into(),
            timeout_ms: None,
            policy_categories: vec!["host.executor.run".to_string()],
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn requires_policy(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageResult {
    pub operation_id: String,
    pub stored: bool,
    pub version: Option<u64>,
}

impl StorageResult {
    pub fn stored(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            stored: true,
            version: None,
        }
    }

    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeResult {
    pub operation_id: String,
    pub timestamp: String,
    pub timezone: Option<String>,
}

impl TimeResult {
    pub fn now(operation_id: impl Into<String>, timestamp: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            timestamp: timestamp.into(),
            timezone: None,
        }
    }

    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentResult {
    pub operation_id: String,
    pub variable_name: String,
    pub value: Option<String>,
}

impl EnvironmentResult {
    pub fn resolved(
        operation_id: impl Into<String>,
        variable_name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            variable_name: variable_name.into(),
            value: Some(value.into()),
        }
    }

    pub fn not_found(operation_id: impl Into<String>, variable_name: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            variable_name: variable_name.into(),
            value: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorStatus {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorResult {
    pub operation_id: String,
    pub action_id: String,
    pub status: ExecutorStatus,
    pub output: Option<String>,
}

impl ExecutorResult {
    pub fn completed(
        operation_id: impl Into<String>,
        action_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            action_id: action_id.into(),
            status: ExecutorStatus::Completed,
            output: Some(output.into()),
        }
    }

    pub fn failed(
        operation_id: impl Into<String>,
        action_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: operation_id.into(),
            action_id: action_id.into(),
            status: ExecutorStatus::Failed,
            output: Some(output.into()),
        }
    }

    pub fn cancelled(operation_id: impl Into<String>, action_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            action_id: action_id.into(),
            status: ExecutorStatus::Cancelled,
            output: None,
        }
    }

    pub fn timed_out(operation_id: impl Into<String>, action_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            action_id: action_id.into(),
            status: ExecutorStatus::TimedOut,
            output: None,
        }
    }
}

fn normalize_candidate_path(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    if normalized.is_empty() || normalized == "." || normalized.starts_with("../") {
        return None;
    }

    Some(normalized)
}

fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut prefix = String::new();
    let mut parts = Vec::new();

    if path.as_bytes().get(1) == Some(&b':') {
        prefix = path[..2].to_ascii_uppercase();
    }

    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    parts.push("..");
                }
            }
            _ => parts.push(part),
        }
    }

    let suffix = parts.join("/");
    if prefix.is_empty() {
        suffix
    } else if suffix.is_empty() {
        prefix
    } else {
        format!("{prefix}/{suffix}")
    }
}
