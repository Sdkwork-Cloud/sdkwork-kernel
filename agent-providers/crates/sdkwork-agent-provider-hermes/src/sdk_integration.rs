use crate::{HermesAdapter, HermesLifecycleProvider, HermesMessageAdapter, HermesModelProvider};
use sdkwork_agent_provider_spi::{
    register_manifest_drivers, AgentSdkBindingManifest, AgentSdkIntegration, BindingRegistry,
    DriverRegistry, SdkBackendKind, SdkDriverHealth, SdkNegotiationError,
    SdkRuntimeBackedModelProvider, SdkRuntimeBackedSessionControlProvider, SdkRuntimeMessageRecord,
    SdkRuntimeOperation, SdkRuntimeRequest, SdkRuntimeResponse, SdkRuntimeRouter,
    SdkRuntimeSessionRecord, StaticCapabilityDriver, HERMES_BINDING_ID, SDK_CAPABILITY_MODEL_CHAT,
};
use sdkwork_agent_provider_transport_core::{
    IpcProtocolTransportHost, ProviderTransportBootstrap, ProviderTransportRegistry,
    PythonProcessTransportHost,
};
use sdkwork_agent_provider_transport_python::PythonSdkBackendRuntime;
use std::sync::Arc;

/// Importable Python module used to probe a live Hermes Agent install (`run_agent` py-module).
pub const HERMES_PYTHON_PROBE_MODULE: &str = "run_agent";
/// TUI gateway JSON-RPC entry (`tui_gateway.server` over stdio).
pub const HERMES_TUI_GATEWAY_MODULE: &str = "tui_gateway";
/// When truthy, prefer `jsonrpc_stdio` IPC via the Hermes TUI gateway module.
pub const HERMES_USE_TUI_GATEWAY_ENV: &str = "SDKWORK_HERMES_USE_TUI_GATEWAY";

const HERMES_BINDING_MANIFEST_JSON: &str =
    include_str!("../../../../bindings/agent-providers/hermes/provider-binding.manifest.json");

const HERMES_IPC_PREFERRED_PYTHON_DRIVERS: &[(&str, &str)] = &[
    (
        "driver.hermes.session.lifecycle.python",
        "sdk.session.lifecycle",
    ),
    ("driver.hermes.model.chat.python", "sdk.model.chat"),
];

/// IPC (TUI gateway) drivers. Their health is tied to
/// `SDKWORK_HERMES_USE_TUI_GATEWAY`: the IPC runtime is only registered when
/// the environment flag is enabled, so the drivers must be unhealthy (and the
/// optional capabilities degraded) otherwise.
const HERMES_IPC_DRIVERS: &[(&str, &str)] = &[
    (
        "driver.hermes.session.lifecycle.ipc",
        "sdk.session.lifecycle",
    ),
    ("driver.hermes.model.chat.ipc", "sdk.model.chat"),
    ("driver.hermes.session.history.ipc", "sdk.session.history"),
    ("driver.hermes.session.control.ipc", "sdk.session.control"),
];

pub fn hermes_binding_manifest() -> AgentSdkBindingManifest {
    AgentSdkBindingManifest::from_json(HERMES_BINDING_MANIFEST_JSON)
        .expect("hermes provider binding manifest must parse")
}

