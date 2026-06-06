use sdkwork_agent_kernel::{
    AgentManifest, AgentSkillDescriptor, AgentSkillInvocationMode, AgentSkillProvider,
    AgentSkillRequest, AgentSkillResult, AgentSkillStatus, KernelResult, McpPromptDescriptor,
    McpPromptMessage, McpProvider, McpResourceContent, McpResourceDescriptor, McpServerDescriptor,
    ModelProvider, ModelRequest, ModelResponse, ProviderHealth, ProviderManifest, RuntimeBuilder,
    SideEffectLevel, ToolCall, ToolDescriptor, ToolResult,
};

const EXTENSIBLE_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.extensible",
  "name": "sdkwork-extensible-agent",
  "display_name": "SDKWork Extensible Agent",
  "description": "Agent used to prove LLM, MCP, and Agent Skill SPI contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
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
    },
    {
      "capability_id": "skill.discover",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "skill.invoke",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.provider.*", "agent.skill.*", "agent.mcp.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[test]
fn runtime_registry_supports_multiple_llm_providers_mcp_and_agent_skills() {
    let manifest = AgentManifest::from_json(EXTENSIBLE_AGENT_MANIFEST_JSON).unwrap();
    let report = RuntimeBuilder::new("runtime.extensible", manifest)
        .with_generated_at("2026-05-29T00:00:00Z")
        .register_model_provider(
            "provider.model.openai",
            "1.0.0",
            StaticModelProvider::new("provider.model.openai", "openai response"),
        )
        .register_model_provider(
            "provider.model.anthropic",
            "1.0.0",
            StaticModelProvider::new("provider.model.anthropic", "anthropic response"),
        )
        .register_mcp_provider(
            "provider.mcp.github",
            "1.0.0",
            FakeMcpProvider::new("provider.mcp.github", "mcp.github", "mcp.github.search"),
        )
        .register_mcp_provider(
            "provider.mcp.gitlab",
            "1.0.0",
            FakeMcpProvider::new("provider.mcp.gitlab", "mcp.gitlab", "mcp.gitlab.search"),
        )
        .register_agent_skill_provider(
            "provider.skill.claude",
            "1.0.0",
            FakeAgentSkillProvider::new(
                "provider.skill.claude",
                "skill.code-review",
                "claude-sonnet",
                "reviewed diff",
            ),
        )
        .register_agent_skill_provider(
            "provider.skill.local",
            "1.0.0",
            FakeAgentSkillProvider::new(
                "provider.skill.local",
                "skill.local-refactor",
                "local-model",
                "refactored locally",
            ),
        )
        .bootstrap()
        .expect("runtime bootstraps");

    assert_eq!(
        report.runtime.model_provider_ids(),
        ["provider.model.openai", "provider.model.anthropic"]
    );

    let default_model = report
        .runtime
        .model_provider()
        .expect("default model provider is registered")
        .invoke(ModelRequest::new(
            "model.default",
            vec!["hello".to_string()],
        ))
        .expect("default model invokes");
    assert_eq!(default_model.provider_id, "provider.model.openai");

    let anthropic_model = report
        .runtime
        .model_provider_by_id("provider.model.anthropic")
        .expect("anthropic model provider is registered")
        .invoke(ModelRequest::new(
            "model.anthropic",
            vec!["hello".to_string()],
        ))
        .expect("selected model invokes");
    assert_eq!(anthropic_model.messages, ["anthropic response"]);

    assert_eq!(
        report.runtime.mcp_provider_ids(),
        ["provider.mcp.github", "provider.mcp.gitlab"]
    );
    let mcp = report
        .runtime
        .mcp_provider()
        .expect("mcp provider is registered");
    assert_eq!(mcp.list_servers()[0].server_id, "mcp.github");
    assert_eq!(
        mcp.list_tools("mcp.github").expect("mcp tools list")[0].tool_id,
        "mcp.github.search"
    );
    assert_eq!(
        mcp.read_resource("mcp.github", "repo://sdkwork/README.md")
            .expect("mcp resource reads")
            .mime_type,
        "text/markdown"
    );
    assert_eq!(
        mcp.get_prompt(
            "mcp.github",
            "prompt.code-review",
            vec![("scope".to_string(), "diff".to_string())],
        )
        .expect("mcp prompt loads")
        .messages,
        ["review diff"]
    );
    let gitlab_mcp = report
        .runtime
        .mcp_provider_by_id("provider.mcp.gitlab")
        .expect("gitlab mcp provider is registered");
    assert_eq!(gitlab_mcp.list_servers()[0].server_id, "mcp.gitlab");
    assert_eq!(
        gitlab_mcp
            .list_tools("mcp.gitlab")
            .expect("gitlab mcp tools list")[0]
            .tool_id,
        "mcp.gitlab.search"
    );

    assert_eq!(
        report.runtime.agent_skill_provider_ids(),
        ["provider.skill.claude", "provider.skill.local"]
    );
    let skills = report
        .runtime
        .agent_skill_provider()
        .expect("agent skill provider is registered");
    assert_eq!(
        skills
            .describe_skill("skill.code-review")
            .expect("skill exists")
            .model_hint
            .as_deref(),
        Some("claude-sonnet")
    );
    let skill_result = skills
        .invoke_skill(
            AgentSkillRequest::new("skill-request.1", "skill.code-review")
                .with_argument("scope", "diff"),
        )
        .expect("skill invokes");
    assert_eq!(skill_result.status, AgentSkillStatus::Succeeded);
    assert_eq!(skill_result.output, "reviewed diff");
    let local_skills = report
        .runtime
        .agent_skill_provider_by_id("provider.skill.local")
        .expect("local skill provider is registered");
    assert_eq!(
        local_skills
            .describe_skill("skill.local-refactor")
            .expect("local skill exists")
            .model_hint
            .as_deref(),
        Some("local-model")
    );
    let local_skill_result = local_skills
        .invoke_skill(
            AgentSkillRequest::new("skill-request.2", "skill.local-refactor")
                .with_argument("scope", "diff"),
        )
        .expect("local skill invokes");
    assert_eq!(local_skill_result.output, "refactored locally");

    let capability_manifest = report.runtime.capability_manifest();
    assert!(capability_manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "mcp"
            && provider.provider_id == "provider.mcp.github"));
    assert_eq!(
        capability_manifest
            .providers
            .iter()
            .find(|provider| provider.provider_id == "provider.mcp.github")
            .expect("mcp provider manifest exists")
            .capabilities,
        ["mcp.tools", "mcp.resources", "mcp.prompts"]
    );
    assert!(capability_manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "skill"
            && provider.provider_id == "provider.skill.claude"));
    assert_eq!(
        capability_manifest
            .providers
            .iter()
            .find(|provider| provider.provider_id == "provider.skill.claude")
            .expect("skill provider manifest exists")
            .capabilities,
        ["skill.discover", "skill.invoke"]
    );
    assert!(capability_manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "mcp"
            && provider.provider_id == "provider.mcp.gitlab"));
    assert_eq!(
        capability_manifest
            .providers
            .iter()
            .find(|provider| provider.provider_id == "provider.mcp.gitlab")
            .expect("selected mcp provider manifest exists")
            .capabilities,
        ["mcp.tools"]
    );
    assert!(capability_manifest
        .providers
        .iter()
        .any(|provider| provider.provider_family == "skill"
            && provider.provider_id == "provider.skill.local"));
    assert_eq!(
        capability_manifest
            .providers
            .iter()
            .find(|provider| provider.provider_id == "provider.skill.local")
            .expect("selected skill provider manifest exists")
            .capabilities,
        ["skill.discover"]
    );
    assert!(capability_manifest
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "mcp.tools"));
    assert!(capability_manifest
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == "skill.invoke"));
}

