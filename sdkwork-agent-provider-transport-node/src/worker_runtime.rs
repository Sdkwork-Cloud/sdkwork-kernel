use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
use sdkwork_agent_provider_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeActivityEvent,
    SdkRuntimeActivityEventSink, SdkRuntimeError, SdkRuntimeOperation, SdkRuntimeRequest,
    SdkRuntimeResponse,
};
use sdkwork_agent_provider_transport_ipc::{
    is_invoke_terminal_frame, is_session_activity_frame, provider_worker_concurrency_limit,
    FailClosedJsonRpcTransport, JsonRpcTransport, PackageStubJsonRpcTransport, SpawnedWorker,
    SpawnedWorkerLease, SpawnedWorkerPool, TransportError, SDKWORK_CAPABILITY_INVOKE_METHOD,
    SDKWORK_PING_METHOD,
};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

const WORKER_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);
const HEALTH_WORKER_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_WORKER_OPERATION_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_WORKER_OPERATION_TIMEOUT: Duration = Duration::from_secs(3600);

const NODE_BINARY_ENV: &str = "SDKWORK_AGENT_NODE_BINARY";
const WORKER_SCRIPT_ENV: &str = "SDKWORK_AGENT_TYPESCRIPT_WORKER_SCRIPT";
const PROVIDER_HOST_ROOT_ENV: &str = "SDKWORK_AGENT_PROVIDER_HOST_ROOT";
const LEGACY_PROVIDER_RUNTIME_ROOT_ENV: &str = "SDKWORK_AGENT_PROVIDER_RUNTIME_ROOT";
const PROVIDER_HOST_DIR_NAME: &str = "provider-host";
const LEGACY_PROVIDER_RUNTIME_DIR_NAME: &str = "provider-runtime";
const TYPESCRIPT_WORKER_RELATIVE_PATH: &str = "workers/generic-ts-sdk-worker.mjs";

#[derive(Debug, Clone)]
pub struct NodeWorkerLaunchOptions {
    pub node_binary: String,
    pub worker_script: PathBuf,
    pub package_name: String,
}

impl NodeWorkerLaunchOptions {
    pub fn for_package(package_name: impl Into<String>) -> Self {
        Self {
            node_binary: default_node_binary(),
            worker_script: default_typescript_worker_script(),
            package_name: package_name.into(),
        }
    }
}

/// Resolves the Node executable used by the provider worker.
///
/// Release packages set `SDKWORK_AGENT_PROVIDER_HOST_ROOT` (or place a
/// `provider-host` directory beside the executable). Development builds
/// continue to use the system `node` command when no packaged host exists.
pub fn default_node_binary() -> String {
    if let Some(configured) = std::env::var_os(NODE_BINARY_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return configured.to_string_lossy().into_owned();
    }

    if let Some(root) = provider_host_root() {
        if let Some(binary) = bundled_node_binary(&root) {
            return binary.to_string_lossy().into_owned();
        }
    }

    "node".to_string()
}

pub fn default_typescript_worker_script() -> PathBuf {
    if let Some(configured) = std::env::var_os(WORKER_SCRIPT_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return configured;
    }

    if let Some(root) = provider_host_root() {
        return root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
    }

    // Keep repository-relative source paths out of release binaries. The
    // fallback is retained for debug/test builds where the sibling kernel
    // checkout is available.
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../scripts/provider-transport-workers/generic-ts-sdk-worker.mjs")
    }

    #[cfg(not(debug_assertions))]
    {
        PathBuf::from(PROVIDER_HOST_DIR_NAME).join(TYPESCRIPT_WORKER_RELATIVE_PATH)
    }
}

