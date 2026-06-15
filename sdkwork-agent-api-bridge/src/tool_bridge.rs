use sdkwork_agent_kernel::{
    KernelResult, SideEffectLevel, ToolCall, ToolDescriptor, ToolResult, ToolSchema,
};

/// Handles tool registration, discovery, and execution
pub struct ToolBridge {
    tools: Vec<ToolDescriptor>,
}

impl ToolBridge {
    pub fn new() -> Self {
        let tools = vec![
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
        ];

        Self { tools }
    }

    /// List all available tools
    pub fn list_tools(&self) -> KernelResult<Vec<ToolDescriptor>> {
        Ok(self.tools.clone())
    }

    /// Get a tool descriptor by name
    pub fn get_tool(&self, tool_name: &str) -> KernelResult<ToolDescriptor> {
        self.tools
            .iter()
            .find(|t| t.name.as_deref() == Some(tool_name))
            .cloned()
            .ok_or_else(|| {
                sdkwork_agent_kernel::KernelError::validation(format!(
                    "tool not found: {}",
                    tool_name
                ))
            })
    }

    /// Execute a tool call (mock implementation)
    pub fn execute(&self, call: &ToolCall) -> KernelResult<ToolResult> {
        // Mock implementation - in production this would execute the actual tool
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

    /// Check if a tool requires policy approval
    pub fn requires_policy(&self, tool_name: &str) -> bool {
        self.tools
            .iter()
            .find(|t| t.name.as_deref() == Some(tool_name))
            .map(|t| !t.policy_categories.is_empty())
            .unwrap_or(false)
    }
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
    fn execute_bash() {
        let bridge = ToolBridge::new();
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
