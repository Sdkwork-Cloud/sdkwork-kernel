use sdkwork_agent_kernel::{
    check_message_against_model_descriptor, validate_structured_model_input, AgentDefinition,
    AgentInputModality, AgentInputPolicy, AgentMessage, AgentMessageRole, AgentPart,
    ModelDescriptor, UnsupportedInputModalityAction,
};

#[test]
fn agent_input_policy_defaults_to_text_and_json() {
    let policy = AgentInputPolicy::default();
    assert!(policy.accepts_modality(AgentInputModality::Text));
    assert!(policy.accepts_modality(AgentInputModality::Json));
    assert!(!policy.accepts_modality(AgentInputModality::Audio));
}

#[test]
fn multimodal_policy_accepts_voice_image_and_video_parts() {
    let policy = AgentInputPolicy::multimodal_chat();
    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![
            AgentPart::text("part.text", "describe this"),
            AgentPart::audio_ref("part.audio", "host://audio/note.ogg", "audio/ogg"),
            AgentPart::image_ref("part.image", "host://images/photo.png", "image/png"),
            AgentPart::video_ref("part.video", "host://video/clip.mp4", "video/mp4"),
        ],
    );

    assert!(policy.accepts_modality(AgentInputModality::Audio));
    assert!(policy.accepts_modality(AgentInputModality::Video));
    validate_structured_model_input(&[message], &policy, None).expect("policy accepts parts");
}

#[test]
fn model_descriptor_rejects_audio_when_only_text_is_supported() {
    let descriptor =
        ModelDescriptor::new("gpt-text-only", "provider.model.fake", "Text Only", "fake")
            .with_input_mode("text");

    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![AgentPart::audio_ref(
            "part.audio",
            "host://audio/note.ogg",
            "audio/ogg",
        )],
    );

    let compatibility = check_message_against_model_descriptor(&message, &descriptor);
    assert!(!compatibility.is_fully_supported());
    assert_eq!(
        compatibility.unsupported_modalities,
        [AgentInputModality::Audio]
    );

    let policy = AgentInputPolicy::multimodal_chat();
    let error = validate_structured_model_input(&[message], &policy, Some(&descriptor))
        .expect_err("text-only model must reject audio input");
    assert!(error.to_string().contains("model.multimodal_input"));
}

#[test]
fn model_descriptor_accepts_multimodal_parts_when_modes_declared() {
    let descriptor = ModelDescriptor::new(
        "gpt-vision-audio",
        "provider.model.fake",
        "Vision Audio",
        "fake",
    )
    .with_input_mode("text")
    .with_input_mode("image")
    .with_input_mode("audio")
    .with_capability("model.multimodal_input");

    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![
            AgentPart::text("part.text", "listen and look"),
            AgentPart::audio_ref("part.audio", "host://audio/note.ogg", "audio/ogg"),
            AgentPart::image_ref("part.image", "host://images/photo.png", "image/png"),
        ],
    );

    assert!(descriptor.supports_multimodal_input());
    let compatibility = check_message_against_model_descriptor(&message, &descriptor);
    assert!(compatibility.is_fully_supported());
}

#[test]
fn agent_definition_parses_input_policy_from_json() {
    const DEFINITION_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent_definition",
  "definition_id": "definition.intelligence.multimodal",
  "agent": {
    "schema_version": "0.1.0",
    "manifest_type": "agent",
    "agent_id": "agent.intelligence.multimodal",
    "name": "multimodal-agent",
    "display_name": "Multimodal Agent",
    "description": "Accepts voice and vision input.",
    "version": "0.1.0",
    "domain": "intelligence",
    "required_capabilities": [
      { "capability_id": "model.chat", "min_version": "0.1.0" }
    ],
    "optional_capabilities": [],
    "event_families": ["agent.runtime.*"],
    "owner": { "name": "sdkwork-platform" },
    "status": "candidate"
  },
  "provider_bindings": [
    {
      "binding_id": "binding.model.primary",
      "family": "model",
      "provider_id": "provider.model.openai",
      "required": true,
      "default": true,
      "mode": "typed_local",
      "capabilities": ["model.catalog", "model.chat", "model.multimodal_input"]
    }
  ],
  "model_selection": {
    "required_capabilities": ["model.chat"],
    "allow_provider_fallback": false
  },
  "tool_call_policy": {
    "policy_required": true,
    "allowed_tool_ids": [],
    "denied_tool_ids": []
  },
  "memory_strategy": {
    "enabled_scopes": [],
    "write_policy_required": true,
    "read_policy_required_for_sensitive": true,
    "retention_required": false
  },
  "input_policy": {
    "accepted_modalities": ["text", "json", "audio", "image", "video"],
    "require_model_support": true,
    "unsupported_action": "reject"
  }
}
"#;

    let definition = AgentDefinition::from_json(DEFINITION_JSON).expect("definition parses");
    assert!(definition
        .input_policy
        .accepts_modality(AgentInputModality::Audio));
    assert!(definition
        .input_policy
        .accepts_modality(AgentInputModality::Video));

    let voice_message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![AgentPart::audio_ref(
            "part.audio",
            "host://audio/note.ogg",
            "audio/ogg",
        )],
    );
    definition
        .accepts_message_input(&voice_message)
        .expect("definition accepts voice input");
}