fn provider_host_root() -> Option<PathBuf> {
    for environment_key in [PROVIDER_HOST_ROOT_ENV, LEGACY_PROVIDER_RUNTIME_ROOT_ENV] {
        if let Some(configured) = std::env::var_os(environment_key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
        {
            return Some(configured);
        }
    }

    let executable = std::env::current_exe().ok()?;
    find_packaged_provider_host_root(executable.parent()?, TYPESCRIPT_WORKER_RELATIVE_PATH)
}

fn find_packaged_provider_host_root(
    start_directory: &Path,
    worker_relative_path: &str,
) -> Option<PathBuf> {
    for directory_name in [PROVIDER_HOST_DIR_NAME, LEGACY_PROVIDER_RUNTIME_DIR_NAME] {
        let mut ancestors = Some(start_directory);
        while let Some(directory) = ancestors {
            let candidates = [
                directory.join(directory_name),
                directory.join("resources").join(directory_name),
                directory.join("Resources").join(directory_name),
                directory
                    .join("share")
                    .join("sdkwork-birdcoder")
                    .join(directory_name),
            ];
            if let Some(candidate) = candidates
                .into_iter()
                .find(|path| path.join(worker_relative_path).is_file())
            {
                return Some(candidate);
            }
            ancestors = directory.parent();
        }
    }

    None
}

fn bundled_node_binary(root: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        [
            root.join("node").join("node.exe"),
            root.join("node").join("bin").join("node.exe"),
            root.join("node").join("bin").join("node"),
        ]
    } else {
        [
            root.join("node").join("bin").join("node"),
            root.join("node").join("node"),
            root.join("node").join("bin").join("node.exe"),
        ]
    };
    candidates.into_iter().find(|path| path.is_file())
}

enum NodeRuntimeBackend {
    Stub(Arc<dyn JsonRpcTransport + Send + Sync>),
    FailClosed(Arc<dyn JsonRpcTransport + Send + Sync>),
    Managed { pool: Arc<SpawnedWorkerPool> },
}

pub struct NodeSdkBackendRuntime {
    package_name: String,
    backend: NodeRuntimeBackend,
    activity_sink: Option<Arc<dyn SdkRuntimeActivityEventSink>>,
}

impl NodeSdkBackendRuntime {
    pub fn bootstrap(package_name: impl Into<String>) -> Self {
        let options = NodeWorkerLaunchOptions::for_package(package_name);
        match Self::spawn(&options) {
            Ok(runtime) => runtime,
            Err(error) => {
                if mock_provider_invocation_allowed() {
                    Self::in_memory_stub(options.package_name, true)
                } else {
                    Self::fail_closed(options.package_name, error.to_string())
                }
            }
        }
    }

    pub fn from_transport(
        transport: Arc<dyn JsonRpcTransport + Send + Sync>,
        package_name: impl Into<String>,
    ) -> Self {
        let package_name = package_name.into();
        if !mock_provider_invocation_allowed() {
            return Self::fail_closed(
                package_name,
                "mock provider transport injection is disabled for this runtime profile",
            );
        }

        Self {
            package_name,
            backend: NodeRuntimeBackend::Stub(transport),
            activity_sink: None,
        }
    }

    pub fn spawn(options: &NodeWorkerLaunchOptions) -> Result<Self, SdkRuntimeError> {
        if !options.worker_script.exists() {
            return Err(SdkRuntimeError::new(
                "worker_script_missing",
                format!(
                    "typescript worker script not found: {}",
                    options.worker_script.display()
                ),
            ));
        }

        let launch_options = options.clone();
        let pool = Arc::new(
            SpawnedWorkerPool::new(provider_worker_concurrency_limit(), move || {
                spawn_worker(&launch_options)
            })
            .map_err(map_transport_error)?,
        );
        pool.warm_up(WORKER_ACQUIRE_TIMEOUT)
            .map_err(map_transport_error)?;
        Ok(Self {
            package_name: options.package_name.clone(),
            backend: NodeRuntimeBackend::Managed { pool },
            activity_sink: None,
        })
    }

    pub fn in_memory_stub(package_name: impl Into<String>, _ping_ok: bool) -> Self {
        let package_name = package_name.into();
        if !mock_provider_invocation_allowed() {
            return Self::fail_closed(
                package_name,
                "mock provider fallback is disabled for this runtime profile",
            );
        }

        Self {
            package_name: package_name.clone(),
            backend: NodeRuntimeBackend::Stub(Arc::new(PackageStubJsonRpcTransport::new(
                package_name,
                "typescript_node",
            ))),
            activity_sink: None,
        }
    }

