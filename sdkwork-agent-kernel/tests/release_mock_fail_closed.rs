//! Release-profile fail-closed checks for mock provider policy.

#[cfg(not(debug_assertions))]
#[test]
fn release_profile_rejects_direct_mock_provider_invocation() {
    use sdkwork_agent_kernel::{
        is_production_kernel_profile_from_env, mock_provider_invocation_allowed_from_env,
        ALLOW_MOCK_PROVIDERS_ENV, KERNEL_ENVIRONMENT_ENV, KERNEL_PROFILE_ID_ENV,
    };
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

    let previous_profile = std::env::var(KERNEL_PROFILE_ID_ENV).ok();
    let previous_environment = std::env::var(KERNEL_ENVIRONMENT_ENV).ok();
    let previous_allow = std::env::var(ALLOW_MOCK_PROVIDERS_ENV).ok();

    std::env::set_var(KERNEL_PROFILE_ID_ENV, "cloud.production");
    std::env::set_var(KERNEL_ENVIRONMENT_ENV, "production");
    std::env::remove_var(ALLOW_MOCK_PROVIDERS_ENV);

    assert!(is_production_kernel_profile_from_env());
    assert!(!mock_provider_invocation_allowed_from_env());
    assert!(
        sdkwork_agent_provider_core::reject_direct_mock_provider_invocation("model.invoke")
            .is_err()
    );

    match previous_profile {
        Some(value) => std::env::set_var(KERNEL_PROFILE_ID_ENV, value),
        None => std::env::remove_var(KERNEL_PROFILE_ID_ENV),
    }
    match previous_environment {
        Some(value) => std::env::set_var(KERNEL_ENVIRONMENT_ENV, value),
        None => std::env::remove_var(KERNEL_ENVIRONMENT_ENV),
    }
    match previous_allow {
        Some(value) => std::env::set_var(ALLOW_MOCK_PROVIDERS_ENV, value),
        None => std::env::remove_var(ALLOW_MOCK_PROVIDERS_ENV),
    }
}
