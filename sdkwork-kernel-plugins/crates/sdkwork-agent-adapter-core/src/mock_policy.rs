//! Fail-closed policy for mock/stub provider responses in production profiles.

use sdkwork_agent_kernel::{KernelError, KernelResult};
use serde_json::Value;

const PROFILE_ENV: &str = "SDKWORK_KERNEL_PROFILE_ID";
const ALLOW_MOCK_ENV: &str = "SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS";
const LEGACY_ALLOW_MOCK_ENV: &str = "SDKWORK_KERNEL_ALLOW_MOCK_FALLBACK";

const PRODUCTION_PROFILES: &[&str] = &["prod", "production", "release"];

pub fn kernel_profile_id() -> Option<String> {
    std::env::var(PROFILE_ENV)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

pub fn is_production_kernel_profile() -> bool {
    kernel_profile_id()
        .as_deref()
        .is_some_and(|profile| PRODUCTION_PROFILES.contains(&profile))
}

pub fn mock_provider_invocation_allowed() -> bool {
    if is_production_kernel_profile() {
        return explicit_mock_override_enabled();
    }

    if cfg!(debug_assertions) {
        return !explicit_mock_override_disabled();
    }

    explicit_mock_override_enabled()
}

fn explicit_mock_override_enabled() -> bool {
    [ALLOW_MOCK_ENV, LEGACY_ALLOW_MOCK_ENV]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .any(|value| matches_allow_truthy(&value))
}

fn explicit_mock_override_disabled() -> bool {
    [ALLOW_MOCK_ENV, LEGACY_ALLOW_MOCK_ENV]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .any(|value| matches_deny_falsy(&value))
}

fn matches_allow_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn matches_deny_falsy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

pub fn reject_direct_mock_provider_invocation(operation: &str) -> KernelResult<()> {
    if mock_provider_invocation_allowed() {
        return Ok(());
    }

    Err(KernelError::ProviderUnavailable {
        provider_id: operation.to_string(),
    })
}

pub fn validate_runtime_model_payload(payload: &Value) -> Result<(), String> {
    if mock_provider_invocation_allowed() {
        return Ok(());
    }

    if payload
        .get("mode")
        .and_then(Value::as_str)
        .is_some_and(|mode| mode == "stub" || mode == "sdk_probe" || mode == "sdk_live_failed")
    {
        return Err(format!(
            "runtime backend mode {:?} is not allowed in production profile",
            payload.get("mode")
        ));
    }

    if payload.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("runtime invoke returned ok=false")
            .to_string());
    }

    if let Some(messages) = payload.get("messages").and_then(Value::as_array) {
        for message in messages {
            if let Some(text) = message.as_str() {
                if is_mock_response_text(text) {
                    return Err(
                        "runtime returned mock provider text in production profile".to_string(),
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn is_mock_response_text(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.contains("mock response")
        || normalized.contains(" stub]")
        || normalized.contains("streaming mock")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("mock policy env lock")
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
    fn production_profile_rejects_mock_invocation() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(PROFILE_ENV, Some("prod"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_ENV, None);

        assert!(!mock_provider_invocation_allowed());
        assert!(reject_direct_mock_provider_invocation("model.invoke").is_err());
    }

    #[test]
    fn production_profile_allows_explicit_mock_override() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(PROFILE_ENV, Some("release"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_ENV, Some("1"));

        assert!(mock_provider_invocation_allowed());
    }

    #[test]
    fn rejects_stub_runtime_payload_in_production_profile() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(PROFILE_ENV, Some("prod"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_ENV, None);

        let payload = serde_json::json!({
            "mode": "stub",
            "messages": ["[pkg stub] hello"]
        });
        assert!(validate_runtime_model_payload(&payload).is_err());
    }

    #[test]
    fn allows_sdk_live_runtime_payload_in_production_profile() {
        let _lock = env_lock();
        let _profile = EnvVarGuard::set(PROFILE_ENV, Some("prod"));
        let _allow = EnvVarGuard::set(ALLOW_MOCK_ENV, None);

        let payload = serde_json::json!({
            "mode": "sdk_live",
            "messages": ["official sdk response"]
        });
        assert!(validate_runtime_model_payload(&payload).is_ok());
    }
}
