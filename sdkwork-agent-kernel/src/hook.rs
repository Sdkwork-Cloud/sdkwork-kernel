//! Kernel hook SPI: lifecycle interception points for model invocation,
//! tool invocation, user prompts, and session lifecycle.
//!
//! The hook surface aligns with the agent ecosystem hooks (claude-code
//! `PreToolUse`/`PostToolUse`/`UserPromptSubmit`/`Stop`/`SessionStart`,
//! rig `PromptHook`) while staying kernel-neutral: hooks observe and can
//! terminate or skip work at phase boundaries. Hooks are synchronous and
//! must not call back into the runtime.

use crate::{KernelResult, ModelRequest, ModelResponse, ToolCall, ToolResult};
use std::sync::Arc;

/// Result of a lifecycle hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookAction {
    /// Continue with the phase.
    Continue,
    /// Stop processing and abort the phase with the given reason.
    Terminate { reason: String },
}

/// Result of a tool-invocation hook, adding a skip decision that feeds a
/// synthetic denied tool result back to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolHookAction {
    Continue,
    /// Do not execute the tool; return a denied result with the reason.
    Skip {
        reason: String,
    },
    /// Stop the whole run.
    Terminate {
        reason: String,
    },
}

/// Permission decision context delivered to hooks before policy
/// evaluation, mirroring the agent SDK permission hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequestContext {
    pub permission_request_id: String,
    pub tool_call_id: Option<String>,
    pub tool_id: Option<String>,
    /// Operation being authorized, e.g. `tool.invoke`.
    pub operation: String,
    pub reason: Option<String>,
}

impl PermissionRequestContext {
    pub fn new(permission_request_id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            permission_request_id: permission_request_id.into(),
            tool_call_id: None,
            tool_id: None,
            operation: operation.into(),
            reason: None,
        }
    }

    pub fn for_tool_call(
        permission_request_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        tool_id: impl Into<String>,
    ) -> Self {
        Self {
            permission_request_id: permission_request_id.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_id: Some(tool_id.into()),
            operation: "tool.invoke".to_string(),
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Hook override for a permission decision: approve or deny without the
/// normal policy flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionHookAction {
    /// Proceed with the normal policy evaluation.
    Continue,
    /// Approve without asking; the hook reason is recorded.
    Approve { reason: String },
    /// Deny without asking; the reason is delivered to the model.
    Deny { reason: String },
}

/// Kernel hook trait. Every method has a no-op default so hooks implement
/// only the interception points they care about.
pub trait KernelHook: Send + Sync {
    /// Before the model provider is invoked. Returning `Terminate` aborts
    /// the model round with a cancelled error.
    fn on_before_model_invoke(&self, _request: &ModelRequest) -> KernelResult<HookAction> {
        Ok(HookAction::Continue)
    }

    /// After the model provider returned. Returning `Terminate` stops
    /// remaining hooks; the response is still delivered.
    fn on_after_model_invoke(
        &self,
        _request: &ModelRequest,
        _response: &ModelResponse,
    ) -> KernelResult<HookAction> {
        Ok(HookAction::Continue)
    }

    /// Before a tool executes. `Skip` produces a denied tool result without
    /// invoking the provider; `Terminate` aborts the run.
    fn on_before_tool_invoke(&self, _tool_call: &ToolCall) -> KernelResult<ToolHookAction> {
        Ok(ToolHookAction::Continue)
    }

    /// After a tool executed.
    fn on_after_tool_invoke(
        &self,
        _tool_call: &ToolCall,
        _result: &ToolResult,
    ) -> KernelResult<HookAction> {
        Ok(HookAction::Continue)
    }

    /// Before a permission decision: approve or deny overrides the normal
    /// policy flow.
    fn on_permission_request(
        &self,
        _context: &PermissionRequestContext,
    ) -> KernelResult<PermissionHookAction> {
        Ok(PermissionHookAction::Continue)
    }

    /// Before a user prompt is processed.
    fn on_user_prompt(&self, _prompt: &str) -> KernelResult<HookAction> {
        Ok(HookAction::Continue)
    }

    fn on_session_start(&self, _session_id: &str) -> KernelResult<()> {
        Ok(())
    }

    fn on_session_end(&self, _session_id: &str, _reason: &str) -> KernelResult<()> {
        Ok(())
    }

    fn on_stop(&self, _reason: &str) -> KernelResult<()> {
        Ok(())
    }
}

/// Registry of kernel hooks attached to a runtime. Hooks run in registration
/// order; the first terminating decision wins and stops remaining hooks.
#[derive(Default)]
pub struct KernelHookRegistry {
    hooks: Vec<Arc<dyn KernelHook>>,
}

impl std::fmt::Debug for KernelHookRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KernelHookRegistry")
            .field("hook_count", &self.hooks.len())
            .finish()
    }
}

