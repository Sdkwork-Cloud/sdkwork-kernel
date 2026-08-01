//! Contract tests for the MCP context injection pipeline.
//!
//! `McpContextInjector` collects server resources and prompts as
//! `ContextFrame` entries (with `mcp:` provenance and trust metadata) and
//! attaches them to model requests, closing the MCP -> context pipeline.

use sdkwork_agent_kernel::{
    AgentManifest, ContextFrame, KernelResult, McpContextInjector, McpPromptDescriptor,
    McpPromptMessage, McpProvider, McpResourceContent, McpResourceDescriptor, McpServerDescriptor,
    McpTransportKind, ModelRequest, ProviderHealth, ProviderManifest, RedactionClassification,
    RuntimeBuilder, TrustLevel,
};

const MCP_INJECT_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.mcp-inject",
  "name": "sdkwork-mcp-inject-agent",
  "display_name": "SDKWork MCP Inject Agent",
  "description": "Agent used to prove MCP context injection contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "mcp.tools",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "mcp.resources",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "mcp.prompts",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.mcp.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

struct ResourcePromptMcpProvider {
    provider_id: String,
}

impl McpProvider for ResourcePromptMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "mcp",
            "resource-prompt-mcp",
            "0.1.0",
            vec![
                "mcp.tools".to_string(),
                "mcp.resources".to_string(),
                "mcp.prompts".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_servers(&self) -> KernelResult<Vec<McpServerDescriptor>> {
        Ok(vec![McpServerDescriptor::new(
            "mcp.context",
            self.provider_id.clone(),
            McpTransportKind::Sse,
        )])
    }

    fn list_resources(&self, _server_id: &str) -> KernelResult<Vec<McpResourceDescriptor>> {
        Ok(vec![McpResourceDescriptor::new(
            "memory://conventions",
            "conventions",
            "text/markdown",
        )])
    }

    fn read_resource(&self, _server_id: &str, uri: &str) -> KernelResult<McpResourceContent> {
        if uri == "memory://conventions" {
            Ok(
                McpResourceContent::new(uri, "text/markdown", "follow the kernel conventions")
                    .with_trust_level(TrustLevel::TrustedSystem)
                    .with_redaction_classification(RedactionClassification::Public),
            )
        } else {
            Err(sdkwork_agent_kernel::KernelError::validation(
                "unknown resource",
            ))
        }
    }

    fn list_prompts(&self, _server_id: &str) -> KernelResult<Vec<McpPromptDescriptor>> {
        Ok(vec![McpPromptDescriptor::new("review", "review")])
    }

    fn get_prompt(
        &self,
        _server_id: &str,
        name: &str,
        _arguments: Vec<(String, String)>,
    ) -> KernelResult<McpPromptMessage> {
        if name == "review" {
            Ok(
                McpPromptMessage::new("review", vec!["review the change".to_string()])
                    .with_trust_level(TrustLevel::TrustedSystem),
            )
        } else {
            Err(sdkwork_agent_kernel::KernelError::validation(
                "unknown prompt",
            ))
        }
    }
}

fn inject_runtime() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.mcp-inject",
        AgentManifest::from_json(MCP_INJECT_AGENT_MANIFEST_JSON).expect("manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .register_mcp_provider(
        "provider.mcp.context",
        "0.1.0",
        ResourcePromptMcpProvider {
            provider_id: "provider.mcp.context".to_string(),
        },
    )
    .bootstrap()
    .expect("mcp inject runtime bootstraps")
    .runtime
}

#[test]
fn injector_collects_resources_and_prompts_as_frames() {
    let runtime = inject_runtime();
    let injector = McpContextInjector::new();

    let frames = injector
        .collect_frames(
            &runtime,
            "session.mcp",
            "mcp.context",
            Some("provider.mcp.context"),
            None,
        )
        .expect("frames collect");

    assert_eq!(frames.len(), 2);

    let resource = frames
        .iter()
        .find(|frame| frame.source == "mcp:mcp.context:resource")
        .expect("resource frame present");
    assert_eq!(resource.content, "follow the kernel conventions");
    assert_eq!(resource.content_type, "text/markdown");
    assert_eq!(
        resource.provenance.as_deref(),
        Some("mcp://mcp.context/resource/memory://conventions")
    );
    assert_eq!(resource.trust_level, TrustLevel::TrustedSystem);

    let prompt = frames
        .iter()
        .find(|frame| frame.source == "mcp:mcp.context:prompt")
        .expect("prompt frame present");
    assert_eq!(prompt.content, "review the change");
    assert_eq!(
        prompt.provenance.as_deref(),
        Some("mcp://mcp.context/prompt/review")
    );
}

#[test]
fn injector_respects_frame_limit() {
    let runtime = inject_runtime();
    let injector = McpContextInjector::new();

    let frames = injector
        .collect_frames(
            &runtime,
            "session.mcp",
            "mcp.context",
            Some("provider.mcp.context"),
            Some(1),
        )
        .expect("limited frames collect");

    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].source, "mcp:mcp.context:resource");
}

#[test]
fn injector_attaches_frames_to_model_request() {
    let runtime = inject_runtime();
    let injector = McpContextInjector::new();

    let request = ModelRequest::new("model.inject.1", vec!["hello".to_string()]);
    let attached = injector
        .attach_frames(
            &runtime,
            "session.mcp",
            "mcp.context",
            Some("provider.mcp.context"),
            None,
            request,
        )
        .expect("frames attach");

    assert_eq!(attached.context_frames.len(), 2);
    assert_eq!(
        attached.context_frames[0].context_frame_id,
        "mcp.resource.mcp.context.memory://conventions"
    );
}

#[test]
fn injector_defaults_to_default_mcp_provider() {
    let runtime = inject_runtime();
    let injector = McpContextInjector::new();

    // Without an explicit provider id the runtime default is used.
    let frames = injector
        .collect_frames(&runtime, "session.mcp", "mcp.context", None, None)
        .expect("default provider frames collect");
    assert_eq!(frames.len(), 2);
}

#[test]
fn frame_id_stability_round_trips() {
    let frame = ContextFrame::new(
        "mcp.resource.mcp.context.memory://conventions",
        "session.mcp",
        "mcp:mcp.context:resource",
        "content",
        TrustLevel::TrustedSystem,
        RedactionClassification::Public,
    );
    let clone = frame.clone();
    assert_eq!(frame, clone);
}