struct StaticModelProvider {
    provider_id: &'static str,
    response: &'static str,
}

impl StaticModelProvider {
    fn new(provider_id: &'static str, response: &'static str) -> Self {
        Self {
            provider_id,
            response,
        }
    }
}

impl ModelProvider for StaticModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id,
            "model",
            self.provider_id,
            "1.0.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id,
            self.response,
        ))
    }
}

struct FakeMcpProvider {
    provider_id: &'static str,
    server_id: &'static str,
    tool_id: &'static str,
}

impl FakeMcpProvider {
    fn new(provider_id: &'static str, server_id: &'static str, tool_id: &'static str) -> Self {
        Self {
            provider_id,
            server_id,
            tool_id,
        }
    }
}

impl McpProvider for FakeMcpProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        let capabilities = if self.provider_id == "provider.mcp.gitlab" {
            vec!["mcp.tools".to_string()]
        } else {
            vec![
                "mcp.tools".to_string(),
                "mcp.resources".to_string(),
                "mcp.prompts".to_string(),
            ]
        };

        ProviderManifest::new(
            self.provider_id,
            "mcp",
            self.provider_id,
            "1.0.0",
            capabilities,
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_servers(&self) -> Vec<McpServerDescriptor> {
        vec![
            McpServerDescriptor::new(self.server_id, self.provider_id, "stdio")
                .with_capability("tools")
                .with_capability("resources")
                .with_capability("prompts"),
        ]
    }

    fn list_tools(&self, server_id: &str) -> KernelResult<Vec<ToolDescriptor>> {
        assert_eq!(server_id, self.server_id);
        Ok(vec![ToolDescriptor::new(
            self.tool_id,
            self.provider_id,
            "MCP Search",
            SideEffectLevel::ReadOnly,
        )])
    }

    fn invoke_tool(&self, server_id: &str, call: ToolCall) -> KernelResult<ToolResult> {
        assert_eq!(server_id, self.server_id);
        Ok(ToolResult::succeeded(call.tool_call_id, "mcp tool output"))
    }

    fn list_resources(&self, server_id: &str) -> KernelResult<Vec<McpResourceDescriptor>> {
        assert_eq!(server_id, self.server_id);
        Ok(vec![McpResourceDescriptor::new(
            "repo://sdkwork/README.md",
            "README.md",
            "text/markdown",
        )])
    }

    fn read_resource(&self, server_id: &str, uri: &str) -> KernelResult<McpResourceContent> {
        assert_eq!(server_id, self.server_id);
        assert_eq!(uri, "repo://sdkwork/README.md");
        Ok(McpResourceContent::new(uri, "text/markdown", "# SDKWork"))
    }

    fn list_prompts(&self, server_id: &str) -> KernelResult<Vec<McpPromptDescriptor>> {
        assert_eq!(server_id, self.server_id);
        Ok(vec![McpPromptDescriptor::new(
            "prompt.code-review",
            "Code Review",
        )
        .with_description("Review a code diff")])
    }

    fn get_prompt(
        &self,
        server_id: &str,
        prompt_id: &str,
        arguments: Vec<(String, String)>,
    ) -> KernelResult<McpPromptMessage> {
        assert_eq!(server_id, self.server_id);
        assert_eq!(prompt_id, "prompt.code-review");
        assert_eq!(arguments, [("scope".to_string(), "diff".to_string())]);
        Ok(McpPromptMessage::new(
            prompt_id,
            vec!["review diff".to_string()],
        ))
    }
}

