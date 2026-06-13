use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest, SideEffectLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    ModelCall,
    ToolCall,
    MemoryRead,
    MemoryWrite,
    HostOperation,
    ProtocolSend,
    Handoff,
    WaitForUser,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Created,
    Ready,
    AwaitingApproval,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub action_id: String,
    pub kind: ActionKind,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub side_effect_level: SideEffectLevel,
    pub policy_categories: Vec<String>,
    pub depends_on: Vec<String>,
    pub status: ActionStatus,
    pub permission_request_id: Option<String>,
}

impl Action {
    pub fn new(
        action_id: impl Into<String>,
        kind: ActionKind,
        description: impl Into<String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            kind,
            description: description.into(),
            required_capabilities: Vec::new(),
            side_effect_level: SideEffectLevel::ReadOnly,
            policy_categories: Vec::new(),
            depends_on: Vec::new(),
            status: ActionStatus::Created,
            permission_request_id: None,
        }
    }

    pub fn with_required_capabilities(mut self, required_capabilities: Vec<String>) -> Self {
        self.required_capabilities = required_capabilities;
        self
    }

    pub fn with_side_effect_level(mut self, side_effect_level: SideEffectLevel) -> Self {
        self.side_effect_level = side_effect_level;
        self
    }

    pub fn with_policy_categories(mut self, policy_categories: Vec<String>) -> Self {
        self.policy_categories = policy_categories;
        self
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.side_effect_level != SideEffectLevel::ReadOnly && self.policy_categories.is_empty()
        {
            return Err(KernelError::validation(
                "side-effectful action requires at least one policy category",
            ));
        }

        Ok(())
    }

    pub fn mark_waiting_for_approval(
        mut self,
        permission_request_id: impl Into<String>,
    ) -> KernelResult<Self> {
        self.validate()?;
        self.status = ActionStatus::AwaitingApproval;
        self.permission_request_id = Some(permission_request_id.into());
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub plan_id: String,
    pub task_id: String,
    pub run_id: String,
    pub summary: String,
    pub actions: Vec<Action>,
    pub revision: u32,
    pub previous_plan_id: Option<String>,
}

impl Plan {
    pub fn new(
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            run_id: run_id.into(),
            summary: summary.into(),
            actions: Vec::new(),
            revision: 1,
            previous_plan_id: None,
        }
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.actions.is_empty() {
            return Err(KernelError::validation("plan requires at least one action"));
        }

        for action in &self.actions {
            action.validate()?;
        }

        Ok(())
    }

    pub fn revise_as(&self, plan_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            task_id: self.task_id.clone(),
            run_id: self.run_id.clone(),
            summary: summary.into(),
            actions: Vec::new(),
            revision: self.revision + 1,
            previous_plan_id: Some(self.plan_id.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub observation_id: String,
    pub action_id: String,
    pub step_id: String,
    pub summary: String,
    pub provenance: Option<String>,
}

impl Observation {
    pub fn new(
        observation_id: impl Into<String>,
        action_id: impl Into<String>,
        step_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            observation_id: observation_id.into(),
            action_id: action_id.into(),
            step_id: step_id.into(),
            summary: summary.into(),
            provenance: None,
        }
    }

    pub fn with_provenance(mut self, provenance: impl Into<String>) -> Self {
        self.provenance = Some(provenance.into());
        self
    }
}

pub trait PlanningProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.planning.unspecified",
            "planning",
            "planning-provider",
            "0.0.0",
            vec!["planning.create".to_string()],
        )
    }

    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> KernelResult<Plan>;

    fn validate_plan(&self, plan: &Plan) -> KernelResult<()> {
        plan.validate()
    }

    fn revise_plan(&self, plan: &Plan, new_summary: &str) -> KernelResult<Plan> {
        let new_plan_id = format!("{}.r{}", plan.plan_id, plan.revision + 1);
        Ok(plan.revise_as(new_plan_id, new_summary))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