    pub fn fail_closed(package_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            backend: NodeRuntimeBackend::FailClosed(Arc::new(FailClosedJsonRpcTransport::new(
                reason.into(),
            ))),
            activity_sink: None,
        }
    }

    pub fn with_activity_sink(mut self, sink: Arc<dyn SdkRuntimeActivityEventSink>) -> Self {
        self.activity_sink = Some(sink);
        self
    }

    fn shared_transport(&self) -> Result<Arc<dyn JsonRpcTransport + Send + Sync>, SdkRuntimeError> {
        match &self.backend {
            NodeRuntimeBackend::Stub(transport) | NodeRuntimeBackend::FailClosed(transport) => {
                Ok(transport.clone())
            }
            NodeRuntimeBackend::Managed { .. } => Err(SdkRuntimeError::new(
                "transport_error",
                "managed workers require a request-scoped lease",
            )),
        }
    }

    fn acquire_worker(
        pool: &SpawnedWorkerPool,
        request: &SdkRuntimeRequest,
    ) -> Result<SpawnedWorkerLease, SdkRuntimeError> {
        match request.operation.request_id() {
            Some(request_id) => pool
                .acquire(request_id, WORKER_ACQUIRE_TIMEOUT)
                .map_err(map_transport_error),
            None => pool
                .acquire_internal("invoke", WORKER_ACQUIRE_TIMEOUT)
                .map_err(map_transport_error),
        }
    }

    fn ping_worker(&self) -> Result<Value, SdkRuntimeError> {
        match &self.backend {
            NodeRuntimeBackend::Managed { pool } => {
                let lease = pool
                    .acquire_internal("health", HEALTH_WORKER_ACQUIRE_TIMEOUT)
                    .map_err(map_transport_error)?;
                lease
                    .call_with_timeout(SDKWORK_PING_METHOD, None, HEALTH_WORKER_ACQUIRE_TIMEOUT)
                    .map_err(map_transport_error)
            }
            NodeRuntimeBackend::Stub(transport) | NodeRuntimeBackend::FailClosed(transport) => {
                transport
                    .call(SDKWORK_PING_METHOD, None)
                    .map_err(map_transport_error)
            }
        }
    }

    fn invoke_worker(&self, request: &SdkRuntimeRequest) -> Result<Value, SdkRuntimeError> {
        let params = json!({
            "capability_id": request.capability_id,
            "operation": request.operation,
            "payload": request.payload,
            "package": self.package_name,
        });
        match &self.backend {
            NodeRuntimeBackend::Managed { pool } => {
                let lease = Self::acquire_worker(pool, request)?;
                lease
                    .call_with_timeout(
                        SDKWORK_CAPABILITY_INVOKE_METHOD,
                        Some(params),
                        worker_operation_timeout(request),
                    )
                    .map_err(map_transport_error)
            }
            NodeRuntimeBackend::Stub(_) | NodeRuntimeBackend::FailClosed(_) => self
                .shared_transport()?
                .call(SDKWORK_CAPABILITY_INVOKE_METHOD, Some(params))
                .map_err(map_transport_error),
        }
    }

    fn invoke_worker_with_activity(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<Value, SdkRuntimeError> {
        let params = json!({
            "capability_id": request.capability_id,
            "operation": request.operation,
            "payload": request.payload,
            "package": self.package_name,
            "activity_stream": true,
        });
        let mut terminal_payload = None;
        match &self.backend {
            NodeRuntimeBackend::Managed { pool } => {
                let lease = Self::acquire_worker(pool, request)?;
                lease
                    .call_streaming_with_timeout(
                        SDKWORK_CAPABILITY_INVOKE_METHOD,
                        Some(params),
                        worker_operation_timeout(request),
                        &mut |frame| {
                            if self
                                .ingest_activity_frame(&frame)
                                .map_err(|error| TransportError::new(error.message))?
                            {
                                return Ok(true);
                            }
                            if is_invoke_terminal_frame(&frame) {
                                terminal_payload = frame.get("payload").cloned();
                                return Ok(true);
                            }
                            Err(TransportError::new(
                                "activity-enabled invoke received an unexpected worker frame",
                            ))
                        },
                    )
                    .map_err(map_transport_error)?;
            }
            NodeRuntimeBackend::Stub(_) | NodeRuntimeBackend::FailClosed(_) => {
                return self.invoke_worker(request);
            }
        }
        terminal_payload.ok_or_else(|| {
            SdkRuntimeError::new(
                "transport_error",
                "activity-enabled invoke completed without invoke.done",
            )
        })
    }

    fn invoke_worker_streaming(
        &self,
        request: &SdkRuntimeRequest,
        sink: &mut dyn FnMut(Value) -> Result<bool, SdkRuntimeError>,
    ) -> Result<(), SdkRuntimeError> {
        let params = json!({
            "capability_id": request.capability_id,
            "operation": request.operation,
            "payload": request.payload,
            "package": self.package_name,
            "activity_stream": self.activity_sink.is_some(),
        });
        match &self.backend {
            NodeRuntimeBackend::Managed { pool } => {
                let lease = Self::acquire_worker(pool, request)?;
                lease
                    .call_streaming_with_timeout(
                        SDKWORK_CAPABILITY_INVOKE_METHOD,
                        Some(params),
                        worker_operation_timeout(request),
                        &mut |frame| {
                            if self
                                .ingest_activity_frame(&frame)
                                .map_err(|error| TransportError::new(error.message))?
                            {
                                return Ok(true);
                            }
                            sink(frame).map_err(|error| TransportError::new(error.message))
                        },
                    )
                    .map_err(map_transport_error)
            }
            NodeRuntimeBackend::Stub(_) | NodeRuntimeBackend::FailClosed(_) => self
                .shared_transport()?
                .call_streaming(
                    SDKWORK_CAPABILITY_INVOKE_METHOD,
                    Some(params),
                    &mut |frame| {
                        if self
                            .ingest_activity_frame(&frame)
                            .map_err(|error| TransportError::new(error.message))?
                        {
                            return Ok(true);
                        }
                        sink(frame).map_err(|error| TransportError::new(error.message))
                    },
                )
                .map_err(map_transport_error),
        }
    }

    fn ingest_activity_frame(&self, frame: &Value) -> Result<bool, SdkRuntimeError> {
        if !is_session_activity_frame(frame) {
            return Ok(false);
        }
        let sink = self.activity_sink.as_ref().ok_or_else(|| {
            SdkRuntimeError::new(
                "unexpected_activity_event",
                "worker emitted session activity without a configured sink",
            )
        })?;
        let event =
            serde_json::from_value::<SdkRuntimeActivityEvent>(frame.clone()).map_err(|error| {
                SdkRuntimeError::new(
                    "invalid_activity_event",
                    format!("decode worker activity event failed: {error}"),
                )
            })?;
        sink.ingest_runtime_activity(event)?;
        Ok(true)
    }
}