impl Clone for KernelHookRegistry {
    fn clone(&self) -> Self {
        Self {
            hooks: self.hooks.clone(),
        }
    }
}

impl PartialEq for KernelHookRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.hooks.len() == other.hooks.len()
    }
}

impl Eq for KernelHookRegistry {}

impl KernelHookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, hook: Arc<dyn KernelHook>) {
        self.hooks.push(hook);
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    pub fn run_before_model_invoke(&self, request: &ModelRequest) -> KernelResult<HookAction> {
        for hook in &self.hooks {
            match hook.on_before_model_invoke(request)? {
                HookAction::Continue => {}
                action => return Ok(action),
            }
        }
        Ok(HookAction::Continue)
    }

    pub fn run_after_model_invoke(
        &self,
        request: &ModelRequest,
        response: &ModelResponse,
    ) -> KernelResult<HookAction> {
        for hook in &self.hooks {
            match hook.on_after_model_invoke(request, response)? {
                HookAction::Continue => {}
                action => return Ok(action),
            }
        }
        Ok(HookAction::Continue)
    }

    pub fn run_before_tool_invoke(&self, tool_call: &ToolCall) -> KernelResult<ToolHookAction> {
        for hook in &self.hooks {
            match hook.on_before_tool_invoke(tool_call)? {
                ToolHookAction::Continue => {}
                action => return Ok(action),
            }
        }
        Ok(ToolHookAction::Continue)
    }

    pub fn run_after_tool_invoke(
        &self,
        tool_call: &ToolCall,
        result: &ToolResult,
    ) -> KernelResult<HookAction> {
        for hook in &self.hooks {
            match hook.on_after_tool_invoke(tool_call, result)? {
                HookAction::Continue => {}
                action => return Ok(action),
            }
        }
        Ok(HookAction::Continue)
    }

    pub fn run_permission_request(
        &self,
        context: &PermissionRequestContext,
    ) -> KernelResult<PermissionHookAction> {
        for hook in &self.hooks {
            match hook.on_permission_request(context)? {
                PermissionHookAction::Continue => {}
                action => return Ok(action),
            }
        }
        Ok(PermissionHookAction::Continue)
    }

    pub fn run_user_prompt(&self, prompt: &str) -> KernelResult<HookAction> {
        for hook in &self.hooks {
            match hook.on_user_prompt(prompt)? {
                HookAction::Continue => {}
                action => return Ok(action),
            }
        }
        Ok(HookAction::Continue)
    }

    pub fn run_session_start(&self, session_id: &str) -> KernelResult<()> {
        for hook in &self.hooks {
            hook.on_session_start(session_id)?;
        }
        Ok(())
    }

    pub fn run_session_end(&self, session_id: &str, reason: &str) -> KernelResult<()> {
        for hook in &self.hooks {
            hook.on_session_end(session_id, reason)?;
        }
        Ok(())
    }

    pub fn run_stop(&self, reason: &str) -> KernelResult<()> {
        for hook in &self.hooks {
            hook.on_stop(reason)?;
        }
        Ok(())
    }
}
