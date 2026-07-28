# SDKWork Sandbox Provider Specification

- **Version**: 0.1.0
- **Status**: Legacy host-command mechanism; not a production Sandbox Runtime
- **Date**: 2025-06-28
- **Scope**: Historical one-shot host-command policy wrapper in `sdkwork-agent-kernel`
- **Domain**: `security`
- **Capability**: `agent-kernel.sandbox-provider`
- **Implementation**: `sdkwork-agent-kernel/src/sandbox.rs`
- **Test Coverage**: 16/16 tests passing (100%)

## 1. Ownership And Limitations

This specification describes the legacy root-exported Kernel `SandboxProvider`
used by `SandboxingHostProvider` for one-shot process execution. It is not the
`sdkwork-sandbox` Provider SPI, does not own `SandboxSession` or Workspace
Attachment lifecycle, and must not be used as a production multi-tenant
isolation claim.

The authoritative Runtime dependency direction is:

```text
sdkwork-agents -> sdkwork-kernel -> sdkwork-sandbox
```

New lifecycle work uses `sandbox_runtime::SandboxSessionLifecycleAdapter` and
the Sandbox-owned `SandboxSessionLifecyclePort`. The `NoOpSandboxProvider` is
test-only and must fail closed in release composition. Platform variants remain
unapproved until their security requirements and conformance evidence are
accepted.

## 2. Legacy Overview

The Sandbox Provider provides secure execution isolation for tool execution across multiple platforms:

- **Linux**: Landlock + seccomp (future: namespaces)
- **Windows**: Restricted token
- **macOS**: Seatbelt (future)

### Key Features

1. **Platform Abstraction**: Unified API across platforms
2. **File System Isolation**: Path-based access control
3. **Network Isolation**: Outbound/full network control
4. **Environment Control**: Controlled environment variables
5. **Policy Validation**: Pre-execution policy validation

## 3. Legacy Architecture

### Component Structure

```text
SandboxProvider (trait)
  ├── sandbox_type() -> SandboxType
  ├── is_available() -> bool
  ├── execute(command, policy) -> Result<SandboxExecutionResult>
  └── validate_policy(policy) -> Result<()>

SandboxPolicy
  ├── sandbox_type: SandboxType
  ├── file_system: FileSystemSandboxPolicy
  ├── network: NetworkSandboxPolicy
  ├── env: HashMap<String, String>
  └── working_dir: Option<PathBuf>

FileSystemSandboxPolicy
  ├── root: PathBuf
  ├── paths: HashMap<PathBuf, FileSystemPermission>
  ├── allow_network_fs: bool
  └── allow_temp: bool

NetworkSandboxPolicy
  ├── permission: NetworkPermission
  ├── allowed_hosts: Vec<String>
  └── allowed_ports: Vec<u16>
```

### Sandbox Types

| Type | Platform | Status |
|------|----------|--------|
| `None` | All | ✅ Implemented (NoOpSandboxProvider) |
| `LinuxSeccomp` | Linux | 🔄 Planned |
| `WindowsRestrictedToken` | Windows | 🔄 Planned |
| `MacosSeatbelt` | macOS | 🔄 Planned |

## 4. File System Permissions

### FileSystemPermission

| Permission | Description |
|------------|-------------|
| `None` | No access allowed |
| `ReadOnly` | Read-only access |
| `ReadWrite` | Read-write access |
| `Full` | Full access (read/write/execute) |

### FileSystemSandboxPolicy

```rust
let policy = FileSystemSandboxPolicy::new("/tmp/sandbox")
    .with_path("workspace", FileSystemPermission::ReadWrite)
    .with_path("config", FileSystemPermission::ReadOnly)
    .allow_network_fs(false)
    .allow_temp(true);
```

### Policy Presets

```rust
// Permissive policy (no restrictions)
let policy = FileSystemSandboxPolicy::permissive();

// Restrictive policy (minimal access)
let policy = FileSystemSandboxPolicy::restrictive("/tmp/sandbox");
```

