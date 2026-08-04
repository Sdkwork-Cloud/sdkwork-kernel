use sdkwork_agent_kernel::ModelProvider;
use sdkwork_agent_provider_claude_code::ClaudeCodeSdkIntegration;

/// Live smoke test: send one real message through the claude-code provider's
/// official `@anthropic-ai/claude-agent-sdk` TypeScript worker (`query()`) and
/// verify the assistant reply comes back with a correlated provider session
/// identity.
///
/// Requires the installed `@anthropic-ai/claude-agent-sdk` package (kernel
/// workspace node_modules) and Claude Code authentication (OAuth login or
/// `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` with `ANTHROPIC_BASE_URL`).
#[test]
#[ignore = "requires the @anthropic-ai/claude-agent-sdk package and live Claude auth"]
fn live_sdk_model_chat() {
    let integration = ClaudeCodeSdkIntegration::bootstrap().expect("bootstrap claude SDK");
    eprintln!("claude_live_phase=bootstrap_complete");
    let models = integration.model.list_models();
    assert!(!models.is_empty(), "claude SDK must publish models");
    let model_id = models[0].model_id.clone();
    eprintln!("claude_live_phase=models model={model_id}");

    let request = sdkwork_agent_kernel::ModelRequest::new(
        format!("claude-live-{}", std::process::id()),
        vec!["Reply with exactly one word: OK".to_string()],
    )
    .with_model_id(model_id.clone());
    let response = integration
        .model
        .invoke(request)
        .expect("live claude invoke succeeds");
    eprintln!(
        "claude_live_phase=invoke_complete status={:?} finish={:?} provider={}",
        response.status, response.finish_reason, response.provider_id,
    );
    eprintln!("claude_live_phase=reply messages={:?}", response.messages);
    assert_eq!(
        response.status,
        sdkwork_agent_kernel::ModelStatus::Succeeded,
        "live claude invoke must succeed: {response:?}"
    );
    assert!(
        response
            .messages
            .iter()
            .any(|message| !message.trim().is_empty()),
        "live claude invoke must return an assistant reply"
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
        "claude_live_phase=parity_ok provider_session_id={provider_session_id:?} messages={:?}",
        response.messages
    );
    assert!(
        provider_session_id.is_some(),
        "live claude invoke must return a provider session id for conversation resumption"
    );

    // Second turn in the same provider session: the session id must be
    // accepted for resumption and the model must reply again.
    let resumed_request = sdkwork_agent_kernel::ModelRequest::new(
        format!("claude-live-resume-{}", std::process::id()),
        vec!["Reply with exactly one word: DONE".to_string()],
    )
    .with_model_id(model_id.clone())
    .for_provider_session(provider_session_id.expect("provider session id"));
    let resumed_response = integration
        .model
        .invoke(resumed_request)
        .expect("live claude resume invoke succeeds");
    eprintln!(
        "claude_live_phase=resume_complete status={:?} messages={:?}",
        resumed_response.status, resumed_response.messages
    );
    assert_eq!(
        resumed_response.status,
        sdkwork_agent_kernel::ModelStatus::Succeeded,
        "live claude resume must succeed: {resumed_response:?}"
    );
    assert!(
        resumed_response
            .messages
            .iter()
            .any(|message| !message.trim().is_empty()),
        "live claude resume must return an assistant reply"
    );
    eprintln!("claude_live_phase=resume_parity_ok");
}
