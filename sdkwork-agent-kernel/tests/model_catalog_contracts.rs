use sdkwork_agent_kernel::{
    AgentManifest, KernelError, KernelResult, ModelDescriptor, ModelProvider, ModelRequest,
    ModelResponse, ModelResponseFormat, ProviderHealth, ProviderManifest, RuntimeBuilder,
    RuntimeState, SideEffectLevel, ToolDescriptor, ToolSchema,
};

const MODEL_CATALOG_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.model-catalog",
  "name": "sdkwork-model-catalog-agent",
  "display_name": "SDKWork Model Catalog Agent",
  "description": "Agent used to prove model catalog SPI contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.catalog",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "model.structured_output",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [
    {
      "capability_id": "model.tool_call",
      "min_version": "0.1.0"
    }
  ],
  "event_families": ["agent.runtime.*", "agent.model.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn model_descriptor_declares_routing_capabilities_limits_and_policy_surface() {
    let descriptor =
        ModelDescriptor::new("model.openai.gpt-5", "provider.openai", "GPT-5", "openai")
            .with_version("2026-05-01")
            .with_capability("chat")
            .with_capability("reasoning")
            .with_capability("tool_call")
            .with_capability("structured_output")
            .with_context_window_tokens(1_000_000)
            .with_max_output_tokens(128_000)
            .with_input_mode("text")
            .with_input_mode("image")
            .with_output_mode("text")
            .with_output_mode("json")
            .with_response_format(ModelResponseFormat::json_schema(
                "sdkwork.answer.schema.v1".to_string(),
            ))
            .with_tool_capability("tool.invoke")
            .with_policy_category("model.invoke")
            .with_policy_category("model.send_sensitive_context")
            .with_metadata("sdkwork.model.routing.tier", "frontier");

    assert_eq!(descriptor.model_id, "model.openai.gpt-5");
    assert_eq!(descriptor.provider_id, "provider.openai");
    assert_eq!(descriptor.family, "openai");
    assert!(descriptor.supports_capability("reasoning"));
    assert!(
        descriptor.supports_response_format(&ModelResponseFormat::json_schema(
            "sdkwork.answer.schema.v1".to_string()
        ))
    );
    assert_eq!(descriptor.context_window_tokens, Some(1_000_000));
    assert_eq!(descriptor.max_output_tokens, Some(128_000));
    assert_eq!(descriptor.input_modes, ["text", "image"]);
    assert_eq!(descriptor.output_modes, ["text", "json"]);
    assert_eq!(descriptor.tool_capabilities, ["tool.invoke"]);
    assert_eq!(
        descriptor.metadata_value("sdkwork.model.routing.tier"),
        Some("frontier")
    );
    assert!(descriptor.requires_policy_for_sensitive_context());
}

#[test]
fn model_request_selects_model_and_attaches_tool_descriptors_explicitly() {
    let tool = ToolDescriptor::new(
        "tool.web.search",
        "provider.tool.web",
        "Web Search",
        SideEffectLevel::ExternalSend,
    )
    .with_input_schema(ToolSchema::json_schema("sdkwork.tool.web.search.input.v1"))
    .with_output_schema(ToolSchema::json_schema("sdkwork.tool.web.search.output.v1"))
    .with_policy_categories(vec!["tool.external_send".to_string()]);

    let request = ModelRequest::new("model-request.1", vec!["search the docs".to_string()])
        .with_model_id("model.openai.gpt-5")
        .with_context_frame("context.repo.1")
        .with_tool_descriptor(tool)
        .with_model_parameter("reasoning.effort", "high");

    assert_eq!(request.model_id.as_deref(), Some("model.openai.gpt-5"));
    assert_eq!(request.context_frame_ids, ["context.repo.1"]);
    assert_eq!(request.tool_descriptors[0].tool_id, "tool.web.search");
    assert_eq!(
        request.metadata_value("model.reasoning.effort"),
        Some("high")
    );
}

#[test]
fn model_provider_exposes_catalog_and_rejects_unknown_model_ids() {
    let provider = CatalogModelProvider;

    let models = provider.list_models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].model_id, "model.openai.gpt-5");
    assert!(models[0].supports_capability("tool_call"));

    let descriptor = provider
        .describe_model("model.local.small")
        .expect("model descriptor exists");
    assert_eq!(descriptor.provider_id, "provider.catalog");

    let selected = provider
        .invoke(
            ModelRequest::new("model-request.1", vec!["hello".to_string()])
                .with_model_id("model.local.small"),
        )
        .expect("selected model invokes");
    assert_eq!(selected.provider_id, "provider.catalog");
    assert_eq!(selected.messages, ["model.local.small:hello"]);

    let error = provider
        .invoke(
            ModelRequest::new("model-request.2", vec!["hello".to_string()])
                .with_model_id("model.missing"),
        )
        .expect_err("unknown model must fail");
    assert!(matches!(
        error,
        KernelError::CapabilityMissing { capability_id } if capability_id == "model.missing"
    ));
}

#[test]
fn runtime_model_registration_uses_provider_manifest_capabilities_for_negotiation() {
    let manifest = AgentManifest::from_json(MODEL_CATALOG_AGENT_MANIFEST_JSON)
        .expect("model catalog manifest parses");
    let report = RuntimeBuilder::new("runtime.model.catalog", manifest)
        .with_generated_at("2026-05-30T00:00:00Z")
        .register_model_provider("provider.catalog", "0.1.0", CatalogModelProvider)
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(report.runtime.state(), RuntimeState::Ready);
    assert!(report
        .runtime
        .capability_manifest()
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "model.catalog"
            && capability.operations == ["list_models", "describe_model", "health"]));
    assert!(report
        .runtime
        .capability_manifest()
        .capabilities
        .iter()
        .any(
            |capability| capability.capability_id == "model.structured_output"
                && capability.provider_id == "provider.catalog"
        ));
    assert_eq!(
        report
            .runtime
            .model_provider()
            .expect("model provider")
            .describe_model("model.openai.gpt-5")
            .expect("catalog descriptor")
            .context_window_tokens,
        Some(1_000_000)
    );
}

struct CatalogModelProvider;

impl CatalogModelProvider {
    fn descriptors(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new("model.openai.gpt-5", "provider.catalog", "GPT-5", "openai")
                .with_capability("chat")
                .with_capability("reasoning")
                .with_capability("tool_call")
                .with_capability("structured_output")
                .with_context_window_tokens(1_000_000)
                .with_policy_category("model.invoke"),
            ModelDescriptor::new(
                "model.local.small",
                "provider.catalog",
                "Local Small",
                "local",
            )
            .with_capability("chat")
            .with_context_window_tokens(32_000)
            .with_policy_category("model.invoke"),
        ]
    }
}

impl ModelProvider for CatalogModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.catalog",
            "model",
            "catalog",
            "0.1.0",
            vec![
                "model.catalog".to_string(),
                "model.chat".to_string(),
                "model.tool_call".to_string(),
                "model.structured_output".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        self.descriptors()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let model_id = request
            .model_id
            .clone()
            .unwrap_or_else(|| "model.openai.gpt-5".to_string());

        let descriptor = self.describe_model(&model_id)?;
        Ok(ModelResponse::text(
            request.model_request_id,
            descriptor.provider_id,
            format!("{model_id}:{}", request.messages.join("\n")),
        ))
    }
}
