use sdkwork_agent_kernel::{
    Action, ActionKind, KernelResult, ModelDescriptor, ModelProvider, ModelRequest, ModelResponse,
    ModelResponseFormat, Plan, PlanningProvider, PolicyCategory, PolicyDecision, PolicyProvider,
    PolicyRequest, ProviderHealth, ProviderManifest, SideEffectLevel, ToolCall, ToolDescriptor,
    ToolProvider, ToolResult,
};

use crate::{backend::RigBackend, ids};

#[derive(Debug, Clone)]
pub struct RigModelProvider {
    backend: RigBackend,
}

impl RigModelProvider {
    pub fn fail_closed() -> Self {
        Self {
            backend: RigBackend::fail_closed(),
        }
    }
}

impl ModelProvider for RigModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::MODEL_PROVIDER_ID,
            "model",
            "rig-rust",
            "0.1.0",
            vec![
                "model.catalog".to_string(),
                "model.chat".to_string(),
                "model.streaming".to_string(),
                "model.tool_call".to_string(),
            ],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ids::DEFAULT_MODEL_ID,
            ids::MODEL_PROVIDER_ID,
            "Rig Default Chat",
            "rig",
        )
        .with_capability("model.chat")
        .with_capability("model.catalog")
        .with_response_format(ModelResponseFormat::Text)
        .with_input_mode("text")
        .with_output_mode("text")
        .with_policy_category(PolicyCategory::ModelInvoke.as_str())
        .with_metadata("sdkwork.backend.default_mode", "fail_closed")]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        self.backend.invoke_model(request)
    }
}

#[derive(Debug, Clone)]
pub struct RigToolProvider {
    backend: RigBackend,
}

impl RigToolProvider {
    pub fn fail_closed() -> Self {
        Self {
            backend: RigBackend::fail_closed(),
        }
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::TOOL_PROVIDER_ID,
            "tool",
            "rig-rust-tools",
            "0.1.0",
            vec!["tool.invoke".to_string()],
        )
    }
}

impl ToolProvider for RigToolProvider {
    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            ids::DEFAULT_TOOL_ID,
            ids::TOOL_PROVIDER_ID,
            "Rig Tool Bridge",
            SideEffectLevel::SideEffectful,
        )
        .with_policy_categories(vec![PolicyCategory::ToolInvoke.as_str().to_string()])
        .require_audit()]
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke_tool(&self, call: ToolCall) -> KernelResult<ToolResult> {
        Ok(self.backend.invoke_tool(call))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RigPlanningProvider;

impl RigPlanningProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            ids::PLANNING_PROVIDER_ID,
            "planning",
            "rig-rust-planning",
            "0.1.0",
            vec!["planning.create".to_string()],
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct RigPolicyProvider;

impl RigPolicyProvider {
    pub fn new() -> Self {
        Self
    }
}

impl PolicyProvider for RigPolicyProvider {
    fn evaluate(&self, request: PolicyRequest) -> KernelResult<PolicyDecision> {
        Ok(PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            ids::POLICY_PROVIDER_ID,
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

impl PlanningProvider for RigPlanningProvider {
    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> Plan {
        Plan::new("plan.rig.runtime", task_id, run_id, summary).add_action(
            Action::new(
                "action.model.invoke",
                ActionKind::ModelCall,
                "invoke Rig model provider",
            )
            .with_required_capabilities(vec!["model.chat".to_string()])
            .with_side_effect_level(SideEffectLevel::ExternalSend)
            .with_policy_categories(vec![PolicyCategory::ModelInvoke.as_str().to_string()]),
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