#[test]
fn strip_unsupported_parts_keeps_supported_modalities() {
    let descriptor = ModelDescriptor::new("gpt-vision", "provider.model.fake", "Vision", "fake")
        .with_input_mode("text")
        .with_input_mode("image");

    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![
            AgentPart::text("part.text", "look at this"),
            AgentPart::audio_ref("part.audio", "host://audio/note.ogg", "audio/ogg"),
            AgentPart::image_ref("part.image", "host://images/photo.png", "image/png"),
        ],
    );

    let policy = AgentInputPolicy::multimodal_chat()
        .with_unsupported_action(UnsupportedInputModalityAction::StripUnsupported);
    let normalized = validate_structured_model_input(&[message], &policy, Some(&descriptor))
        .expect("strip audio");
    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].parts.len(), 2);
    assert!(normalized[0]
        .parts
        .iter()
        .all(|part| part.input_modality() != Some(AgentInputModality::Audio)));
}

#[cfg(feature = "sdkwork-models")]
mod catalog_contracts {
    use sdkwork_agent_kernel::{
        catalog, AgentInputModality, AgentMessage, AgentMessageRole, AgentPart,
    };
    use sdkwork_models::{ModelInfo, SourceEvidence};

    fn fixture_model(input_modalities: Vec<&str>) -> ModelInfo {
        ModelInfo {
            catalog_key: "vendor/model".to_string(),
            model_id: "model".to_string(),
            display_name: "Fixture".to_string(),
            vendor_code: "vendor".to_string(),
            region_code: "global".to_string(),
            vendor_name: None,
            family_code: "chat".to_string(),
            primary_capability: "chat".to_string(),
            capabilities: vec![],
            input_modalities: input_modalities.into_iter().map(str::to_string).collect(),
            output_modalities: vec!["text".to_string()],
            api_format: "openai".to_string(),
            context_tokens: Some(128_000),
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            supports_tools: true,
            supports_json_schema: false,
            rank_score: None,
            lifecycle: "active".to_string(),
            release_stage: "ga".to_string(),
            shelf_state: "listed".to_string(),
            routing_state: "routable".to_string(),
            replacement_model: None,
            description: None,
            strengths: vec![],
            color_token: None,
            latency_p50_ms: None,
            latency_p95_ms: None,
            win_rate: None,
            trend_score: None,
            source: SourceEvidence {
                source_url: "https://example.test".to_string(),
                observed_at: "2026-01-01T00:00:00Z".to_string(),
                source_hash: None,
            },
        }
    }

    #[test]
    fn catalog_model_supports_speech_input_predicate() {
        let speech_model = fixture_model(vec!["text", "audio"]);
        let text_model = fixture_model(vec!["text"]);

        assert!(catalog::catalog_model_supports_modality(
            &speech_model,
            AgentInputModality::Audio
        ));
        assert!(!catalog::catalog_model_supports_modality(
            &text_model,
            AgentInputModality::Audio
        ));
    }

    #[test]
    fn catalog_bridge_builds_model_descriptor_with_multimodal_capability() {
        let model = fixture_model(vec!["text", "image", "audio"]);
        let descriptor = catalog::descriptor_from_catalog_model(&model, "provider.model.catalog");
        assert!(descriptor.supports_input_modality(AgentInputModality::Audio));
        assert!(descriptor.supports_multimodal_input());

        let message = AgentMessage::new(
            "message.1",
            AgentMessageRole::User,
            vec![AgentPart::audio_ref(
                "part.audio",
                "host://audio/note.ogg",
                "audio/ogg",
            )],
        );
        let compatibility = catalog::check_message_against_catalog_model(&message, &model);
        assert!(compatibility.is_fully_supported());
    }
}

