use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
use sdkwork_agent_provider_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeError, SdkRuntimeOperation,
    SdkRuntimeRequest, SdkRuntimeResponse,
};
use sdkwork_agent_provider_transport_ipc::{
    provider_worker_concurrency_limit, FailClosedJsonRpcTransport, JsonRpcTransport,
    PackageStubJsonRpcTransport, SpawnedWorker, SpawnedWorkerLease, SpawnedWorkerPool,
    TransportError, SDKWORK_CAPABILITY_INVOKE_METHOD, SDKWORK_PING_METHOD,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

const WORKER_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);
const HEALTH_WORKER_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_WORKER_OPERATION_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_WORKER_OPERATION_TIMEOUT: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct PythonWorkerLaunchOptions {
    pub python_binary: String,
    pub worker_script: PathBuf,
    pub package_name: String,
}

impl PythonWorkerLaunchOptions {
    pub fn for_package(package_name: impl Into<String>) -> Self {
        Self {
            python_binary: default_python_binary(),
            worker_script: default_python_worker_script(),
            package_name: package_name.into(),
        }
    }
}

pub fn default_python_binary() -> String {
    if cfg!(windows) {
        "python".to_string()
    } else {
        "python3".to_string()
    }
}

pub fn default_python_worker_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts/provider-transport-workers/generic_python_sdk_worker.py")
}

enum PythonRuntimeBackend {
    Stub(Arc<dyn JsonRpcTransport + Send + Sync>),
    FailClosed(Arc<dyn JsonRpcTransport + Send + Sync>),
    Managed { pool: Arc<SpawnedWorkerPool> },
}

pub struct PythonSdkBackendRuntime {
    package_name: String,
    backend: PythonRuntimeBackend,
}

