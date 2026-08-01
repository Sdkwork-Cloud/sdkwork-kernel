use sdkwork_agent_kernel::{
    api::ContentBlock, AgentMessage, AgentMessageRole, AgentPart, ContentReference, ModelRequest,
};
use sdkwork_agent_provider_core::{
    model_request_has_structured_input, model_request_prompt, resolve_model_wire_messages,
    wire_messages_to_openai_json,
};

#[test]
fn model_request_keeps_canonical_and_provider_session_identities_distinct() {
    let request = ModelRequest::new("req.session-identities", vec!["hello".to_string()])
        .for_session("session-canonical")
        .for_provider_session("provider-session-opaque");

    assert_eq!(request.session_id.as_deref(), Some("session-canonical"));
    assert_eq!(
        request.provider_session_id.as_deref(),
        Some("provider-session-opaque")
    );
}

#[test]
fn model_request_prompt_preserves_multimodal_markers() {
    let mut request = ModelRequest::new("req.1", vec!["legacy".to_string()]);
    request.input_messages = vec![AgentMessage::new(
        "msg.1",
        AgentMessageRole::User,
        vec![
            AgentPart::text("part.text", "describe"),
            AgentPart::image_ref(
                "part.image",
                ContentReference::host("images/a.png").uri,
                "image/png",
            ),
        ],
    )];

    let prompt = model_request_prompt(&request);
    assert!(prompt.contains("[image:image/png:"));
    assert!(model_request_has_structured_input(&request));
}

#[test]
fn wire_openai_json_projects_image_url() {
    let mut request = ModelRequest::new("req.1", Vec::new());
    request.input_messages = vec![AgentMessage::new(
        "msg.1",
        AgentMessageRole::User,
        vec![
            AgentPart::text("part.text", "hello"),
            AgentPart::image_ref(
                "part.image",
                ContentReference::parse("https://example.com/a.png")
                    .expect("url")
                    .uri,
                "image/png",
            ),
        ],
    )];

    let wire = resolve_model_wire_messages(&request).expect("wire");
    let json = wire_messages_to_openai_json(&wire).expect("json");
    let content = json[0]["content"].as_array().expect("content");
    assert_eq!(content.len(), 2);
    assert_eq!(content[1]["type"], "image_url");
}

#[test]
fn content_block_round_trip_from_kernel_part() {
    let part = AgentPart::image_ref(
        "part.image",
        ContentReference::host("images/a.png").uri,
        "image/png",
    );
    let block = ContentBlock::from_part(&part).expect("block");
    let restored = block.to_part("part.image").expect("part");
    assert_eq!(restored.kind, part.kind);
}