## 5. Network Permissions

### NetworkPermission

| Permission | Description |
|------------|-------------|
| `None` | No network access |
| `Outbound` | Outbound connections only |
| `Full` | Full network access |

### NetworkSandboxPolicy

```rust
let policy = NetworkSandboxPolicy::outbound_only()
    .with_allowed_host("api.example.com")
    .with_allowed_host("cdn.example.com")
    .with_allowed_port(443)
    .with_allowed_port(80);
```

### Policy Presets

```rust
// No network access
let policy = NetworkSandboxPolicy::no_network();

// Outbound only
let policy = NetworkSandboxPolicy::outbound_only();

// Full network access
let policy = NetworkSandboxPolicy::full_network();
```

## 6. Complete Sandbox Policy

### Example: Restricted Workspace Access

```rust
let policy = SandboxPolicy::new(SandboxType::LinuxSeccomp)
    .with_file_system(
        FileSystemSandboxPolicy::new("/tmp/sandbox")
            .with_path("workspace", FileSystemPermission::ReadWrite)
            .with_path("output", FileSystemPermission::ReadWrite)
            .allow_temp(true)
    )
    .with_network(
        NetworkSandboxPolicy::outbound_only()
            .with_allowed_host("api.openai.com")
            .with_allowed_port(443)
    )
    .with_env("API_KEY", "secret")
    .with_working_dir("workspace");
```

### Example: No Network, Read-Only FS

```rust
let policy = SandboxPolicy::new(SandboxType::LinuxSeccomp)
    .with_file_system(
        FileSystemSandboxPolicy::restrictive("/tmp/sandbox")
            .with_path("data", FileSystemPermission::ReadOnly)
    )
    .with_network(NetworkSandboxPolicy::no_network());
```

## 7. Command Execution

### SandboxCommand

```rust
let command = SandboxCommand::new("python3")
    .with_arg("script.py")
    .with_arg("--input")
    .with_arg("data.txt")
    .with_cwd("/tmp/sandbox/workspace")
    .with_env("PYTHONPATH", "/opt/lib");
```

### Execution

```rust
let provider = LinuxSeccompSandboxProvider::new();
let result = provider.execute(command, policy)?;

if result.success() {
    println!("Output: {}", result.stdout);
} else {
    eprintln!("Error: {}", result.stderr);
}
```

### SandboxExecutionResult

```rust
pub struct SandboxExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub killed: bool,
}

impl SandboxExecutionResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.killed
    }
}
```

## 8. Provider Trait

### SandboxProvider

```rust
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
    fn info(&self) -> String;
}
```

### NoOpSandboxProvider

For testing environments where sandboxing is not needed:

```rust
let provider = NoOpSandboxProvider;
assert_eq!(provider.sandbox_type(), SandboxType::None);
assert!(provider.is_available());
```

## 9. Error Handling

### SandboxError

| Error | Description |
|-------|-------------|
| `NotAvailable` | Sandbox not available on platform |
| `InvalidPolicy(msg)` | Invalid sandbox policy |
| `ExecutionFailed(msg)` | Execution failed |
| `PermissionDenied(msg)` | Permission denied |
| `Timeout` | Execution timeout |
| `Unsupported(feature)` | Unsupported feature |

### Example Error Handling

```rust
match provider.execute(command, policy) {
    Ok(result) => {
        if result.success() {
            // Handle success
        } else {
            // Handle execution failure
        }
    }
    Err(SandboxError::NotAvailable) => {
        // Fallback to no-sandbox execution
    }
    Err(SandboxError::PermissionDenied(msg)) => {
        // Log and request user approval
    }
    Err(e) => {
        // Handle other errors
    }
}
```

## 10. Platform-Specific Implementations

### Linux: Landlock + Seccomp (Planned)

