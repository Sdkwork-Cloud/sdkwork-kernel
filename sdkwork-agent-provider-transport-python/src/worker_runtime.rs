use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
use sdkwork_agent_provider_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeError, SdkRuntimeOperation,
    SdkRuntimeRequest, SdkRuntimeResponse,
};
use sdkwork_agent_provider_transport_ipc::{
    FailClosedJsonRpcTransport, JsonRpcTransport, PackageStubJsonRpcTransport, SpawnedWorker,
    TransportError, SDKWORK_CAPABILITY_INVOKE_METHOD, SDKWORK_PING_METHOD,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

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
        .join("../../scripts/provider-transport-workers/generic_python_sdk_worker.py")
}

enum PythonRuntimeBackend {
    Stub(Arc<dyn JsonRpcTransport + Send + Sync>),
    FailClosed(Arc<dyn JsonRpcTransport + Send + Sync>),
    Managed {
        worker: Mutex<Option<SpawnedWorker>>,
        launch_options: PythonWorkerLaunchOptions,
    },
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

        let worker = spawn_worker(options).map_err(map_transport_error)?;
        Ok(Self {
            package_name: options.package_name.clone(),
            backend: PythonRuntimeBackend::Managed {
                worker: Mutex::new(Some(worker)),
                launch_options: options.clone(),
            },
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

    fn transport(&self) -> Result<Arc<dyn JsonRpcTransport + Send + Sync>, SdkRuntimeError> {
        match &self.backend {
            PythonRuntimeBackend::Stub(transport) | PythonRuntimeBackend::FailClosed(transport) => {
                Ok(transport.clone())
            }
            PythonRuntimeBackend::Managed {
                worker,
                launch_options,
            } => {
                let mut guard = worker
                    .lock()
                    .map_err(|error| SdkRuntimeError::new("lock_error", error.to_string()))?;
                let needs_respawn = guard
                    .as_ref()
                    .map(|entry| !entry.is_running())
                    .unwrap_or(true);
                if needs_respawn {
                    *guard = Some(spawn_worker(launch_options).map_err(map_transport_error)?);
                }
                let entry = guard
                    .as_ref()
                    .expect("worker should be present after respawn");
                Ok(Arc::new(entry.transport()))
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
        self.transport()?
            .call(SDKWORK_CAPABILITY_INVOKE_METHOD, Some(params))
            .map_err(map_transport_error)
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
        self.transport()?
            .call_streaming(
                SDKWORK_CAPABILITY_INVOKE_METHOD,
                Some(params),
                &mut |frame| sink(frame).map_err(|error| TransportError::new(error.message)),
            )
            .map_err(map_transport_error)
    }
}

impl SdkBackendRuntime for PythonSdkBackendRuntime {
    fn backend_kind(&self) -> SdkBackendKind {
        SdkBackendKind::PythonProcess
    }

    fn health(&self) -> SdkDriverHealth {
        match self.transport() {
            Ok(transport) => match transport.call(SDKWORK_PING_METHOD, None) {
                Ok(result) if result.get("ok").and_then(Value::as_bool) == Some(true) => {
                    SdkDriverHealth::healthy()
                }
                Ok(_) => SdkDriverHealth::degraded("worker ping returned unexpected payload"),
                Err(error) => SdkDriverHealth::unhealthy(error.to_string()),
            },
            Err(error) => SdkDriverHealth::unhealthy(error.message),
        }
    }

    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        if matches!(request.operation, SdkRuntimeOperation::Ping) {
            let payload = self
                .transport()?
                .call(SDKWORK_PING_METHOD, None)
                .map_err(map_transport_error)?;
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
            let payload = self
                .transport()?
                .call(SDKWORK_PING_METHOD, None)
                .map_err(map_transport_error)?;
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

    fn cancel_inflight(&self) -> Result<(), SdkRuntimeError> {
        if let PythonRuntimeBackend::Managed { worker, .. } = &self.backend {
            let guard = worker
                .lock()
                .map_err(|error| SdkRuntimeError::new("lock_error", error.to_string()))?;
            if let Some(entry) = guard.as_ref() {
                entry.cancel_inflight().map_err(map_transport_error)?;
            }
        }
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
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
}
