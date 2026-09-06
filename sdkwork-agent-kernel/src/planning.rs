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

    /// Abandon a plan (mark as cancelled and clean up resources)
    fn abandon_plan(&self, _plan_id: &str) -> KernelResult<()> {
        Err(KernelError::validation(
            "abandon_plan not implemented by this provider",
        ))
    }

    /// Get a specific plan by ID
    fn get_plan(&self, _plan_id: &str) -> KernelResult<Plan> {
        Err(KernelError::validation(
            "get_plan not implemented by this provider",
        ))
    }

    /// List all plans for a given task
    fn list_plans(&self, _task_id: &str) -> KernelResult<Vec<Plan>> {
        Err(KernelError::validation(
            "list_plans not implemented by this provider",
        ))
    }

    /// Visualize a plan as a structured representation (e.g., DAG, tree, flowchart)
    fn visualize_plan(&self, _plan_id: &str) -> KernelResult<PlanVisualization> {
        Err(KernelError::validation(
            "visualize_plan not implemented by this provider",
        ))
    }

    fn health(&self) -> ProviderHealth;
}

/// Plan visualization structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanVisualization {
    pub plan_id: String,
    pub format: VisualizationFormat,
    pub nodes: Vec<VisualizationNode>,
    pub edges: Vec<VisualizationEdge>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Visualization format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationFormat {
    /// Directed Acyclic Graph (DAG)
    Dag,
    /// Tree structure
    Tree,
    /// Flowchart
    Flowchart,
    /// Timeline/Gantt chart
    Timeline,
    /// JSON/structured data
    Json,
}

/// Visualization node representing an action
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualizationNode {
    pub node_id: String,
    pub action_id: String,
    pub label: String,
    pub status: ActionStatus,
    pub kind: ActionKind,
    pub position: Option<(u32, u32)>,
}

/// Visualization edge representing dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualizationEdge {
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: Option<String>,
    pub edge_type: EdgeType,
}

/// Edge type for visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// Sequential dependency
    Sequential,
    /// Parallel branch
    Parallel,
    /// Conditional branch
    Conditional,
    /// Error handling path
    ErrorHandling,
}

impl PlanVisualization {
    /// Create a new visualization for a plan
    pub fn new(plan_id: impl Into<String>, format: VisualizationFormat) -> Self {
        Self {
            plan_id: plan_id.into(),
            format,
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add a node to the visualization
    pub fn add_node(mut self, node: VisualizationNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Add an edge to the visualization
    pub fn add_edge(mut self, edge: VisualizationEdge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Generate a default DAG visualization from a plan
    pub fn from_plan(plan: &Plan) -> Self {
        let mut viz = Self::new(&plan.plan_id, VisualizationFormat::Dag);

        // Add nodes for each action
        for (idx, action) in plan.actions.iter().enumerate() {
            viz = viz.add_node(VisualizationNode {
                node_id: format!("node_{}", idx),
                action_id: action.action_id.clone(),
                label: action.description.clone(),
                status: action.status,
                kind: action.kind,
                position: Some((idx as u32, 0)),
            });
        }

        // Add edges for dependencies
        for (idx, action) in plan.actions.iter().enumerate() {
            for dep_id in &action.depends_on {
                // Find the dependency action index
                if let Some(dep_idx) = plan.actions.iter().position(|a| &a.action_id == dep_id) {
                    viz = viz.add_edge(VisualizationEdge {
                        from_node_id: format!("node_{}", dep_idx),
                        to_node_id: format!("node_{}", idx),
                        label: None,
                        edge_type: EdgeType::Sequential,
                    });
                }
            }
        }

        viz = viz
            .with_metadata("plan_revision", plan.revision.to_string())
            .with_metadata("task_id", plan.task_id.clone())
            .with_metadata("run_id", plan.run_id.clone());

        viz
    }
}
