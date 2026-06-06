use sdkwork_agent_kernel::{
    McpPromptMessage, McpResourceContent, RedactionClassification, TrustLevel,
};

#[test]
fn mcp_resource_content_preserves_trust_redaction_and_context_mapping() {
    let content = McpResourceContent::new(
        "repo://sdkwork/kernel/AGENT_MCP_PROVIDER_SPI_SPEC.md",
        "text/markdown",
        "MCP resources remain first-class context surfaces.",
    )
    .with_trust_level(TrustLevel::RetrievedExternal)
    .with_redaction_classification(RedactionClassification::TenantSensitive)
    .with_metadata("mcp.server_id", "mcp.github");

    assert_eq!(content.trust_level, TrustLevel::RetrievedExternal);
    assert_eq!(
        content.redaction_classification,
        RedactionClassification::TenantSensitive
    );
    assert_eq!(content.metadata_value("mcp.server_id"), Some("mcp.github"));

    let frame = content.to_context_frame("session.mcp.1");

    assert_eq!(
        frame.context_frame_id,
        "context.mcp.resource.repo://sdkwork/kernel/AGENT_MCP_PROVIDER_SPI_SPEC.md"
    );
    assert_eq!(frame.session_id, "session.mcp.1");
    assert_eq!(frame.source, "mcp.resource");
    assert_eq!(frame.content_type, "text/markdown");
    assert_eq!(frame.content, content.content);
    assert_eq!(
        frame.provenance.as_deref(),
        Some("repo://sdkwork/kernel/AGENT_MCP_PROVIDER_SPI_SPEC.md")
    );
    assert_eq!(frame.metadata_value("mcp.server_id"), Some("mcp.github"));
    assert_eq!(frame.trust_level, TrustLevel::RetrievedExternal);
    assert_eq!(
        frame.redaction_classification,
        RedactionClassification::TenantSensitive
    );
    assert!(frame.is_untrusted());
}

#[test]
fn mcp_prompt_message_preserves_trust_redaction_and_context_mapping() {
    let prompt = McpPromptMessage::new(
        "prompt.code-review",
        vec![
            "Review the diff for correctness and security risks.".to_string(),
            "Return findings with file and line references.".to_string(),
        ],
    )
    .with_trust_level(TrustLevel::TrustedHost)
    .with_redaction_classification(RedactionClassification::Internal)
    .with_metadata("mcp.server_id", "mcp.github");

    assert_eq!(prompt.trust_level, TrustLevel::TrustedHost);
    assert_eq!(
        prompt.redaction_classification,
        RedactionClassification::Internal
    );
    assert_eq!(prompt.metadata_value("mcp.server_id"), Some("mcp.github"));

    let frames = prompt.to_context_frames("session.mcp.1");

    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0].context_frame_id,
        "context.mcp.prompt.prompt.code-review.0"
    );
    assert_eq!(frames[0].session_id, "session.mcp.1");
    assert_eq!(frames[0].source, "mcp.prompt");
    assert_eq!(frames[0].content_type, "text/plain");
    assert_eq!(
        frames[0].provenance.as_deref(),
        Some("mcp.prompt:prompt.code-review")
    );
    assert_eq!(frames[0].trust_level, TrustLevel::TrustedHost);
    assert_eq!(
        frames[0].redaction_classification,
        RedactionClassification::Internal
    );
    assert_eq!(
        frames[0].metadata_value("mcp.server_id"),
        Some("mcp.github")
    );
    assert!(!frames[0].is_untrusted());
}
