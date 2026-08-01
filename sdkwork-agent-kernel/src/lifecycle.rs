use crate::{parse_agent_input_contract_json, AgentInputContract};
use crate::{
    EventRecorder, KernelError, KernelEvent, KernelEventSeverity, KernelResult,
    SessionActivitySnapshot,
};

// ============================================================================
// Session State
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Active,
    Paused,
    Waiting,
    Working,
    Closed,
    Failed,
    Archived,
}

impl SessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Waiting => "waiting",
            Self::Working => "working",
            Self::Closed => "closed",
            Self::Failed => "failed",
            Self::Archived => "archived",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Working | Self::Waiting)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Failed | Self::Archived)
    }
}

// ============================================================================
// Session Kind - unified across all agents
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// Main conversation session
    Main,
    /// Subagent/child session
    Subagent,
    /// Background/scheduled session
    Background,
    /// Direct message session
    Direct,
    /// Group session
    Group,
    /// Task-specific session
    Task,
    /// Ephemeral/title generation session
    Ephemeral,
}

impl SessionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Subagent => "subagent",
            Self::Background => "background",
            Self::Direct => "direct",
            Self::Group => "group",
            Self::Task => "task",
            Self::Ephemeral => "ephemeral",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "main" => Some(Self::Main),
            "subagent" => Some(Self::Subagent),
            "background" => Some(Self::Background),
            "direct" => Some(Self::Direct),
            "group" => Some(Self::Group),
            "task" => Some(Self::Task),
            "ephemeral" => Some(Self::Ephemeral),
            _ => None,
        }
    }
}

// ============================================================================
// Session Continuation - how a run attaches to an existing session
// ============================================================================

/// How a new agent run attaches to an existing session, aligning the kernel
/// with the agent SDK resume primitives (`resume`, `continue`, `forkSession`,
/// `resumeSessionAt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionContinuationMode {
    /// Resume an explicit session by id.
    Resume { session_id: String },
    /// Continue the most recent session for the agent/user scope.
    ContinueLatest,
    /// Fork a new session from a point in an existing session.
    Fork {
        source_session_id: String,
        /// Truncate the forked history before this message when present.
        before_message_id: Option<String>,
    },
    /// Resume the session as of a given timestamp.
    ResumeAt { session_id: String, at: String },
}

impl SessionContinuationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Resume { .. } => "resume",
            Self::ContinueLatest => "continue_latest",
            Self::Fork { .. } => "fork",
            Self::ResumeAt { .. } => "resume_at",
        }
    }

    /// The session id this continuation targets, when explicit.
    pub fn target_session_id(&self) -> Option<&str> {
        match self {
            Self::Resume { session_id } => Some(session_id),
            Self::ContinueLatest => None,
            Self::Fork {
                source_session_id, ..
            } => Some(source_session_id),
            Self::ResumeAt { session_id, .. } => Some(session_id),
        }
    }
}

/// A session continuation request: the explicit attachment strategy a run
/// uses to resume, continue, or fork session history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContinuation {
    pub mode: SessionContinuationMode,
    pub reason: Option<String>,
}

impl SessionContinuation {
    pub fn resume(session_id: impl Into<String>) -> Self {
        Self {
            mode: SessionContinuationMode::Resume {
                session_id: session_id.into(),
            },
            reason: None,
        }
    }

    pub fn continue_latest() -> Self {
        Self {
            mode: SessionContinuationMode::ContinueLatest,
            reason: None,
        }
    }

    pub fn fork(source_session_id: impl Into<String>) -> Self {
        Self {
            mode: SessionContinuationMode::Fork {
                source_session_id: source_session_id.into(),
                before_message_id: None,
            },
            reason: None,
        }
    }

    pub fn fork_before(
        source_session_id: impl Into<String>,
        before_message_id: impl Into<String>,
    ) -> Self {
        Self {
            mode: SessionContinuationMode::Fork {
                source_session_id: source_session_id.into(),
                before_message_id: Some(before_message_id.into()),
            },
            reason: None,
        }
    }