#[test]
fn interaction_contract_parses_modality_slots_with_preprocess_delivery() {
    const DEFINITION_JSON: &str = r#"
{
  "schema_version": "1.0.0",
  "manifest_type": "agent_definition",
  "definition_id": "definition.agent.voice",
  "agent": {
    "schema_version": "1.0.0",
    "manifest_type": "agent",
    "agent_id": "agent.intelligence.voice",
    "name": "voice-agent",
    "display_name": "Voice Agent",
    "description": "Accepts voice with STT preprocessing.",
    "version": "1.0.0",
    "domain": "intelligence",
    "required_capabilities": [
      { "capability_id": "model.chat", "min_version": "1.0.0" }
    ],
    "optional_capabilities": [],
    "event_families": ["agent.runtime.*"],
    "owner": { "name": "sdkwork-platform" },
    "status": "candidate"
  },
  "provider_bindings": [{
    "binding_id": "binding.model.default",
    "family": "model",
    "provider_id": "provider.model.fake",
    "required": true,
    "default": true,
    "mode": "manifest_or_typed",
    "capabilities": ["model.chat"]
  }],
  "model_selection": {
    "required_capabilities": [],
    "allow_provider_fallback": false
  },
  "tool_call_policy": {
    "policy_required": true,
    "allowed_tool_ids": [],
    "denied_tool_ids": []
  },
  "memory_strategy": {
    "enabled_scopes": [],
    "write_policy_required": true,
    "read_policy_required_for_sensitive": true,
    "retention_required": false
  },
  "interaction_contract": {
    "schema_version": "1.0.0",
    "input": {
      "slots": [
        { "modality": "text", "enabled": true },
        {
          "modality": "audio",
          "enabled": true,
          "delivery": {
            "strategy": "preprocess",
            "processor_id": "skill.speech_to_text",
            "output_modality": "text"
          }
        }
      ],
      "require_model_support": true,
      "unsupported_action": "reject"
    },
    "output": { "modalities": ["text", "json"] }
  }
}
"#;

    let definition =
        AgentDefinition::from_json(DEFINITION_JSON).expect("parse interaction contract");
    assert!(definition
        .interaction_contract
        .input
        .accepts_modality(AgentInputModality::Audio));
    assert!(matches!(
        definition
            .interaction_contract
            .input
            .delivery_for(AgentInputModality::Audio),
        sdkwork_agent_kernel::ModelDeliveryStrategy::Preprocess { .. }
    ));
    assert_eq!(
        definition.input_policy.accepted_modalities,
        definition
            .interaction_contract
            .input_policy()
            .accepted_modalities
    );
}

#[test]
fn content_reference_parses_host_and_artifact_schemes() {
    let host_part = AgentPart::image_ref("part.image", "host://images/photo.png", "image/png");
    let host_ref = host_part
        .content_reference()
        .expect("parse")
        .expect("reference");
    assert_eq!(
        host_ref.scheme,
        sdkwork_agent_kernel::ContentReferenceScheme::Host
    );

    let artifact_part = AgentPart::artifact_ref("part.artifact", "artifact.abc123");
    let artifact_ref = artifact_part
        .content_reference()
        .expect("parse")
        .expect("reference");
    assert_eq!(
        artifact_ref.scheme,
        sdkwork_agent_kernel::ContentReferenceScheme::Artifact
    );
    assert!(artifact_ref.uri.contains("artifact.abc123"));
}

#[test]
fn chat_rpc_payload_parses_structured_multimodal_parts() {
    let payload = r#"{
        "message_id": "message.rpc.1",
        "role": "user",
        "parts": [
            { "part_id": "part.text", "kind": "text", "text": "describe this" },
            {
                "part_id": "part.image",
                "kind": "image_ref",
                "content_ref": "host://images/photo.png",
                "mime_type": "image/png"
            }
        ]
    }"#;

    let messages =
        sdkwork_agent_kernel::parse_chat_rpc_payload("rpc.1", payload).expect("parse payload");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].parts.len(), 2);
    assert_eq!(
        messages[0].parts[1].content_ref.as_deref(),
        Some("host://images/photo.png")
    );
}

struct StubPreprocessor;

impl sdkwork_agent_kernel::InputModalityPreprocessor for StubPreprocessor {
    fn preprocess_part(
        &self,
        _processor_id: &str,
        part: &AgentPart,
        output_modality: AgentInputModality,
    ) -> sdkwork_agent_kernel::KernelResult<AgentPart> {
        assert_eq!(output_modality, AgentInputModality::Text);
        Ok(AgentPart::text(
            format!("{}.preprocessed", part.part_id),
            "transcribed text",
        ))
    }
}

#[test]
fn preprocess_delivery_transforms_audio_before_text_only_model_check() {
    use sdkwork_agent_kernel::{
        resolve_model_input_with_options, AgentInputContract, ModalitySlot, ModelDeliveryStrategy,
        ModelInputResolveOptions,
    };

    let mut contract = AgentInputContract::multimodal_chat();
    contract.slots = vec![
        ModalitySlot::enabled(AgentInputModality::Text),
        ModalitySlot::enabled(AgentInputModality::Audio).with_delivery(
            ModelDeliveryStrategy::Preprocess {
                processor_id: "skill.speech_to_text".to_string(),
                output_modality: AgentInputModality::Text,
            },
        ),
    ];
    let policy = contract.to_legacy_policy();
    let message = AgentMessage::new(
        "message.1",
        AgentMessageRole::User,
        vec![AgentPart::audio_ref(
            "part.audio",
            "host://audio/note.ogg",
            "audio/ogg",
        )],
    );
    let descriptor =
        ModelDescriptor::new("gpt-text-only", "provider.model.fake", "Text Only", "fake")
            .with_input_mode("text");

    let options = ModelInputResolveOptions {
        input_policy: &policy,
        input_contract: Some(&contract),
        model_descriptor: Some(&descriptor),
        preprocessor: Some(&StubPreprocessor),
    };
    let resolution = resolve_model_input_with_options(&[message], &options).expect("resolved");
    assert_eq!(resolution.messages.len(), 1);
    assert_eq!(
        resolution.messages[0].parts[0].text.as_deref(),
        Some("transcribed text")
    );
}
