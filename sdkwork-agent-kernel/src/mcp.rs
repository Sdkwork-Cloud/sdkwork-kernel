use crate::{
    ContextFrame, KernelError, KernelResult, ProviderHealth, ProviderManifest,
    RedactionClassification, ToolCall, ToolDescriptor, ToolResult, TrustLevel,
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
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
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
            trust_level: TrustLevel::RetrievedExternal,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
        self
    }

    pub fn with_redaction_classification(
        mut self,
        redaction_classification: RedactionClassification,
    ) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn to_context_frame(&self, session_id: impl Into<String>) -> ContextFrame {
        let mut frame = ContextFrame::new(
            format!("context.mcp.resource.{}", self.uri),
            session_id,
            "mcp.resource",
            self.content.clone(),
            self.trust_level,
            self.redaction_classification,
        )
        .with_content_type(self.mime_type.clone())
        .with_provenance(self.uri.clone());

        for (key, value) in &self.metadata {
            frame = frame.with_metadata(key.clone(), value.clone());
        }

        frame
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
    pub trust_level: TrustLevel,
    pub redaction_classification: RedactionClassification,
    pub metadata: Vec<(String, String)>,
}

impl McpPromptMessage {
    pub fn new(prompt_id: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            prompt_id: prompt_id.into(),
            messages,
            trust_level: TrustLevel::TrustedHost,
            redaction_classification: RedactionClassification::Internal,
            metadata: Vec::new(),
        }
    }

    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self {
        self.trust_level = trust_level;
        self
    }

    pub fn with_redaction_classification(
        mut self,
        redaction_classification: RedactionClassification,
    ) -> Self {
        self.redaction_classification = redaction_classification;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn to_context_frames(&self, session_id: impl Into<String>) -> Vec<ContextFrame> {
        let session_id = session_id.into();
        let provenance = format!("mcp.prompt:{}", self.prompt_id);

        self.messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                let mut frame = ContextFrame::new(
                    format!("context.mcp.prompt.{}.{}", self.prompt_id, index),
                    session_id.clone(),
                    "mcp.prompt",
                    message.clone(),
                    self.trust_level,
                    self.redaction_classification,
                )
                .with_content_type("text/plain")
                .with_provenance(provenance.clone());

                for (key, value) in &self.metadata {
                    frame = frame.with_metadata(key.clone(), value.clone());
                }

                frame
            })
            .collect()
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