    pub fn resume_at(session_id: impl Into<String>, at: impl Into<String>) -> Self {
        Self {
            mode: SessionContinuationMode::ResumeAt {
                session_id: session_id.into(),
                at: at.into(),
            },
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn mode(&self) -> &SessionContinuationMode {
        &self.mode
    }
}

// ============================================================================
// Session Source - where the session originated
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionSource {
    Cli,
    Api,
    Web,
    Telegram,
    Slack,
    Discord,
    Ide,
    Desktop,
    Mobile,
    Scheduled,
    Unknown,
}

impl SessionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Api => "api",
            Self::Web => "web",
            Self::Telegram => "telegram",
            Self::Slack => "slack",
            Self::Discord => "discord",
            Self::Ide => "ide",
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Scheduled => "scheduled",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cli" => Some(Self::Cli),
            "api" => Some(Self::Api),
            "web" => Some(Self::Web),
            "telegram" => Some(Self::Telegram),
            "slack" => Some(Self::Slack),
            "discord" => Some(Self::Discord),
            "ide" => Some(Self::Ide),
            "desktop" => Some(Self::Desktop),
            "mobile" => Some(Self::Mobile),
            "scheduled" => Some(Self::Scheduled),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

// ============================================================================
// Token Usage - unified across all agents
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
}

impl SessionTokenUsage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_input(&mut self, tokens: u64) {
        self.input_tokens = self.input_tokens.saturating_add(tokens);
        self.total_tokens = self.total_tokens.saturating_add(tokens);
    }

    pub fn record_output(&mut self, tokens: u64) {
        self.output_tokens = self.output_tokens.saturating_add(tokens);
        self.total_tokens = self.total_tokens.saturating_add(tokens);
    }

    pub fn record_cached(&mut self, tokens: u64) {
        self.cached_tokens = self.cached_tokens.saturating_add(tokens);
    }

    pub fn record_reasoning(&mut self, tokens: u64) {
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(tokens);
    }
}

// ============================================================================
// Session Change Summary - tracks file changes during session
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionChangeSummary {
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

impl SessionChangeSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_change(&mut self, additions: u32, deletions: u32) {
        self.additions = self.additions.saturating_add(additions);
        self.deletions = self.deletions.saturating_add(deletions);
        self.files_changed = self.files_changed.saturating_add(1);
    }
}

// ============================================================================
// AgentSession - unified session abstraction
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    // --- Core Identity ---
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub forked_from_id: Option<String>,
    pub slug: Option<String>,
    pub source: SessionSource,
    pub kind: SessionKind,
    pub agent_id: Option<String>,
    pub user_ref: Option<String>,
    pub tenant_id: Option<String>,

    // --- Display & Discovery ---
    pub title: Option<String>,
    pub preview: Option<String>,
    pub goal: Option<String>,
    pub summary: Option<String>,

    // --- Lifecycle State ---
    pub state: SessionState,
    pub activity: SessionActivitySnapshot,

    // --- Temporal ---
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub ended_at: Option<String>,
    pub archived_at: Option<String>,

    // --- Runtime Configuration ---
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub cwd: Option<String>,
    pub workspace_roots: Vec<String>,
    pub instructions: Option<String>,
    pub personality: Option<String>,
    pub reasoning_effort: Option<String>,
    pub approval_policy: Option<String>,
    pub permission_profile: Option<String>,

    // --- Usage & Metrics ---
    pub token_usage: SessionTokenUsage,
    pub message_count: u32,
    pub tool_call_count: u32,
    pub compression_count: u32,
    pub cost_cents: Option<u64>,
    pub change_summary: SessionChangeSummary,

    // --- Context & Compaction ---
    pub context_from: Option<String>,
    pub context_watermark: Option<String>,
    pub summary_message_id: Option<String>,

    // --- Multi-Agent ---
    pub child_session_ids: Vec<String>,
    pub agent_nickname: Option<String>,
    pub agent_role: Option<String>,

    // --- Configuration ---
    pub timeout_ms: Option<u64>,

    // --- Extensibility ---
    pub metadata: Vec<(String, String)>,
}