impl SdkBackendRuntime for NodeSdkBackendRuntime {
    fn backend_kind(&self) -> SdkBackendKind {
        SdkBackendKind::TypeScriptNode
    }

    fn health(&self) -> SdkDriverHealth {
        match self.ping_worker() {
            Ok(result) => match result.get("ok").and_then(Value::as_bool) {
                Some(true) => package_probe_health(&self.package_name, &result),
                _ => SdkDriverHealth::degraded("worker ping returned unexpected payload"),
            },
            Err(error) => SdkDriverHealth::unhealthy(error.message),
        }
    }

    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        if matches!(request.operation, SdkRuntimeOperation::Ping) {
            let payload = self.ping_worker()?;
            return Ok(SdkRuntimeResponse::success(
                SdkBackendKind::TypeScriptNode,
                &request.capability_id,
                payload,
            ));
        }

        let payload = if matches!(request.operation, SdkRuntimeOperation::ModelChat { .. })
            && self.activity_sink.is_some()
            && matches!(&self.backend, NodeRuntimeBackend::Managed { .. })
        {
            self.invoke_worker_with_activity(request)?
        } else {
            self.invoke_worker(request)?
        };
        if payload.get("ok").and_then(Value::as_bool) == Some(false) {
            let message = payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("typescript worker invoke failed")
                .to_string();
            return Ok(SdkRuntimeResponse::failure(
                SdkBackendKind::TypeScriptNode,
                message,
            ));
        }
        Ok(SdkRuntimeResponse::success(
            SdkBackendKind::TypeScriptNode,
            &request.capability_id,
            payload,
        ))
    }

    fn invoke_streaming(
        &self,
        request: &SdkRuntimeRequest,
        sink: &mut dyn FnMut(Value) -> Result<bool, SdkRuntimeError>,
    ) -> Result<(), SdkRuntimeError> {
        if matches!(request.operation, SdkRuntimeOperation::Ping) {
            let payload = self.ping_worker()?;
            sink(payload)?;
            return Ok(());
        }

        if matches!(
            request.operation,
            SdkRuntimeOperation::ModelChatStream { .. }
        ) {
            return self.invoke_worker_streaming(request, sink);
        }

        SdkBackendRuntime::invoke_streaming(self, request, sink)
    }

    fn cancel_inflight(&self, request_id: &str) -> Result<bool, SdkRuntimeError> {
        match &self.backend {
            NodeRuntimeBackend::Managed { pool } => {
                pool.cancel(request_id).map_err(map_transport_error)
            }
            NodeRuntimeBackend::Stub(_) | NodeRuntimeBackend::FailClosed(_) => Ok(false),
        }
    }
}