struct FakeAgentSkillProvider {
    provider_id: &'static str,
    skill_id: &'static str,
    model_hint: &'static str,
    output: &'static str,
}

impl FakeAgentSkillProvider {
    fn new(
        provider_id: &'static str,
        skill_id: &'static str,
        model_hint: &'static str,
        output: &'static str,
    ) -> Self {
        Self {
            provider_id,
            skill_id,
            model_hint,
            output,
        }
    }
}

impl AgentSkillProvider for FakeAgentSkillProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        let capabilities = if self.provider_id == "provider.skill.local" {
            vec!["skill.discover".to_string()]
        } else {
            vec!["skill.discover".to_string(), "skill.invoke".to_string()]
        };

        ProviderManifest::new(
            self.provider_id,
            "skill",
            self.provider_id,
            "1.0.0",
            capabilities,
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_skills(&self) -> Vec<AgentSkillDescriptor> {
        vec![AgentSkillDescriptor::new(
            self.skill_id,
            self.provider_id,
            "Agent Skill",
            "Review code changes and return risks.",
            AgentSkillInvocationMode::ModelInvocable,
        )
        .with_model_hint(self.model_hint)
        .with_allowed_tool("Read")
        .with_allowed_tool("Grep")]
    }

    fn invoke_skill(&self, request: AgentSkillRequest) -> KernelResult<AgentSkillResult> {
        assert_eq!(request.skill_id, self.skill_id);
        assert_eq!(request.argument_value("scope"), Some("diff"));
        Ok(AgentSkillResult::succeeded(
            request.skill_request_id,
            request.skill_id,
            self.output,
        ))
    }
}