impl AgentSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let activity = SessionActivitySnapshot::unsupported(session_id.clone());
        Self {
            // Core Identity
            session_id,
            parent_session_id: None,
            forked_from_id: None,
            slug: None,
            source: SessionSource::Unknown,
            kind: SessionKind::Main,
            agent_id: None,
            user_ref: None,
            tenant_id: None,

            // Display & Discovery
            title: None,
            preview: None,
            goal: None,
            summary: None,

            // Lifecycle State
            state: SessionState::Created,
            activity,

            // Temporal
            created_at: None,
            updated_at: None,
            ended_at: None,
            archived_at: None,

            // Runtime Configuration
            model: None,
            model_provider: None,
            cwd: None,
            workspace_roots: Vec::new(),
            instructions: None,
            personality: None,
            reasoning_effort: None,
            approval_policy: None,
            permission_profile: None,

            // Usage & Metrics
            token_usage: SessionTokenUsage::new(),
            message_count: 0,
            tool_call_count: 0,
            compression_count: 0,
            cost_cents: None,
            change_summary: SessionChangeSummary::new(),

            // Context & Compaction
            context_from: None,
            context_watermark: None,
            summary_message_id: None,

            // Multi-Agent
            child_session_ids: Vec::new(),
            agent_nickname: None,
            agent_role: None,

            // Configuration
            timeout_ms: None,

            // Extensibility
            metadata: Vec::new(),
        }
    }

    // --- Builder Methods ---

    pub fn with_activity(mut self, activity: SessionActivitySnapshot) -> KernelResult<Self> {
        self.apply_activity(activity)?;
        Ok(self)
    }

    pub fn apply_activity(&mut self, activity: SessionActivitySnapshot) -> KernelResult<()> {
        if activity.provider_session_id != self.session_id {
            return Err(KernelError::validation(format!(
                "provider session activity belongs to {}, expected {}",
                activity.provider_session_id, self.session_id
            )));
        }
        self.state = activity.project_lifecycle_state(self.state);
        self.activity = activity;
        Ok(())
    }

    pub fn with_parent(mut self, parent_session_id: impl Into<String>) -> Self {
        self.parent_session_id = Some(parent_session_id.into());
        self.kind = SessionKind::Subagent;
        self
    }

    pub fn with_fork(mut self, forked_from_id: impl Into<String>) -> Self {
        self.forked_from_id = Some(forked_from_id.into());
        self
    }

    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    pub fn with_source(mut self, source: SessionSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_kind(mut self, kind: SessionKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_user_ref(mut self, user_ref: impl Into<String>) -> Self {
        self.user_ref = Some(user_ref.into());
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_model_provider(mut self, model_provider: impl Into<String>) -> Self {
        self.model_provider = Some(model_provider.into());
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_roots.push(root.into());
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn with_personality(mut self, personality: impl Into<String>) -> Self {
        self.personality = Some(personality.into());
        self
    }

    pub fn with_reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn with_approval_policy(mut self, policy: impl Into<String>) -> Self {
        self.approval_policy = Some(policy.into());
        self
    }

    pub fn with_permission_profile(mut self, profile: impl Into<String>) -> Self {
        self.permission_profile = Some(profile.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    // --- Accessor Methods ---

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Resolves the session input contract from `interactionContract` metadata or defaults.
    pub fn resolved_input_contract(&self) -> AgentInputContract {
        self.metadata_value("interactionContract")
            .and_then(|body| parse_agent_input_contract_json(body).ok())
            .unwrap_or_default()
    }

    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    pub fn is_subagent(&self) -> bool {
        self.kind == SessionKind::Subagent || self.parent_session_id.is_some()
    }

    pub fn has_parent(&self) -> bool {
        self.parent_session_id.is_some()
    }

    pub fn has_children(&self) -> bool {
        !self.child_session_ids.is_empty()
    }

    // --- Lifecycle Transitions ---

    pub fn activate(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        self.ensure_session_state(SessionState::Created, "activate")?;
        self.state = SessionState::Active;
        self.updated_at = self.created_at.clone();
        recorder.record(self.session_event("agent.session.activated"));
        Ok(self)
    }

    pub fn pause(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        if !self.state.is_active() {
            return Err(KernelError::validation(format!(
                "cannot pause session from state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Paused;
        recorder.record(self.session_event("agent.session.paused"));
        Ok(self)
    }

    pub fn resume(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        self.ensure_session_state(SessionState::Paused, "resume")?;
        self.state = SessionState::Active;
        recorder.record(self.session_event("agent.session.resumed"));
        Ok(self)
    }

    pub fn start_work(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        if !self.state.is_active() {
            return Err(KernelError::validation(format!(
                "cannot start work from state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Working;
        recorder.record(self.session_event("agent.session.work_started"));
        Ok(self)
    }

    pub fn finish_work(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        self.ensure_session_state(SessionState::Working, "finish_work")?;
        self.state = SessionState::Active;
        recorder.record(self.session_event("agent.session.work_finished"));
        Ok(self)
    }

    pub fn wait(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        if !self.state.is_active() {
            return Err(KernelError::validation(format!(
                "cannot wait from state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Waiting;
        recorder.record(self.session_event("agent.session.waiting"));
        Ok(self)
    }

    pub fn close(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        if self.state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot close session from state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Closed;
        self.ended_at = self.updated_at.clone();
        recorder.record(self.session_event("agent.session.closed"));
        Ok(self)
    }

    pub fn fail(
        mut self,
        recorder: &mut EventRecorder,
        reason: impl Into<String>,
    ) -> KernelResult<Self> {
        if self.state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot fail session from state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Failed;
        self.ended_at = self.updated_at.clone();
        let reason = reason.into();
        recorder.record(KernelEvent::new(
            format!("event.{}.agent.session.failed", self.session_id),
            "agent.session.failed",
            KernelEventSeverity::Error,
            format!("session_id={};reason={}", self.session_id, reason),
        ));
        Ok(self)
    }

    pub fn archive(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        if !self.state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot archive session from state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Archived;
        self.archived_at = self.updated_at.clone();
        recorder.record(self.session_event("agent.session.archived"));
        Ok(self)
    }

    // --- Metric Recording ---

    pub fn record_message_received(&mut self) {
        self.message_count = self.message_count.saturating_add(1);
    }

    pub fn record_tool_call(&mut self) {
        self.tool_call_count = self.tool_call_count.saturating_add(1);
    }

    pub fn record_compression(&mut self) {
        self.compression_count = self.compression_count.saturating_add(1);
    }

    pub fn record_cost(&mut self, cents: u64) {
        self.cost_cents = Some(self.cost_cents.unwrap_or(0).saturating_add(cents));
    }

    pub fn add_child_session(&mut self, child_session_id: impl Into<String>) {
        let child_id = child_session_id.into();
        if !self.child_session_ids.contains(&child_id) {
            self.child_session_ids.push(child_id);
        }
    }

    pub fn remove_child_session(&mut self, child_session_id: &str) {
        self.child_session_ids.retain(|id| id != child_session_id);
    }

    pub fn update_preview(&mut self, preview: impl Into<String>) {
        if self.preview.is_none() {
            self.preview = Some(preview.into());
        }
    }

    pub fn update_summary(&mut self, summary: impl Into<String>) {
        self.summary = Some(summary.into());
    }

    pub fn update_context_chain(
        &mut self,
        context_from: impl Into<String>,
        context_watermark: impl Into<String>,
    ) {
        self.context_from = Some(context_from.into());
        self.context_watermark = Some(context_watermark.into());
    }

    // --- Internal Helpers ---

    fn ensure_session_state(&self, expected: SessionState, action: &str) -> KernelResult<()> {
        if self.state == expected {
            return Ok(());
        }

        Err(KernelError::validation(format!(
            "cannot {action} session from state {:?}",
            self.state
        )))
    }

    fn session_event(&self, event_type: &str) -> KernelEvent {
        KernelEvent::new(
            format!("event.{}.{}", self.session_id, event_type),
            event_type,
            KernelEventSeverity::Info,
            format!(
                "session_id={};kind={};source={};agent_id={};user_ref={}",
                self.session_id,
                self.kind.as_str(),
                self.source.as_str(),
                self.agent_id.as_deref().unwrap_or(""),
                self.user_ref.as_deref().unwrap_or("")
            ),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created,
    Accepted,
    Planned,
    Running,
    AwaitingPermission,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTask {
    pub task_id: String,
    pub session_id: String,
    pub instruction: String,
    pub state: TaskState,
}

impl AgentTask {
    pub fn new(
        task_id: impl Into<String>,
        session_id: impl Into<String>,
        instruction: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            session_id: session_id.into(),
            instruction: instruction.into(),
            state: TaskState::Created,
        }
    }

    pub fn accept(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        self.ensure_task_state(TaskState::Created, "accept")?;
        self.state = TaskState::Accepted;
        recorder.record(self.task_event("agent.task.accepted"));
        Ok(self)
    }

    pub fn start(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        self.ensure_task_state(TaskState::Accepted, "start")?;
        self.state = TaskState::Running;
        recorder.record(self.task_event("agent.task.started"));
        Ok(self)
    }

    pub fn complete(self) -> KernelResult<Self> {
        if self.state != TaskState::Running {
            return Err(KernelError::validation(format!(
                "cannot complete task from state {:?}",
                self.state
            )));
        }

        Ok(Self {
            state: TaskState::Completed,
            ..self
        })
    }

    pub fn complete_with_events(mut self, recorder: &mut EventRecorder) -> KernelResult<Self> {
        self.ensure_task_state(TaskState::Running, "complete")?;
        self.state = TaskState::Completed;
        recorder.record(self.task_event("agent.task.completed"));
        Ok(self)
    }

    pub fn retry_as(&self, run_id: impl Into<String>) -> AgentRun {
        AgentRun::new(run_id, self.task_id.clone())
    }

    fn ensure_task_state(&self, expected: TaskState, action: &str) -> KernelResult<()> {
        if self.state == expected {
            return Ok(());
        }

        Err(KernelError::validation(format!(
            "cannot {action} task from state {:?}",
            self.state
        )))
    }

    fn task_event(&self, event_type: &str) -> KernelEvent {
        KernelEvent::new(
            format!("event.{}.{}", self.task_id, event_type),
            event_type,
            KernelEventSeverity::Info,
            format!("task_id={};session_id={}", self.task_id, self.session_id),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Created,
    Planning,
    Executing,
    AwaitingPermission,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRun {
    pub run_id: String,
    pub task_id: String,
    pub state: RunState,
}

impl AgentRun {
    pub fn new(run_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            task_id: task_id.into(),
            state: RunState::Created,
        }
    }

    pub fn complete(mut self) -> KernelResult<Self> {
        match self.state {
            RunState::Created | RunState::Planning | RunState::Executing => {
                self.state = RunState::Completed;
                Ok(self)
            }
            _ => Err(KernelError::validation(format!(
                "cannot complete run from state {:?}",
                self.state
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Created,
    Ready,
    Running,
    AwaitingPermission,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStep {
    pub step_id: String,
    pub run_id: String,
    pub kind: String,
    pub state: StepState,
    pub error_reason: Option<String>,
}

impl AgentStep {
    pub fn new(
        step_id: impl Into<String>,
        run_id: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            run_id: run_id.into(),
            kind: kind.into(),
            state: StepState::Created,
            error_reason: None,
        }
    }

    pub fn mark_ready(mut self) -> KernelResult<Self> {
        if self.state != StepState::Created {
            return Err(KernelError::validation(format!(
                "cannot mark step ready from state {:?}",
                self.state
            )));
        }

        self.state = StepState::Ready;
        Ok(self)
    }

    pub fn deny_by_policy(
        mut self,
        recorder: &mut EventRecorder,
        reason_code: impl Into<String>,
    ) -> KernelResult<Self> {
        if self.state != StepState::Ready && self.state != StepState::AwaitingPermission {
            return Err(KernelError::validation(format!(
                "cannot deny step from state {:?}",
                self.state
            )));
        }

        let reason_code = reason_code.into();
        self.state = StepState::Failed;
        self.error_reason = Some(reason_code.clone());
        recorder.record(KernelEvent::new(
            format!("event.{}.agent.step.denied", self.step_id),
            "agent.step.denied",
            KernelEventSeverity::Warn,
            format!(
                "step_id={};run_id={};reason_code={}",
                self.step_id, self.run_id, reason_code
            ),
        ));
        Ok(self)
    }
}