fn worker_operation_timeout(request: &SdkRuntimeRequest) -> Duration {
    let timeout_ms = match &request.operation {
        SdkRuntimeOperation::ModelChat { timeout_ms, .. }
        | SdkRuntimeOperation::ModelChatStream { timeout_ms, .. } => *timeout_ms,
        _ => None,
    };
    timeout_ms
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_WORKER_OPERATION_TIMEOUT)
        .clamp(Duration::from_millis(1), MAX_WORKER_OPERATION_TIMEOUT)
}

fn spawn_worker(options: &NodeWorkerLaunchOptions) -> Result<SpawnedWorker, TransportError> {
    let mut command = Command::new(&options.node_binary);
    command
        .arg(&options.worker_script)
        .arg("--package")
        .arg(&options.package_name);
    prepend_provider_host_bin_to_path(&mut command);
    SpawnedWorker::spawn(command)
}

fn prepend_provider_host_bin_to_path(command: &mut Command) {
    let Some(root) = provider_host_root() else {
        return;
    };
    if let Some(path) = provider_worker_path(&root, std::env::var_os("PATH")) {
        command.env("PATH", path);
    }
}

fn provider_worker_path(root: &Path, inherited_path: Option<OsString>) -> Option<OsString> {
    let provider_bin = root.join("node_modules").join(".bin");
    let inherited = inherited_path
        .as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten();
    std::env::join_paths(std::iter::once(provider_bin).chain(inherited)).ok()
}

fn map_transport_error(error: TransportError) -> SdkRuntimeError {
    SdkRuntimeError::new("transport_error", error.message)
}