pub fn hermes_prefer_tui_gateway_ipc() -> bool {
    std::env::var(HERMES_USE_TUI_GATEWAY_ENV)
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub struct HermesSdkIntegration {
    pub sdk: AgentSdkIntegration,
    pub transports: ProviderTransportRegistry,
    pub runtime: Arc<SdkRuntimeRouter>,
    pub lifecycle: HermesLifecycleProvider,
    pub model: SdkRuntimeBackedModelProvider,
    pub session_control: SdkRuntimeBackedSessionControlProvider,
    pub session_adapter: HermesAdapter,
    pub message_adapter: HermesMessageAdapter,
}

impl HermesSdkIntegration {
    pub fn bootstrap() -> Result<Self, SdkNegotiationError> {
        let manifest = hermes_binding_manifest();
        let prefer_tui_gateway = hermes_prefer_tui_gateway_ipc();
        let mut drivers = DriverRegistry::new();
        register_hermes_driver_overrides(&mut drivers, prefer_tui_gateway);
        let mut bindings = BindingRegistry::new();
        register_manifest_drivers(&manifest, &mut drivers);
        bindings.register(manifest);
        let negotiation = bindings.negotiate(HERMES_BINDING_ID, &drivers)?;

        let mut bootstrap = ProviderTransportBootstrap::new();
        bootstrap.register_host(Arc::new(PythonProcessTransportHost::new(
            HERMES_PYTHON_PROBE_MODULE,
        )));
        bootstrap.register_host(Arc::new(IpcProtocolTransportHost::new("jsonrpc_stdio")));
        bootstrap.with_python_runtime(Arc::new(PythonSdkBackendRuntime::bootstrap(
            HERMES_PYTHON_PROBE_MODULE,
        )));
        if prefer_tui_gateway {
            bootstrap.with_ipc_runtime(Arc::new(PythonSdkBackendRuntime::bootstrap(
                HERMES_TUI_GATEWAY_MODULE,
            )));
        }
        let (transports, runtime) = bootstrap.finalize_pair(negotiation.clone())?;
        let model = SdkRuntimeBackedModelProvider::new(
            runtime.clone(),
            Arc::new(HermesModelProvider::new()),
            SDK_CAPABILITY_MODEL_CHAT,
            "provider.model.hermes",
        );
        let session_control = SdkRuntimeBackedSessionControlProvider::new(
            runtime.clone(),
            crate::ids::SESSION_CONTROL_PROVIDER_ID,
        );

        Ok(Self {
            sdk: AgentSdkIntegration::new(negotiation),
            transports,
            runtime,
            lifecycle: HermesLifecycleProvider::new(),
            model,
            session_control,
            session_adapter: HermesAdapter::new(),
            message_adapter: HermesMessageAdapter::new(),
        })
    }

    pub fn binding_id(&self) -> &str {
        HERMES_BINDING_ID
    }

    pub fn invoke_runtime(
        &self,
        request: &SdkRuntimeRequest,
    ) -> Result<SdkRuntimeResponse, sdkwork_agent_provider_spi::SdkRuntimeError> {
        self.runtime.invoke(request)
    }

    /// Lists provider sessions through the TUI gateway (`session.list`),
    /// mirroring the Hermes desktop app's session inventory.
    pub fn list_provider_sessions(
        &self,
        working_directory: Option<String>,
    ) -> Result<Vec<SdkRuntimeSessionRecord>, sdkwork_agent_provider_spi::SdkRuntimeError> {
        let request = SdkRuntimeRequest {
            capability_id: "sdk.session.lifecycle".to_string(),
            operation: SdkRuntimeOperation::SessionList {
                working_directory,
                cursor: None,
                limit: sdkwork_agent_provider_spi::SDK_SESSION_PAGE_SIZE,
                source_kinds: None,
                section_id: None,
                archived: None,
                search_term: None,
                sort_key: None,
                sort_direction: None,
                model_providers: None,
            },
            payload: None,
        };
        let response = self.runtime.invoke(&request)?;
        if !response.success {
            return Err(sdkwork_agent_provider_spi::SdkRuntimeError::new(
                "hermes_session_list_failed",
                response
                    .message
                    .unwrap_or_else(|| "hermes session.list failed".to_string()),
            ));
        }
        let payload = response.payload.unwrap_or(serde_json::Value::Null);
        let items = payload
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        items
            .into_iter()
            .map(|item| {
                serde_json::from_value::<SdkRuntimeSessionRecord>(item).map_err(|error| {
                    sdkwork_agent_provider_spi::SdkRuntimeError::new(
                        "hermes_session_record_invalid",
                        format!("hermes session record is invalid: {error}"),
                    )
                })
            })
            .map(|record| {
                record
                    .map_err(|error| {
                        sdkwork_agent_provider_spi::SdkRuntimeError::new(
                            "hermes_session_record_invalid",
                            error.to_string(),
                        )
                    })?
                    .validated("hermes")
                    .map_err(|error| {
                        sdkwork_agent_provider_spi::SdkRuntimeError::new(
                            "hermes_session_record_invalid",
                            error.to_string(),
                        )
                    })
            })
            .collect()
    }

    /// Loads one provider session transcript through the TUI gateway
    /// (`session.resume`), mirroring the Hermes desktop app's history view.
    pub fn get_provider_session_history(
        &self,
        provider_session_id: &str,
    ) -> Result<Vec<SdkRuntimeMessageRecord>, sdkwork_agent_provider_spi::SdkRuntimeError> {
        let request = SdkRuntimeRequest {
            capability_id: "sdk.session.history".to_string(),
            operation: SdkRuntimeOperation::SessionHistory {
                provider_session_id: provider_session_id.to_string(),
                working_directory: None,
                cursor: None,
                limit: sdkwork_agent_provider_spi::SDK_SESSION_PAGE_SIZE,
            },
            payload: None,
        };
        let response = self.runtime.invoke(&request)?;
        if !response.success {
            return Err(sdkwork_agent_provider_spi::SdkRuntimeError::new(
                "hermes_session_history_failed",
                response
                    .message
                    .unwrap_or_else(|| "hermes session.resume failed".to_string()),
            ));
        }
        let payload = response.payload.unwrap_or(serde_json::Value::Null);
        let items = payload
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        items
            .into_iter()
            .map(|item| {
                serde_json::from_value::<SdkRuntimeMessageRecord>(item).map_err(|error| {
                    sdkwork_agent_provider_spi::SdkRuntimeError::new(
                        "hermes_message_record_invalid",
                        format!("hermes message record is invalid: {error}"),
                    )
                })
            })
            .map(|record| {
                record
                    .map_err(|error| {
                        sdkwork_agent_provider_spi::SdkRuntimeError::new(
                            "hermes_message_record_invalid",
                            error.to_string(),
                        )
                    })?
                    .validated(provider_session_id)
                    .map_err(|error| {
                        sdkwork_agent_provider_spi::SdkRuntimeError::new(
                            "hermes_message_record_invalid",
                            error.to_string(),
                        )
                    })
            })
            .collect()
    }
}

fn register_hermes_driver_overrides(drivers: &mut DriverRegistry, prefer_tui_gateway: bool) {
    let ipc_health = || {
        if prefer_tui_gateway {
            SdkDriverHealth::healthy()
        } else {
            SdkDriverHealth::unhealthy(
                "SDKWORK_HERMES_USE_TUI_GATEWAY is not enabled; TUI gateway IPC runtime is unavailable",
            )
        }
    };
    let python_health = || {
        if prefer_tui_gateway {
            SdkDriverHealth::unhealthy("SDKWORK_HERMES_USE_TUI_GATEWAY prefers jsonrpc_stdio IPC")
        } else {
            SdkDriverHealth::healthy()
        }
    };
    for (driver_id, capability_id) in HERMES_IPC_PREFERRED_PYTHON_DRIVERS {
        drivers.register(Arc::new(
            StaticCapabilityDriver::new(*driver_id, *capability_id, SdkBackendKind::PythonProcess)
                .with_health(python_health()),
        ));
    }
    for (driver_id, capability_id) in HERMES_IPC_DRIVERS {
        drivers.register(Arc::new(
            StaticCapabilityDriver::new(*driver_id, *capability_id, SdkBackendKind::IpcProtocol)
                .with_health(ipc_health()),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::{ModelProvider, ModelRequest};
    use sdkwork_agent_provider_spi::SdkBackendKind;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("hermes sdk integration env lock")
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
    fn bootstrap_prefers_python_backend_for_model_chat() {
        let _lock = env_lock();
        let _gateway = EnvVarGuard::set(HERMES_USE_TUI_GATEWAY_ENV, None);
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(integration.binding_id(), HERMES_BINDING_ID);
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::PythonProcess)
        );
    }

    #[test]
    fn bootstrap_prefers_ipc_when_tui_gateway_env_is_set() {
        let _lock = env_lock();
        let _gateway = EnvVarGuard::set(HERMES_USE_TUI_GATEWAY_ENV, Some("1"));
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        assert_eq!(
            integration.sdk.selected_backend_kind("sdk.model.chat"),
            Some(SdkBackendKind::IpcProtocol)
        );
    }

    #[test]
    fn runtime_ping_reaches_python_backend() {
        let _lock = env_lock();
        let _gateway = EnvVarGuard::set(HERMES_USE_TUI_GATEWAY_ENV, None);
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .invoke_runtime(&SdkRuntimeRequest::ping("sdk.model.chat"))
            .expect("runtime ping should succeed");
        assert!(response.success);
        assert_eq!(response.backend_kind, SdkBackendKind::PythonProcess);
    }

    #[test]
    fn model_provider_routes_invoke_through_python_runtime() {
        let _lock = env_lock();
        let _gateway = EnvVarGuard::set(HERMES_USE_TUI_GATEWAY_ENV, None);
        let integration = HermesSdkIntegration::bootstrap().expect("bootstrap should succeed");
        let response = integration
            .model
            .invoke(ModelRequest::new("req-kernel-1", vec!["hello".to_string()]))
            .expect("model invoke should succeed");
        assert!(response
            .messages
            .iter()
            .any(|message| message.contains(HERMES_PYTHON_PROBE_MODULE)));
    }
}
