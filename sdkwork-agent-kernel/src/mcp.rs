use crate::{
    KernelError, KernelResult, ProviderHealth, ProviderManifest, ToolCall, ToolDescriptor,
    ToolResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerDescriptor {
    pub server_id: String,
    pub provider_id: String,
    pub transport: String,
    pub capabilities: Vec<String>,
}

impl McpServerDescriptor {
    pub fn new(
        server_id: impl Into<String>,
        provider_id: impl Into<String>,
        transport: impl Into<String>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            provider_id: provider_id.into(),
            transport: transport.into(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResourceDescriptor {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
    pub description: Option<String>,
}

impl McpResourceDescriptor {
    pub fn new(
        uri: impl Into<String>,
        name: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            mime_type: mime_type.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub content: String,
}

impl McpResourceContent {
    pub fn new(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            mime_type: mime_type.into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPromptDescriptor {
    pub prompt_id: String,
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<String>,
}

impl McpPromptDescriptor {
    pub fn new(prompt_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            prompt_id: prompt_id.into(),
            name: name.into(),
            description: None,
            arguments: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_argument(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpPromptMessage {
    pub prompt_id: String,
    pub messages: Vec<String>,
}

impl McpPromptMessage {
    pub fn new(prompt_id: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            prompt_id: prompt_id.into(),
            messages,
        }
    }
}

pub trait McpProvider {
    fn provider_manifest(&self) -> ProviderManifest;

    fn health(&self) -> ProviderHealth;

    fn list_servers(&self) -> Vec<McpServerDescriptor>;

    fn list_tools(&self, _server_id: &str) -> KernelResult<Vec<ToolDescriptor>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.tools".to_string(),
        })
    }

    fn invoke_tool(&self, _server_id: &str, _call: ToolCall) -> KernelResult<ToolResult> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.tools".to_string(),
        })
    }

    fn list_resources(&self, _server_id: &str) -> KernelResult<Vec<McpResourceDescriptor>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.resources".to_string(),
        })
    }

    fn read_resource(&self, _server_id: &str, uri: &str) -> KernelResult<McpResourceContent> {
        Err(KernelError::CapabilityMissing {
            capability_id: uri.to_string(),
        })
    }

    fn list_prompts(&self, _server_id: &str) -> KernelResult<Vec<McpPromptDescriptor>> {
        Err(KernelError::CapabilityMissing {
            capability_id: "mcp.prompts".to_string(),
        })
    }

    fn get_prompt(
        &self,
        _server_id: &str,
        prompt_id: &str,
        _arguments: Vec<(String, String)>,
    ) -> KernelResult<McpPromptMessage> {
        Err(KernelError::CapabilityMissing {
            capability_id: prompt_id.to_string(),
        })
    }
}
