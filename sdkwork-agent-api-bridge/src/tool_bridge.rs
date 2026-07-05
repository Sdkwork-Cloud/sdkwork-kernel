use std::sync::Arc;

use sdkwork_agent_kernel::{
    AgentRuntime, KernelResult, SideEffectLevel, ToolCall, ToolCallStatus, ToolDescriptor,
    ToolResult, ToolSchema,
};

/// Handles tool registration, discovery, and execution
pub struct ToolBridge {
    tools: Vec<ToolDescriptor>,
    agent_runtime: Option<Arc<AgentRuntime>>,
    allow_mock_fallback: bool,
}

impl ToolBridge {
    pub fn new() -> Self {
        Self {
            tools: builtin_tools(),
            agent_runtime: None,
            allow_mock_fallback: false,
        }
    }

    pub fn with_agent_runtime(agent_runtime: Arc<AgentRuntime>, allow_mock_fallback: bool) -> Self {
        Self {
            tools: builtin_tools(),
            agent_runtime: Some(agent_runtime),
            allow_mock_fallback,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_mock_fallback_enabled() -> Self {
        Self {
            tools: builtin_tools(),
            agent_runtime: None,
            allow_mock_fallback: true,
        }
    }

    /// List all available tools
    pub fn list_tools(&self) -> KernelResult<Vec<ToolDescriptor>> {
        let mut tools = self.tools.clone();
        if let Some(runtime) = &self.agent_runtime {
            if let Ok(provider) = runtime.tool_provider() {
                for descriptor in provider.list_tools() {
                    let duplicate = tools.iter().any(|existing| {
                        existing.tool_id == descriptor.tool_id || existing.name == descriptor.name
                    });
                    if !duplicate {
                        tools.push(descriptor);
                    }
                }
            }
        }
        Ok(tools)
    }

    /// Get a tool descriptor by name
    pub fn get_tool(&self, tool_name: &str) -> KernelResult<ToolDescriptor> {
        if let Ok(tools) = self.list_tools() {
            if let Some(tool) = tools
                .into_iter()
                .find(|tool| tool.name.as_deref() == Some(tool_name))
            {
                return Ok(tool);
            }
        }

        Err(sdkwork_agent_kernel::KernelError::validation(format!(
            "tool not found: {tool_name}"
        )))
    }

    /// Execute a tool call through typed providers with optional mock fallback.
    pub fn execute(&self, call: &ToolCall) -> KernelResult<ToolResult> {
        if let Some(runtime) = &self.agent_runtime {
            let provider = runtime.tool_provider()?;
            let result = provider.invoke_tool(call.clone())?;
            if result.normalized_status == ToolCallStatus::Succeeded || !self.allow_mock_fallback {
                return Ok(result);
            }
        } else if !self.allow_mock_fallback {
            return Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: "provider.tool".to_string(),
            });
        }

        if !self.allow_mock_fallback {
            return Err(sdkwork_agent_kernel::KernelError::ProviderUnavailable {
                provider_id: "provider.tool".to_string(),
            });
        }

        self.execute_mock(call)
    }

    /// Check if a tool requires policy approval
    pub fn requires_policy(&self, tool_name: &str) -> bool {
        self.get_tool(tool_name)
            .ok()
            .map(|tool| !tool.policy_categories.is_empty())
            .unwrap_or(false)
    }

    fn execute_mock(&self, call: &ToolCall) -> KernelResult<ToolResult> {
        let output = match call.tool_id.as_str() {
            "bash" => format!("$ {}\nCommand executed successfully.", call.arguments),
            "read_file" => "File contents would appear here.".to_string(),
            "write_file" => "File written successfully.".to_string(),
            "list_dir" => "file1.txt\nfile2.rs\ndir1/".to_string(),
            "search" => "Found 3 matches in 2 files.".to_string(),
            _ => format!("Tool '{}' executed.", call.tool_id),
        };

        Ok(ToolResult::succeeded(&call.tool_call_id, &output).with_duration_ms(100))
    }
}

fn builtin_tools() -> Vec<ToolDescriptor> {
    vec![
        ToolDescriptor::new(
            "tool.bash",
            "provider.tool.builtin",
            "bash",
            SideEffectLevel::SideEffectful,
        )
        .with_name("bash")
        .with_description("Execute shell commands")
        .with_input_schema(ToolSchema::json_schema("bash_input"))
        .with_policy_categories(vec!["host.process.execute".to_string()])
        .with_timeout_ms(30000),
        ToolDescriptor::new(
            "tool.read_file",
            "provider.tool.builtin",
            "read_file",
            SideEffectLevel::ReadOnly,
        )
        .with_name("read_file")
        .with_description("Read file contents")
        .with_input_schema(ToolSchema::json_schema("read_file_input")),
        ToolDescriptor::new(
            "tool.write_file",
            "provider.tool.builtin",
            "write_file",
            SideEffectLevel::SideEffectful,
        )
        .with_name("write_file")
        .with_description("Write file contents")
        .with_input_schema(ToolSchema::json_schema("write_file_input"))
        .with_policy_categories(vec!["host.filesystem.write".to_string()]),
        ToolDescriptor::new(
            "tool.list_dir",
            "provider.tool.builtin",
            "list_dir",
            SideEffectLevel::ReadOnly,
        )
        .with_name("list_dir")
        .with_description("List directory contents")
        .with_input_schema(ToolSchema::json_schema("list_dir_input")),
        ToolDescriptor::new(
            "tool.search",
            "provider.tool.builtin",
            "search",
            SideEffectLevel::ReadOnly,
        )
        .with_name("search")
        .with_description("Search files by pattern")
        .with_input_schema(ToolSchema::json_schema("search_input")),
    ]
}

impl Default for ToolBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tools_returns_all() {
        let bridge = ToolBridge::new();
        let tools = bridge.list_tools().expect("listed");
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn get_tool_by_name() {
        let bridge = ToolBridge::new();
        let tool = bridge.get_tool("bash").expect("found");
        assert_eq!(tool.name, Some("bash".to_string()));
    }

    #[test]
    fn execute_bash_requires_mock_fallback_when_runtime_missing() {
        let bridge = ToolBridge::new();
        let call = ToolCall::new("call.1", "bash", "echo hello");
        let error = bridge.execute(&call).expect_err("mock disabled without runtime");
        assert!(matches!(
            error,
            sdkwork_agent_kernel::KernelError::ProviderUnavailable { .. }
        ));
    }

    #[test]
    fn execute_bash_with_mock_fallback() {
        let bridge = ToolBridge::with_mock_fallback_enabled();
        let call = ToolCall::new("call.1", "bash", "echo hello");
        let result = bridge.execute(&call).expect("executed");
        assert_eq!(result.status, "succeeded");
    }

    #[test]
    fn requires_policy_for_dangerous_tools() {
        let bridge = ToolBridge::new();
        assert!(bridge.requires_policy("bash"));
        assert!(bridge.requires_policy("write_file"));
        assert!(!bridge.requires_policy("read_file"));
    }
}
