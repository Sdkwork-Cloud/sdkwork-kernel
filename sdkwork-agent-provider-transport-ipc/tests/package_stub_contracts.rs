use sdkwork_agent_provider_transport_ipc::{
    JsonRpcTransport, PackageStubJsonRpcTransport, SDKWORK_CAPABILITY_INVOKE_METHOD,
};
use serde_json::json;
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
fn package_stub_returns_model_chat_messages() {
    let _lock = env_lock();
    let _profile = EnvVarGuard::set(KERNEL_PROFILE_ID_ENV, None);
    let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("development"));
    let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, Some("1"));

    let transport = PackageStubJsonRpcTransport::new("run_agent", "python_process");
    let result = transport
        .call(
            SDKWORK_CAPABILITY_INVOKE_METHOD,
            Some(json!({
                "operation": {
                    "operation": "model_chat",
                    "model_request_id": "req-1",
                    "messages": ["hello"]
                }
            })),
        )
        .expect("invoke should succeed");
    let messages = result
        .get("messages")
        .and_then(|value| value.as_array())
        .expect("messages array");
    assert!(messages[0].as_str().unwrap().contains("run_agent"));
}

#[test]
fn package_stub_fails_closed_for_synthetic_operations_in_production_profile() {
    let _lock = env_lock();
    let _profile = EnvVarGuard::set(
        KERNEL_PROFILE_ID_ENV,
        Some("cloud.split-services.production"),
    );
    let _environment = EnvVarGuard::set(KERNEL_ENVIRONMENT_ENV, Some("production"));
    let _allow = EnvVarGuard::set(ALLOW_MOCK_PROVIDERS_ENV, None);

    let transport = PackageStubJsonRpcTransport::new("run_agent", "python_process");
    for operation in [
        json!({ "operation": "session_create", "agent_id": "agent.1" }),
        json!({ "operation": "model_chat", "model_request_id": "req.1", "messages": ["hello"] }),
        json!({ "operation": "tool_invoke", "tool_call_id": "tool.1", "tool_id": "shell", "arguments": "{}" }),
        json!({ "operation": "unimplemented_operation" }),
    ] {
        let result = transport
            .call(
                SDKWORK_CAPABILITY_INVOKE_METHOD,
                Some(json!({ "operation": operation })),
            )
            .expect("stub transport should return a fail-closed payload");

        assert_eq!(
            result.get("ok").and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            result.get("mode").and_then(|value| value.as_str()),
            Some("sdk_live_failed")
        );
        assert!(result
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .contains("production profile"));
    }
}
