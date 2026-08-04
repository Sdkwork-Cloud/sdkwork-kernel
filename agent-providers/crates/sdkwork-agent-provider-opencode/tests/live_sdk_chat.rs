use sdkwork_agent_kernel::ModelProvider;
use sdkwork_agent_provider_opencode::OpenCodeSdkIntegration;

/// Live smoke test: send one real message through the opencode provider's
/// official `@opencode-ai/sdk` TypeScript worker and verify the assistant
/// reply comes back with a correlated provider session identity.
///
/// Requires the installed `@opencode-ai/sdk` package (kernel workspace
/// node_modules) and an opencode model provider configuration. The SDK worker
/// starts an in-process opencode server by default; a running opencode server
/// can be used instead via `OPENCODE_SERVER_URL`.
#[test]
#[ignore = "requires the @opencode-ai/sdk package and a live model provider"]
fn live_sdk_model_chat() {
    let integration = OpenCodeSdkIntegration::bootstrap().expect("bootstrap opencode SDK");
    eprintln!("opencode_live_phase=bootstrap_complete");
    let models = integration.model.list_models();
    assert!(!models.is_empty(), "opencode SDK must publish models");
    // The durable v2 runner resolves models from the server's built-in
    // catalog, not config-file providers: prefer OPENCODE_MODEL (the agents
    // e2e convention) and otherwise pick a catalog model instead of the first
    // (often config-file) entry.
    let model_id = std::env::var("OPENCODE_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            models
                .iter()
                .map(|model| model.model_id.as_str())
                .find(|model_id| model_id.starts_with("opencode/"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "opencode/deepseek-v4-flash-free".to_string());
    eprintln!("opencode_live_phase=models model={model_id}");

    let request = sdkwork_agent_kernel::ModelRequest::new(
        format!("opencode-live-{}", std::process::id()),
        vec!["Reply with exactly one word: OK".to_string()],
    )
    .with_model_id(model_id.clone())
    .with_metadata("sdkwork.code_engine.require_live_provider", "true");
    let response = integration
        .model
        .invoke(request)
        .expect("live opencode invoke succeeds");
    eprintln!(
        "opencode_live_phase=invoke_complete status={:?} finish={:?} provider={}",
        response.status, response.finish_reason, response.provider_id,
    );
    eprintln!("opencode_live_phase=reply messages={:?}", response.messages);
    assert_eq!(
        response.status,
        sdkwork_agent_kernel::ModelStatus::Succeeded,
        "live opencode invoke must succeed: {response:?}"
    );
    assert!(
        response
            .messages
            .iter()
            .any(|message| !message.trim().is_empty()),
        "live opencode invoke must return an assistant reply"
    );
    let provider_session_id = response
        .diagnostics
        .iter()
        .find_map(|diagnostic| {
            let (key, value) = diagnostic.split_once('=')?;
            (key.trim() == "sdk_runtime_provider_session_id").then(|| value.trim().to_string())
        })
        .or_else(|| {
            response.diagnostics.iter().find_map(|diagnostic| {
                let (key, value) = diagnostic.split_once('=')?;
                (key.trim() == "provider_session_id").then(|| value.trim().to_string())
            })
        });
    eprintln!(
        "opencode_live_phase=parity_ok provider_session_id={provider_session_id:?} messages={:?}",
        response.messages
    );
    assert!(
        provider_session_id.is_some(),
        "live opencode invoke must return a provider session id for conversation resumption"
    );

    // Second turn in the same provider session: the session id must be
    // accepted for resumption and the model must reply again.
    let resumed_request = sdkwork_agent_kernel::ModelRequest::new(
        format!("opencode-live-resume-{}", std::process::id()),
        vec!["Reply with exactly one word: DONE".to_string()],
    )
    .with_model_id(model_id.clone())
    .for_provider_session(provider_session_id.expect("provider session id"));
    let resumed_response = integration
        .model
        .invoke(resumed_request)
        .expect("live opencode resume invoke succeeds");
    eprintln!(
        "opencode_live_phase=resume_complete status={:?} messages={:?}",
        resumed_response.status, resumed_response.messages
    );
    assert_eq!(
        resumed_response.status,
        sdkwork_agent_kernel::ModelStatus::Succeeded,
        "live opencode resume must succeed: {resumed_response:?}"
    );
    assert!(
        resumed_response
            .messages
            .iter()
            .any(|message| !message.trim().is_empty()),
        "live opencode resume must return an assistant reply"
    );
    eprintln!("opencode_live_phase=resume_parity_ok");
}
