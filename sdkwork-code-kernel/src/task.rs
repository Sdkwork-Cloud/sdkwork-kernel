use crate::Workspace;
use sdkwork_agent_kernel::{KernelError, KernelResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeTask {
    pub task_id: String,
    pub workspace: Workspace,
    pub intent: CodeTaskIntent,
    pub state: CodeTaskState,
    pub plan: Option<CodePlan>,
    pub checkpoints: Vec<CodeCheckpoint>,
    pub review_status: CodeReviewStatus,
    pub trace_refs: Vec<CodeTraceRef>,
}

impl CodeTask {
    pub fn new(task_id: impl Into<String>, workspace: Workspace, intent: CodeTaskIntent) -> Self {
        Self {
            task_id: task_id.into(),
            workspace,
            intent,
            state: CodeTaskState::Created,
            plan: None,
            checkpoints: Vec::new(),
            review_status: CodeReviewStatus::NotRequired,
            trace_refs: Vec::new(),
        }
    }

    pub fn with_plan(mut self, plan: CodePlan) -> Self {
        self.plan = Some(plan);
        self
    }

    pub fn add_checkpoint(mut self, checkpoint: CodeCheckpoint) -> Self {
        self.checkpoints.push(checkpoint);
        self
    }

    pub fn with_review_status(mut self, review_status: CodeReviewStatus) -> Self {
        self.review_status = review_status;
        self
    }

    pub fn add_trace_ref(mut self, trace_ref: CodeTraceRef) -> Self {
        self.trace_refs.push(trace_ref);
        self
    }

    pub fn transition(mut self, next_state: CodeTaskState) -> KernelResult<Self> {
        if !is_valid_code_task_transition(self.state, next_state) {
            return Err(KernelError::validation("invalid code task transition"));
        }

        self.state = next_state;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeTaskState {
    Created,
    Planning,
    AwaitingPermission,
    Running,
    Reviewing,
    Completed,
    Failed,
    Cancelled,
}

impl CodeTaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Planning => "planning",
            Self::AwaitingPermission => "awaiting_permission",
            Self::Running => "running",
            Self::Reviewing => "reviewing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeTaskIntent {
    pub prompt: String,
    pub context_paths: Vec<String>,
    pub constraints: Vec<String>,
}

impl CodeTaskIntent {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            context_paths: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn with_context_path(mut self, context_path: impl Into<String>) -> Self {
        self.context_paths.push(context_path.into());
        self
    }

    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePlan {
    pub plan_id: String,
    pub revision: u32,
    pub steps: Vec<CodePlanStep>,
}

impl CodePlan {
    pub fn new(plan_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            revision: 1,
            steps: Vec::new(),
        }
    }

    pub fn add_step(mut self, step: CodePlanStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn revise(mut self) -> Self {
        self.revision += 1;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePlanStep {
    pub step_id: String,
    pub capability_id: String,
    pub summary: Option<String>,
    pub policy_required: bool,
}

impl CodePlanStep {
    pub fn new(step_id: impl Into<String>, capability_id: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            capability_id: capability_id.into(),
            summary: None,
            policy_required: false,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn requires_policy(mut self) -> Self {
        self.policy_required = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeReviewStatus {
    NotRequired,
    Required,
    InReview,
    Approved,
    ChangesRequested,
}

impl CodeReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::Required => "required",
            Self::InReview => "in_review",
            Self::Approved => "approved",
            Self::ChangesRequested => "changes_requested",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCheckpoint {
    pub checkpoint_id: String,
    pub label: String,
    pub artifact_ids: Vec<String>,
    pub vcs_revision: Option<String>,
}

impl CodeCheckpoint {
    pub fn new(checkpoint_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            checkpoint_id: checkpoint_id.into(),
            label: label.into(),
            artifact_ids: Vec::new(),
            vcs_revision: None,
        }
    }

    pub fn with_artifact_id(mut self, artifact_id: impl Into<String>) -> Self {
        self.artifact_ids.push(artifact_id.into());
        self
    }

    pub fn with_vcs_revision(mut self, vcs_revision: impl Into<String>) -> Self {
        self.vcs_revision = Some(vcs_revision.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeTraceRef {
    pub trace_id: String,
    pub run_id: String,
    pub span_id: Option<String>,
}

impl CodeTraceRef {
    pub fn new(trace_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            run_id: run_id.into(),
            span_id: None,
        }
    }

    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }
}

fn is_valid_code_task_transition(current: CodeTaskState, next: CodeTaskState) -> bool {
    use CodeTaskState::{
        AwaitingPermission, Cancelled, Completed, Created, Failed, Planning, Reviewing, Running,
    };

    matches!(
        (current, next),
        (Created, Planning | Running | Cancelled)
            | (Planning, AwaitingPermission | Running | Failed | Cancelled)
            | (AwaitingPermission, Running | Failed | Cancelled)
            | (
                Running,
                AwaitingPermission | Reviewing | Completed | Failed | Cancelled
            )
            | (Reviewing, Running | Completed | Failed | Cancelled)
    )
}