```rust
#[cfg(target_os = "linux")]
pub struct LinuxSeccompSandboxProvider {
    landlock_rules: Vec<LandlockRule>,
    seccomp_filter: SeccompFilter,
}

#[cfg(target_os = "linux")]
impl SandboxProvider for LinuxSeccompSandboxProvider {
    // Implementation using landlock syscall and seccomp-bpf
}
```

**Reference**: Codex CLI `codex-rs/sandboxing/src/landlock.rs`

### Windows: Restricted Token (Planned)

```rust
#[cfg(target_os = "windows")]
pub struct WindowsRestrictedTokenSandboxProvider {
    integrity_level: IntegrityLevel,
    restricted_sids: Vec<Sid>,
}

#[cfg(target_os = "windows")]
impl SandboxProvider for WindowsRestrictedTokenSandboxProvider {
    // Implementation using CreateRestrictedToken WinAPI
}
```

**Reference**: Codex CLI `codex-rs/windows-sandbox-rs/`

### macOS: Seatbelt (Future)

```rust
#[cfg(target_os = "macos")]
pub struct MacosSeatbeltSandboxProvider {
    seatbelt_profile: String,
}

#[cfg(target_os = "macos")]
impl SandboxProvider for MacosSeatbeltSandboxProvider {
    // Implementation using seatbelt sandbox_init()
}
```

**Reference**: Codex CLI `codex-rs/sandboxing/src/seatbelt.rs`

## 11. Conformance Tests

### Test Coverage (16 tests)

| Test Name | Coverage |
|-----------|----------|
| `test_sandbox_type_is_available` | Platform detection |
| `test_sandbox_type_as_str` | String conversion |
| `test_file_system_sandbox_policy_new` | FS policy creation |
| `test_file_system_sandbox_policy_with_path` | Path permissions |
| `test_file_system_sandbox_policy_permissive` | Permissive preset |
| `test_file_system_sandbox_policy_restrictive` | Restrictive preset |
| `test_network_sandbox_policy_new` | Network policy creation |
| `test_network_sandbox_policy_with_allowed` | Host/port restrictions |
| `test_sandbox_policy_new` | Complete policy creation |
| `test_sandbox_policy_with_env` | Environment variables |
| `test_sandbox_command_new` | Command creation |
| `test_sandbox_command_with_args` | Command arguments |
| `test_sandbox_execution_result_success` | Success detection |
| `test_sandbox_execution_result_failure` | Failure detection |
| `test_no_op_sandbox_provider` | NoOp provider |
| `test_sandbox_error_display` | Error formatting |

### Test Execution

```bash
cargo test --package sdkwork-agent-kernel --lib sandbox::tests
```

### Expected Result

```
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured
```

## 12. Integration Points

### HostProvider Integration

```rust
// In HostProvider
pub trait HostProvider {
    fn execute_tool(
        &self,
        tool: ToolSpec,
        input: ToolInput,
    ) -> Result<ToolOutput> {
        // Check if sandbox is required
        if self.policy.requires_sandbox(&tool) {
            let sandbox = self.sandbox_provider.as_ref()
                .ok_or(HostError::SandboxNotConfigured)?;
            
            let command = SandboxCommand::new(&tool.executable)
                .with_args(tool.args);
            
            let policy = self.policy.sandbox_policy_for(&tool);
            
            let result = sandbox.execute(command, policy)?;
            // Process result
        } else {
            // Execute without sandbox
        }
    }
}
```

### PolicyProvider Integration

```rust
// In PolicyProvider
pub trait PolicyProvider {
    fn sandbox_policy_for_tool(
        &self,
        tool: &ToolSpec,
    ) -> SandboxPolicy {
        match self.get_tool_permission(tool) {
            Permission::Full => SandboxPolicy::new(SandboxType::None),
            Permission::Sandboxed => {
                SandboxPolicy::new(SandboxType::is_available().unwrap_or(SandboxType::None))
                    .with_file_system(self.fs_policy_for(tool))
                    .with_network(self.network_policy_for(tool))
            }
            Permission::Denied => {
                // Return most restrictive policy
                SandboxPolicy::new(SandboxType::is_available().unwrap_or(SandboxType::None))
                    .with_file_system(FileSystemSandboxPolicy::restrictive("/"))
                    .with_network(NetworkSandboxPolicy::no_network())
            }
        }
    }
}
```