fn package_probe_health(package_name: &str, payload: &Value) -> SdkDriverHealth {
    if payload.get("runtime_available").and_then(Value::as_bool) == Some(true) {
        return SdkDriverHealth::healthy();
    }

    match payload.get("package_resolved").and_then(Value::as_bool) {
        Some(true) | None => SdkDriverHealth::healthy(),
        Some(false) if mock_provider_invocation_allowed() => SdkDriverHealth::degraded(format!(
            "official sdk package is not resolved; development mock fallback is enabled: {package_name}"
        )),
        Some(false) => SdkDriverHealth::unhealthy(format!(
            "official sdk package is not resolved and mock fallback is disabled: {package_name}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
    use sdkwork_agent_provider_spi::SdkDriverStatus;
    use std::sync::{Mutex, OnceLock};

    #[derive(Default)]
    struct RecordingActivitySink {
        events: Mutex<Vec<SdkRuntimeActivityEvent>>,
    }

    impl SdkRuntimeActivityEventSink for RecordingActivitySink {
        fn ingest_runtime_activity(
            &self,
            event: SdkRuntimeActivityEvent,
        ) -> Result<(), SdkRuntimeError> {
            self.events
                .lock()
                .expect("activity events lock")
                .push(event);
            Ok(())
        }
    }

    const KERNEL_PROFILE_ID_ENV: &str = "SDKWORK_KERNEL_PROFILE_ID";
    const KERNEL_ENVIRONMENT_ENV: &str = "SDKWORK_KERNEL_ENVIRONMENT";
    const ALLOW_MOCK_PROVIDERS_ENV: &str = "SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS";
    const OPENCLAW_GATEWAY_URL_ENV: &str = "OPENCLAW_GATEWAY_URL";
    const NODE_BINARY_ENV: &str = "SDKWORK_AGENT_NODE_BINARY";
    const WORKER_SCRIPT_ENV: &str = "SDKWORK_AGENT_TYPESCRIPT_WORKER_SCRIPT";
    const PROVIDER_HOST_ROOT_ENV: &str = "SDKWORK_AGENT_PROVIDER_HOST_ROOT";
    const LEGACY_PROVIDER_RUNTIME_ROOT_ENV: &str = "SDKWORK_AGENT_PROVIDER_RUNTIME_ROOT";

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let previous = std::env::var(key).ok();
            match value {
                Some(next) => std::env::set_var(key, next),
                None => std::env::remove_var(key),
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn default_worker_script_points_to_repository_script() {
        let _lock = env_lock();
        let _host_root = EnvVarGuard::set(PROVIDER_HOST_ROOT_ENV, None);
        let _legacy_root = EnvVarGuard::set(LEGACY_PROVIDER_RUNTIME_ROOT_ENV, None);
        let _script = EnvVarGuard::set(WORKER_SCRIPT_ENV, None);
        let script = default_typescript_worker_script();
        assert!(
            script.exists(),
            "default TypeScript worker script must exist: {}",
            script.display()
        );
    }

    #[test]
    fn packaged_host_root_resolves_worker_and_node_without_repository_paths() {
        let _lock = env_lock();
        let root =
            std::env::temp_dir().join(format!("sdkwork-provider-host-test-{}", std::process::id()));
        let worker = root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
        let node = if cfg!(windows) {
            root.join("node").join("node.exe")
        } else {
            root.join("node").join("bin").join("node")
        };
        std::fs::create_dir_all(worker.parent().expect("worker parent")).expect("worker dir");
        std::fs::create_dir_all(node.parent().expect("node parent")).expect("node dir");
        std::fs::write(&worker, "#!/usr/bin/env node\n").expect("worker file");
        std::fs::write(&node, "provider node\n").expect("node file");

        let _host_root = EnvVarGuard::set(
            PROVIDER_HOST_ROOT_ENV,
            Some(root.to_string_lossy().as_ref()),
        );
        let _legacy_root = EnvVarGuard::set(LEGACY_PROVIDER_RUNTIME_ROOT_ENV, None);
        let _script = EnvVarGuard::set(WORKER_SCRIPT_ENV, None);
        let _node = EnvVarGuard::set(NODE_BINARY_ENV, None);

        assert_eq!(default_typescript_worker_script(), worker);
        assert_eq!(default_node_binary(), node.to_string_lossy());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_host_root_precedes_the_legacy_runtime_root() {
        let _lock = env_lock();
        let base = std::env::temp_dir().join(format!(
            "sdkwork-provider-host-precedence-test-{}",
            std::process::id()
        ));
        let host_root = base.join("provider-host");
        let legacy_root = base.join("provider-runtime");
        let host_worker = host_root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
        let legacy_worker = legacy_root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
        std::fs::create_dir_all(host_worker.parent().expect("host worker parent"))
            .expect("host worker dir");
        std::fs::create_dir_all(legacy_worker.parent().expect("legacy worker parent"))
            .expect("legacy worker dir");
        std::fs::write(&host_worker, "host worker\n").expect("host worker file");
        std::fs::write(&legacy_worker, "legacy worker\n").expect("legacy worker file");

        let _host_root = EnvVarGuard::set(
            PROVIDER_HOST_ROOT_ENV,
            Some(host_root.to_string_lossy().as_ref()),
        );
        let _legacy_root = EnvVarGuard::set(
            LEGACY_PROVIDER_RUNTIME_ROOT_ENV,
            Some(legacy_root.to_string_lossy().as_ref()),
        );
        let _script = EnvVarGuard::set(WORKER_SCRIPT_ENV, None);

        assert_eq!(default_typescript_worker_script(), host_worker);

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn legacy_runtime_root_remains_a_compatibility_input() {
        let _lock = env_lock();
        let root = std::env::temp_dir().join(format!(
            "sdkwork-provider-runtime-compatibility-test-{}",
            std::process::id()
        ));
        let worker = root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
        std::fs::create_dir_all(worker.parent().expect("worker parent")).expect("worker dir");
        std::fs::write(&worker, "legacy worker\n").expect("worker file");

        let _host_root = EnvVarGuard::set(PROVIDER_HOST_ROOT_ENV, None);
        let _legacy_root = EnvVarGuard::set(
            LEGACY_PROVIDER_RUNTIME_ROOT_ENV,
            Some(root.to_string_lossy().as_ref()),
        );
        let _script = EnvVarGuard::set(WORKER_SCRIPT_ENV, None);

        assert_eq!(default_typescript_worker_script(), worker);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn packaged_host_directory_precedes_a_nearer_legacy_runtime_directory() {
        let base = std::env::temp_dir().join(format!(
            "sdkwork-provider-host-directory-precedence-test-{}",
            std::process::id()
        ));
        let start_directory = base.join("application").join("bin");
        let host_root = base.join(PROVIDER_HOST_DIR_NAME);
        let legacy_root = start_directory.join(LEGACY_PROVIDER_RUNTIME_DIR_NAME);
        let host_worker = host_root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
        let legacy_worker = legacy_root.join(TYPESCRIPT_WORKER_RELATIVE_PATH);
        std::fs::create_dir_all(&start_directory).expect("start directory");
        std::fs::create_dir_all(host_worker.parent().expect("host worker parent"))
            .expect("host worker directory");
        std::fs::create_dir_all(legacy_worker.parent().expect("legacy worker parent"))
            .expect("legacy worker directory");
        std::fs::write(&host_worker, "host worker\n").expect("host worker file");
        std::fs::write(&legacy_worker, "legacy worker\n").expect("legacy worker file");

        assert_eq!(
            find_packaged_provider_host_root(&start_directory, TYPESCRIPT_WORKER_RELATIVE_PATH,),
            Some(host_root)
        );

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_worker_and_node_paths_take_precedence_over_packaged_host() {
        let _lock = env_lock();
        let root = std::env::temp_dir().join(format!(
            "sdkwork-provider-host-explicit-test-{}",
            std::process::id()
        ));
        let explicit_worker = root.join("explicit-worker.mjs");
        let explicit_node = root.join(if cfg!(windows) {
            "explicit-node.exe"
        } else {
            "explicit-node"
        });
        std::fs::create_dir_all(&root).expect("runtime test dir");
        std::fs::write(&explicit_worker, "worker\n").expect("worker file");
        std::fs::write(&explicit_node, "node\n").expect("node file");

        let _host_root = EnvVarGuard::set(PROVIDER_HOST_ROOT_ENV, None);
        let _legacy_root = EnvVarGuard::set(LEGACY_PROVIDER_RUNTIME_ROOT_ENV, None);
        let _script = EnvVarGuard::set(
            WORKER_SCRIPT_ENV,
            Some(explicit_worker.to_string_lossy().as_ref()),
        );
        let _node = EnvVarGuard::set(
            NODE_BINARY_ENV,
            Some(explicit_node.to_string_lossy().as_ref()),
        );

        assert_eq!(default_typescript_worker_script(), explicit_worker);
        assert_eq!(default_node_binary(), explicit_node.to_string_lossy());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn provider_host_bin_precedes_the_inherited_worker_path() {
        let root = PathBuf::from("provider-host-test");
        let inherited = std::env::join_paths([
            PathBuf::from("existing-bin-one"),
            PathBuf::from("existing-bin-two"),
        ])
        .expect("test path");

        let path = provider_worker_path(&root, Some(inherited)).expect("worker path");
        let entries: Vec<_> = std::env::split_paths(&path).collect();

        assert_eq!(entries[0], root.join("node_modules").join(".bin"));
        assert_eq!(entries[1], PathBuf::from("existing-bin-one"));
        assert_eq!(entries[2], PathBuf::from("existing-bin-two"));
    }

    #[test]
    fn in_memory_stub_invokes_ping() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(KERNEL_PROFILE_ID_ENV, None);
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("development"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, Some("1"));

        let runtime = NodeSdkBackendRuntime::in_memory_stub("openclaw", true);
        let response = runtime
            .invoke(&SdkRuntimeRequest::ping("sdk.session.lifecycle"))
            .expect("ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::TypeScriptNode);
    }

    #[test]
    fn typed_activity_frame_reaches_configured_sink() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(KERNEL_PROFILE_ID_ENV, None);
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("development"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, Some("1"));
        let sink = Arc::new(RecordingActivitySink::default());
        let runtime = NodeSdkBackendRuntime::in_memory_stub("provider-test", true)
            .with_activity_sink(sink.clone());

        assert!(runtime
            .ingest_activity_frame(&json!({
                "event": "session.activity",
                "provider_session_id": "provider.test",
                "phase": "working",
                "observed_at": "2026-01-01T00:00:00Z"
            }))
            .expect("activity frame should ingest"));
        let events = sink.events.lock().expect("activity events lock");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].provider_session_id, "provider.test");
    }

    #[test]
    fn in_memory_stub_fails_closed_in_production_profile() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);
        assert!(!mock_provider_invocation_allowed());

        let runtime = NodeSdkBackendRuntime::in_memory_stub("openclaw", true);
        let response = runtime
            .invoke(&SdkRuntimeRequest::model_chat(
                "sdk.model.chat",
                "req.production.stub",
                vec!["hello".to_string()],
            ))
            .expect("runtime should map fail-closed payloads");

        assert!(!response.success);
        assert!(response
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("production profile"));
    }

    #[test]
    fn from_transport_fails_closed_in_production_profile() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);
        assert!(!mock_provider_invocation_allowed());

        let transport = Arc::new(PackageStubJsonRpcTransport::new(
            "openclaw",
            "typescript_node",
        ));
        let runtime = NodeSdkBackendRuntime::from_transport(transport, "openclaw");
        let response = runtime
            .invoke(&SdkRuntimeRequest::model_chat(
                "sdk.model.chat",
                "req.production.transport",
                vec!["hello".to_string()],
            ))
            .expect("runtime should map fail-closed payloads");

        assert!(!response.success);
        assert!(response
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("production profile"));
    }

    #[test]
    fn health_is_unhealthy_when_official_sdk_is_missing_in_production_profile() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);
        assert!(!mock_provider_invocation_allowed());

        let runtime = NodeSdkBackendRuntime::bootstrap("@sdkwork/missing-sdk");
        let health = runtime.health();

        assert_eq!(health.status, SdkDriverStatus::Unhealthy);
        assert!(health
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("official sdk package is not resolved"));
    }

    #[test]
    fn health_rejects_openclaw_gateway_without_local_package_in_production_profile() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);
        let _gateway = EnvVarGuard::set(OPENCLAW_GATEWAY_URL_ENV, Some("http://127.0.0.1:43190"));
        assert!(!mock_provider_invocation_allowed());

        let runtime = NodeSdkBackendRuntime::bootstrap("openclaw");
        let health = runtime.health();

        assert_eq!(health.status, SdkDriverStatus::Unhealthy);
        assert!(health
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("official sdk package is not resolved"));
    }

    #[test]
    fn package_probe_is_healthy_when_real_cli_runtime_is_available() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);

        let health = package_probe_health(
            "@openai/codex-sdk",
            &json!({
                "package_resolved": false,
                "cli_available": true,
                "runtime_available": true,
                "runtime_mode": "sdk_cli"
            }),
        );

        assert_eq!(health.status, SdkDriverStatus::Healthy);
    }

    #[test]
    fn package_probe_is_unhealthy_without_package_or_real_runtime_in_production() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(
            KERNEL_PROFILE_ID_ENV,
            Some("cloud.split-services.production"),
        );
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);

        let health = package_probe_health(
            "@openai/codex-sdk",
            &json!({
                "package_resolved": false,
                "cli_available": false,
                "runtime_available": false
            }),
        );

        assert_eq!(health.status, SdkDriverStatus::Unhealthy);
    }

    #[test]
    fn package_probe_is_healthy_when_official_package_is_resolved() {
        let health = package_probe_health(
            "@openai/codex-sdk",
            &json!({
                "package_resolved": true,
                "runtime_available": true,
                "runtime_mode": "sdk_live"
            }),
        );

        assert_eq!(health.status, SdkDriverStatus::Healthy);
    }
}
