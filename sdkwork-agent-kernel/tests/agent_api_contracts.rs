use sdkwork_agent_kernel::{
    api::{
        AgentConversation, AgentInvokeRequest, ContentBlock, ConversationRole,
        InteractionContractBuilder, MessageBuilder,
    },
    AgentMessageRole, ContentReference,
};

#[test]
fn conversation_role_maps_to_industry_wire_roles() {
    assert_eq!(ConversationRole::Assistant.as_openai_role(), "assistant");
    assert_eq!(ConversationRole::Assistant.as_gemini_role(), "model");
    assert_eq!(ConversationRole::Assistant.as_a2a_role(), "agent");
    assert_eq!(
        ConversationRole::from_kernel_role(AgentMessageRole::Model),
        Some(ConversationRole::Assistant)
    );
}

#[test]
fn content_block_round_trips_image_part() {
    let part = ContentBlock::image(ContentReference::host("images/photo.png"), "image/png")
        .to_part("part.image")
        .expect("part");

    let block = ContentBlock::from_part(&part).expect("block");
    assert!(matches!(block, ContentBlock::Image { .. }));
}

#[test]
fn message_builder_creates_multimodal_user_turn() {
    let message = MessageBuilder::user()
        .text("describe this")
        .block(ContentBlock::image(
            ContentReference::host("images/photo.png"),
            "image/png",
        ))
        .build("message.1")
        .expect("message");

    assert_eq!(message.parts.len(), 2);
}

#[test]
fn invoke_request_projects_single_canonical_model_input() {
    let conversation = AgentConversation::new()
        .user_text("message.1", "hello")
        .expect("conversation");

    let interaction = InteractionContractBuilder::text_chat()
        .build()
        .expect("contract");

    let invoke = AgentInvokeRequest::builder("invoke.1")
        .conversation(conversation)
        .interaction(interaction)
        .model_id("gpt-4o")
        .build()
        .expect("invoke");

    let model_request = invoke
        .to_model_request("policy.invoke.1")
        .expect("model request");

    assert!(!model_request.input_messages.is_empty());
    assert_eq!(model_request.messages, vec!["hello".to_string()]);
    assert!(model_request.input_contract.is_some());
    assert!(model_request.input_policy.is_some());
}

#[test]
fn invoke_request_maps_to_chat_and_execution_surfaces() {
    let conversation = AgentConversation::new()
        .system_text("message.system", "You are helpful.")
        .expect("conversation")
        .user_text("message.user", "ping")
        .expect("conversation");

    let invoke = AgentInvokeRequest::from_conversation("exec.1", conversation)
        .to_chat_request()
        .expect("chat");

    assert_eq!(invoke.input_messages.len(), 2);
    assert!(!invoke.messages.is_empty());
}