### TelemetryProvider Integration

```rust
// Record sandbox execution metrics
telemetry.counter("sandbox.execution", 1, &[
    ("type", sandbox.sandbox_type().as_str()),
    ("success", result.success().to_string()),
]);

telemetry.histogram("sandbox.execution_time", duration.as_millis(), &[
    ("type", sandbox.sandbox_type().as_str()),
]);
```

## 13. Security Considerations

### Defense in Depth

1. **File System Isolation**: Prevent unauthorized file access
2. **Network Isolation**: Prevent unauthorized network access
3. **Environment Control**: Prevent information leakage via env vars
4. **Resource Limits**: Prevent resource exhaustion (future)

### Threat Mitigation

| Threat | Mitigation |
|--------|------------|
| Unauthorized file read | FileSystemPermission::ReadOnly |
| Unauthorized file write | FileSystemPermission::None |
| Data exfiltration | NetworkPermission::None |
| Privilege escalation | Restricted token / seccomp |
| Resource exhaustion | Resource limits (future) |

### Audit Trail

All sandbox executions should be logged:

```rust
telemetry.audit(AuditRecord::new(
    "sandbox.execute",
    AuditSeverity::Info,
    format!("tool={};policy={}", tool.name, policy.sandbox_type.as_str()),
));
```

## 14. Performance Characteristics

### Overhead

| Platform | Overhead | Notes |
|----------|----------|-------|
| Linux (Landlock) | ~5-10ms | One-time setup cost |
| Windows (Restricted Token) | ~10-20ms | Token creation cost |
| macOS (Seatbelt) | ~5-10ms | Profile compilation |
| NoOp | ~0ms | Direct execution |

### Memory

- **Per Execution**: ~1-5MB (sandbox process overhead)
- **Policy**: ~1KB per policy object

### Recommendations

- Cache sandbox providers for reuse
- Pre-validate policies before execution
- Use permissive policies for trusted tools
- Use restrictive policies for untrusted tools

## 15. Future Extensions

### Planned Extensions (Phase 6)

1. **Linux Namespaces**: PID, mount, network namespaces
2. **Resource Limits**: CPU, memory, file descriptor limits
3. **Time Limits**: Execution timeout enforcement
4. **Seccomp BPF**: Custom syscall filtering
5. **Container Integration**: Docker/Podman integration

### Extension Points

```rust
// Future: Resource limits
pub struct ResourceLimits {
    pub cpu_time: Option<Duration>,
    pub memory: Option<usize>,
    pub file_descriptors: Option<usize>,
    pub processes: Option<usize>,
}

// Future: Container integration
pub trait ContainerSandboxProvider: SandboxProvider {
    fn execute_in_container(
        &self,
        command: SandboxCommand,
        policy: SandboxPolicy,
        container: ContainerSpec,
    ) -> Result<SandboxExecutionResult>;
}
```

## 16. References

- `sdkwork-agent-kernel/src/sandbox.rs` - Implementation
- `sdkwork-agent-kernel/src/lib.rs` - Module exports
- `codex-rs/sandboxing/` - Codex CLI sandbox reference
- `specs/AGENT_KERNEL_SPEC.md` - Kernel specification
- `specs/HOST_PROVIDER_SPEC.md` - Host provider specification

## 17. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-06-28 | Core abstraction, 16/16 tests passing |

---

**Status**: Legacy one-shot host-command abstraction retained for existing Kernel behavior
**Next Steps**: Do not extend this contract into a production Sandbox Runtime; use the `sdkwork-sandbox` lifecycle and Provider contracts
