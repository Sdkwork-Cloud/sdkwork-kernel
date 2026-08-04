//! Contract tests for the provider model configuration read-back SPI.
//!
//! `AgentConfigurationProvider::read_model_configuration` reports the
//! currently effective model configuration from the provider's native config
//! surface so callers can detect drift and stale CLI state. Providers without
//! a readable native surface (in-process providers) return `Unsupported`.

use sdkwork_agent_kernel::{
    AgentConfigurationProvider, ProviderHealth, ProviderModelConfigurationStatus,
    ProviderModelMaterializationState,
};

/// Minimal provider exercising the default read-back implementation.
struct ReadBackUnsupportedProvider;

impl AgentConfigurationProvider for ReadBackUnsupportedProvider {
    fn configuration_spec(
        &self,
        agent_id: &str,
    ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentConfigurationSpec> {
        Ok(sdkwork_agent_kernel::AgentConfigurationSpec::new(agent_id))
    }

    fn validate_configuration(
        &self,
        configuration: &sdkwork_agent_kernel::AgentConfiguration,
    ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentConfigurationValidation>
    {
        Ok(sdkwork_agent_kernel::AgentConfigurationValidation::new(
            &configuration.agent_id,
            &configuration.profile_id,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

#[test]
fn default_read_back_reports_unsupported_with_manifest_scope() {
    let provider = ReadBackUnsupportedProvider;
    let status = provider
        .read_model_configuration("agent.test", "profile.test")
        .expect("default read-back succeeds");
    assert_eq!(
        status.materialization,
        ProviderModelMaterializationState::Unsupported
    );
    assert_eq!(
        status.provider_scope,
        provider.provider_manifest().provider_id
    );
    assert_eq!(status.effective_base_url, None);
    assert_eq!(status.effective_default_model, None);
    assert!(!status.credential_configured);
    assert!(status.issues.is_empty());
}

#[test]
fn materialization_state_vocabulary_is_stable() {
    assert_eq!(
        ProviderModelMaterializationState::Unsupported.as_str(),
        "unsupported"
    );
    assert_eq!(
        ProviderModelMaterializationState::NotMaterialized.as_str(),
        "not_materialized"
    );
    assert_eq!(
        ProviderModelMaterializationState::Materialized.as_str(),
        "materialized"
    );
    assert_eq!(
        ProviderModelMaterializationState::Diverged.as_str(),
        "diverged"
    );
}

#[test]
fn status_constructors_carry_expected_state() {
    let unsupported = ProviderModelConfigurationStatus::unsupported("rig");
    assert_eq!(
        unsupported.materialization,
        ProviderModelMaterializationState::Unsupported
    );
    assert_eq!(unsupported.provider_scope, "rig");

    let not_materialized = ProviderModelConfigurationStatus::not_materialized("codex");
    assert_eq!(
        not_materialized.materialization,
        ProviderModelMaterializationState::NotMaterialized
    );
    assert_eq!(not_materialized.provider_scope, "codex");
}
