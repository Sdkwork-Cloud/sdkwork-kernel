use sdkwork_agent_integration_rig::{ids, RigConfigurationProvider};
use sdkwork_agent_kernel::{
    AgentConfigValue, AgentConfiguration, AgentConfigurationProvider, ProviderHealth,
};

#[test]
fn rig_configuration_spec_requires_secret_refs_and_security_defaults() {
    let provider = RigConfigurationProvider::new();
    let spec = provider
        .configuration_spec(ids::AGENT_ID)
        .expect("rig configuration spec loads");

    assert!(spec.required_keys().contains(&"agent.display_name"));
    assert!(spec.required_keys().contains(&"llm.rig.provider_id"));
    assert!(spec.required_keys().contains(&"llm.rig.api_key"));
    assert!(spec.required_keys().contains(&"runtime.rig.backend_mode"));
    assert!(spec.required_keys().contains(&"security.fail_closed"));
    assert_eq!(provider.health(), ProviderHealth::available());
}

#[test]
fn rig_configuration_rejects_raw_api_keys() {
    let provider = RigConfigurationProvider::new();
    let configuration = AgentConfiguration::new(ids::AGENT_ID, "profile.rig.local")
        .set("agent.display_name", AgentConfigValue::string("Rig"))
        .set("llm.rig.provider_id", AgentConfigValue::string("openai"))
        .set("llm.rig.api_key", AgentConfigValue::string("raw-secret"))
        .set(
            "runtime.rig.backend_mode",
            AgentConfigValue::string("fail_closed"),
        )
        .set("security.fail_closed", AgentConfigValue::string("true"));

    let validation = provider
        .validate_configuration(&configuration)
        .expect("validation report is produced");

    assert!(!validation.is_valid());
    assert!(validation
        .invalid_fields
        .iter()
        .any(|field| field.field_key == "llm.rig.api_key"));
}

#[test]
fn rig_configuration_accepts_secret_ref_profile() {
    let provider = RigConfigurationProvider::new();
    let configuration = AgentConfiguration::new(ids::AGENT_ID, "profile.rig.local")
        .set("agent.display_name", AgentConfigValue::string("Rig"))
        .set("llm.rig.provider_id", AgentConfigValue::string("openai"))
        .set(
            "llm.rig.api_key",
            AgentConfigValue::secret_ref("secret://rig/openai"),
        )
        .set(
            "runtime.rig.backend_mode",
            AgentConfigValue::string("fail_closed"),
        )
        .set("security.fail_closed", AgentConfigValue::string("true"));

    let validation = provider
        .validate_configuration(&configuration)
        .expect("validation report is produced");

    assert!(validation.is_valid());
}
