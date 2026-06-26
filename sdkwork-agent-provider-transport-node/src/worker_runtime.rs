use sdkwork_agent_provider_core::mock_provider_invocation_allowed;
use sdkwork_agent_provider_transport_ipc::{
    FailClosedJsonRpcTransport, JsonRpcTransport, PackageStubJsonRpcTransport,
    SharedJsonRpcTransport, StdioJsonRpcSession, TransportError, SDKWORK_CAPABILITY_INVOKE_METHOD,
    SDKWORK_PING_METHOD,
};
use sdkwork_agent_provider_spi::{
    SdkBackendKind, SdkBackendRuntime, SdkDriverHealth, SdkRuntimeError, SdkRuntimeOperation,
    SdkRuntimeRequest, SdkRuntimeResponse,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NodeWorkerLaunchOptions {
    pub node_binary: String,
    pub worker_script: PathBuf,
    pub package_name: String,
}

impl NodeWorkerLaunchOptions {
    pub fn for_package(package_name: impl Into<String>) -> Self {
        Self {
            node_binary: "node".to_string(),
            worker_script: default_typescript_worker_script(),
            package_name: package_name.into(),
        }
    }
}

pub fn default_typescript_worker_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/provider-transport-workers/generic-ts-sdk-worker.mjs")
}

pub struct NodeSdkBackendRuntime {
    transport: Arc<dyn JsonRpcTransport + Send + Sync>,
    package_name: String,
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
        Self {
            transport,
            package_name: package_name.into(),
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

        let mut command = Command::new(&options.node_binary);
        command
            .arg(&options.worker_script)
            .arg("--package")
            .arg(&options.package_name);

        let (session, _child) = StdioJsonRpcSession::spawn(command).map_err(map_transport_error)?;
        Ok(Self::from_transport(
            Arc::new(SharedJsonRpcTransport::new(Arc::new(session))),
            &options.package_name,
        ))
    }

    pub fn in_memory_stub(package_name: impl Into<String>, _ping_ok: bool) -> Self {
        let package_name = package_name.into();
        Self::from_transport(
            Arc::new(PackageStubJsonRpcTransport::new(
                package_name.clone(),
                "typescript_node",
            )),
            package_name,
        )
    }

    pub fn fail_closed(package_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::from_transport(
            Arc::new(FailClosedJsonRpcTransport::new(reason.into())),
            package_name,
        )
    }

    fn invoke_worker(&self, request: &SdkRuntimeRequest) -> Result<Value, SdkRuntimeError> {
        let params = json!({
            "capability_id": request.capability_id,
            "operation": request.operation,
            "payload": request.payload,
            "package": self.package_name,
        });
        self.transport
            .call(SDKWORK_CAPABILITY_INVOKE_METHOD, Some(params))
            .map_err(map_transport_error)
    }
}

impl SdkBackendRuntime for NodeSdkBackendRuntime {
    fn backend_kind(&self) -> SdkBackendKind {
        SdkBackendKind::TypeScriptNode
    }

    fn health(&self) -> SdkDriverHealth {
        match self.transport.call(SDKWORK_PING_METHOD, None) {
            Ok(result) if result.get("ok").and_then(Value::as_bool) == Some(true) => {
                SdkDriverHealth::healthy()
            }
            Ok(_) => SdkDriverHealth::degraded("worker ping returned unexpected payload"),
            Err(error) => SdkDriverHealth::unhealthy(error.to_string()),
        }
    }

    fn invoke(&self, request: &SdkRuntimeRequest) -> Result<SdkRuntimeResponse, SdkRuntimeError> {
        if matches!(request.operation, SdkRuntimeOperation::Ping) {
            let payload = self
                .transport
                .call(SDKWORK_PING_METHOD, None)
                .map_err(map_transport_error)?;
            return Ok(SdkRuntimeResponse::success(
                SdkBackendKind::TypeScriptNode,
                &request.capability_id,
                payload,
            ));
        }

        let payload = self.invoke_worker(request)?;
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
}

fn map_transport_error(error: TransportError) -> SdkRuntimeError {
    SdkRuntimeError::new("transport_error", error.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_stub_invokes_ping() {
        let runtime = NodeSdkBackendRuntime::in_memory_stub("openclaw", true);
        let response = runtime
            .invoke(&SdkRuntimeRequest::ping("sdk.session.lifecycle"))
            .expect("ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::TypeScriptNode);
    }
}