impl PythonSdkBackendRuntime {
    pub fn bootstrap(package_name: impl Into<String>) -> Self {
        let options = PythonWorkerLaunchOptions::for_package(package_name);
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
            backend: PythonRuntimeBackend::Stub(transport),
        }
    }

    pub fn spawn(options: &PythonWorkerLaunchOptions) -> Result<Self, SdkRuntimeError> {
        if !options.worker_script.exists() {
            return Err(SdkRuntimeError::new(
                "worker_script_missing",
                format!(
                    "python worker script not found: {}",
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
            backend: PythonRuntimeBackend::Managed { pool },
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
            backend: PythonRuntimeBackend::Stub(Arc::new(PackageStubJsonRpcTransport::new(
                package_name,
                "python_process",
            ))),
        }
    }

    pub fn fail_closed(package_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            backend: PythonRuntimeBackend::FailClosed(Arc::new(FailClosedJsonRpcTransport::new(
                reason.into(),
            ))),
        }
    }

    fn shared_transport(&self) -> Result<Arc<dyn JsonRpcTransport + Send + Sync>, SdkRuntimeError> {
        match &self.backend {
            PythonRuntimeBackend::Stub(transport) | PythonRuntimeBackend::FailClosed(transport) => {
                Ok(transport.clone())
            }
            PythonRuntimeBackend::Managed { .. } => Err(SdkRuntimeError::new(
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
            PythonRuntimeBackend::Managed { pool } => {
                let lease = pool
                    .acquire_internal("health", HEALTH_WORKER_ACQUIRE_TIMEOUT)
                    .map_err(map_transport_error)?;
                lease
                    .call_with_timeout(SDKWORK_PING_METHOD, None, HEALTH_WORKER_ACQUIRE_TIMEOUT)
                    .map_err(map_transport_error)
            }
            PythonRuntimeBackend::Stub(transport) | PythonRuntimeBackend::FailClosed(transport) => {
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
            PythonRuntimeBackend::Managed { pool } => {
                let lease = Self::acquire_worker(pool, request)?;
                lease
                    .call_with_timeout(
                        SDKWORK_CAPABILITY_INVOKE_METHOD,
                        Some(params),
                        worker_operation_timeout(request),
                    )
                    .map_err(map_transport_error)
            }
            PythonRuntimeBackend::Stub(_) | PythonRuntimeBackend::FailClosed(_) => self
                .shared_transport()?
                .call(SDKWORK_CAPABILITY_INVOKE_METHOD, Some(params))
                .map_err(map_transport_error),
        }
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
        });
        match &self.backend {
            PythonRuntimeBackend::Managed { pool } => {
                let lease = Self::acquire_worker(pool, request)?;
                lease
                    .call_streaming_with_timeout(
                        SDKWORK_CAPABILITY_INVOKE_METHOD,
                        Some(params),
                        worker_operation_timeout(request),
                        &mut |frame| {
                            sink(frame).map_err(|error| TransportError::new(error.message))
                        },
                    )
                    .map_err(map_transport_error)
            }
            PythonRuntimeBackend::Stub(_) | PythonRuntimeBackend::FailClosed(_) => self
                .shared_transport()?
                .call_streaming(
                    SDKWORK_CAPABILITY_INVOKE_METHOD,
                    Some(params),
                    &mut |frame| sink(frame).map_err(|error| TransportError::new(error.message)),
                )
                .map_err(map_transport_error),
        }
    }
}

impl SdkBackendRuntime for PythonSdkBackendRuntime {
    fn backend_kind(&self) -> SdkBackendKind {
        SdkBackendKind::PythonProcess
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
                SdkBackendKind::PythonProcess,
                &request.capability_id,
                payload,
            ));
        }

        let payload = self.invoke_worker(request)?;
        if payload.get("ok").and_then(Value::as_bool) == Some(false) {
            let message = payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("python worker invoke failed")
                .to_string();
            return Ok(SdkRuntimeResponse::failure(
                SdkBackendKind::PythonProcess,
                message,
            ));
        }
        Ok(SdkRuntimeResponse::success(
            SdkBackendKind::PythonProcess,
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
            PythonRuntimeBackend::Managed { pool } => {
                pool.cancel(request_id).map_err(map_transport_error)
            }
            PythonRuntimeBackend::Stub(_) | PythonRuntimeBackend::FailClosed(_) => Ok(false),
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

fn spawn_worker(options: &PythonWorkerLaunchOptions) -> Result<SpawnedWorker, TransportError> {
    let mut command = Command::new(&options.python_binary);
    command
        .arg(&options.worker_script)
        .arg("--package")
        .arg(&options.package_name);
    SpawnedWorker::spawn(command)
}

fn map_transport_error(error: TransportError) -> SdkRuntimeError {
    SdkRuntimeError::new("transport_error", error.message)
}

fn package_probe_health(package_name: &str, payload: &Value) -> SdkDriverHealth {
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

    const KERNEL_PROFILE_ID_ENV: &str = "SDKWORK_KERNEL_PROFILE_ID";
    const KERNEL_ENVIRONMENT_ENV: &str = "SDKWORK_KERNEL_ENVIRONMENT";
    const ALLOW_MOCK_PROVIDERS_ENV: &str = "SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS";

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
        let script = default_python_worker_script();
        assert!(
            script.exists(),
            "default Python worker script must exist: {}",
            script.display()
        );
    }

    #[test]
    fn in_memory_stub_invokes_ping() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(KERNEL_PROFILE_ID_ENV, None);
        let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("development"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, Some("1"));

        let runtime = PythonSdkBackendRuntime::in_memory_stub("run_agent", true);
        let response = runtime
            .invoke(&SdkRuntimeRequest::ping("sdk.model.chat"))
            .expect("ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::PythonProcess);
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

        let runtime = PythonSdkBackendRuntime::in_memory_stub("run_agent", true);
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
            "run_agent",
            "python_process",
        ));
        let runtime = PythonSdkBackendRuntime::from_transport(transport, "run_agent");
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

        let runtime = PythonSdkBackendRuntime::bootstrap("sdkwork_missing_python_sdk");
        let health = runtime.health();

        assert_eq!(health.status, SdkDriverStatus::Unhealthy);
        assert!(health
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("official sdk package is not resolved"));
    }
}
