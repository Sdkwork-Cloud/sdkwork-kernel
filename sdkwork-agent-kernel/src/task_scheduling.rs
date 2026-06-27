use crate::{KernelError, KernelResult, ProviderHealth, ProviderManifest};

// ============================================================================
// Schedule State
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleState {
    Created,
    Scheduled,
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

impl ScheduleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Scheduled => "scheduled",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "created" => Some(Self::Created),
            "scheduled" => Some(Self::Scheduled),
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Scheduled | Self::Queued | Self::Running)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Expired
        )
    }
}

// ============================================================================
// Trigger Kind - how a scheduled task is triggered
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    /// Execute once at a specific time
    OneShot,
    /// Execute on a cron schedule
    Cron,
    /// Execute at fixed intervals
    Interval,
    /// Execute when a specific event occurs
    Event,
}

impl TriggerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OneShot => "one_shot",
            Self::Cron => "cron",
            Self::Interval => "interval",
            Self::Event => "event",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "one_shot" => Some(Self::OneShot),
            "cron" => Some(Self::Cron),
            "interval" => Some(Self::Interval),
            "event" => Some(Self::Event),
            _ => None,
        }
    }
}

// ============================================================================
// Task Priority
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn numeric_value(&self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }
}

// ============================================================================
// Task Trigger - defines when and how a task executes
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTrigger {
    pub kind: TriggerKind,
    /// Cron expression for Cron triggers (e.g., "0 */5 * * * *")
    pub cron_expression: Option<String>,
    /// Interval in seconds for Interval triggers
    pub interval_seconds: Option<u64>,
    /// Event name for Event triggers
    pub event_name: Option<String>,
    /// Specific execution time for OneShot triggers (ISO 8601)
    pub scheduled_at: Option<String>,
    /// Timezone offset in minutes from UTC (default: 0)
    pub timezone_offset_minutes: i32,
}

impl TaskTrigger {
    pub fn one_shot(scheduled_at: impl Into<String>) -> Self {
        Self {
            kind: TriggerKind::OneShot,
            cron_expression: None,
            interval_seconds: None,
            event_name: None,
            scheduled_at: Some(scheduled_at.into()),
            timezone_offset_minutes: 0,
        }
    }

    pub fn cron(cron_expression: impl Into<String>) -> Self {
        Self {
            kind: TriggerKind::Cron,
            cron_expression: Some(cron_expression.into()),
            interval_seconds: None,
            event_name: None,
            scheduled_at: None,
            timezone_offset_minutes: 0,
        }
    }

    pub fn interval(interval_seconds: u64) -> Self {
        Self {
            kind: TriggerKind::Interval,
            cron_expression: None,
            interval_seconds: Some(interval_seconds),
            event_name: None,
            scheduled_at: None,
            timezone_offset_minutes: 0,
        }
    }

    pub fn event(event_name: impl Into<String>) -> Self {
        Self {
            kind: TriggerKind::Event,
            cron_expression: None,
            interval_seconds: None,
            event_name: Some(event_name.into()),
            scheduled_at: None,
            timezone_offset_minutes: 0,
        }
    }

    pub fn with_timezone_offset(mut self, offset_minutes: i32) -> Self {
        self.timezone_offset_minutes = offset_minutes;
        self
    }

    pub fn validate(&self) -> KernelResult<()> {
        match self.kind {
            TriggerKind::OneShot => {
                if self.scheduled_at.as_deref().is_none_or(str::is_empty) {
                    return Err(KernelError::validation(
                        "one_shot trigger requires a scheduled_at time",
                    ));
                }
            }
            TriggerKind::Cron => {
                if self.cron_expression.as_deref().is_none_or(str::is_empty) {
                    return Err(KernelError::validation(
                        "cron trigger requires a cron_expression",
                    ));
                }
            }
            TriggerKind::Interval => {
                if self.interval_seconds.is_none_or(|seconds| seconds == 0) {
                    return Err(KernelError::validation(
                        "interval trigger requires a positive interval_seconds",
                    ));
                }
            }
            TriggerKind::Event => {
                if self.event_name.as_deref().is_none_or(str::is_empty) {
                    return Err(KernelError::validation(
                        "event trigger requires an event_name",
                    ));
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// Scheduled Task - a task managed by the scheduling SPI
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTask {
    pub schedule_id: String,
    pub task_id: String,
    pub session_id: String,
    pub agent_id: Option<String>,
    pub instruction: String,
    pub trigger: TaskTrigger,
    pub state: ScheduleState,
    pub priority: TaskPriority,
    pub scheduled_at: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub max_retries: u32,
    pub retry_count: u32,
    pub retry_delay_seconds: u64,
    pub timeout_ms: Option<u64>,
    pub tenant_id: Option<String>,
    pub user_ref: Option<String>,
    pub metadata: Vec<(String, String)>,
}

impl ScheduledTask {
    pub fn new(
        schedule_id: impl Into<String>,
        task_id: impl Into<String>,
        session_id: impl Into<String>,
        instruction: impl Into<String>,
        trigger: TaskTrigger,
    ) -> Self {
        Self {
            schedule_id: schedule_id.into(),
            task_id: task_id.into(),
            session_id: session_id.into(),
            agent_id: None,
            instruction: instruction.into(),
            trigger,
            state: ScheduleState::Created,
            priority: TaskPriority::Normal,
            scheduled_at: None,
            next_run_at: None,
            last_run_at: None,
            max_retries: 3,
            retry_count: 0,
            retry_delay_seconds: 30,
            timeout_ms: None,
            tenant_id: None,
            user_ref: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_scheduled_at(mut self, scheduled_at: impl Into<String>) -> Self {
        self.scheduled_at = Some(scheduled_at.into());
        self
    }

    pub fn with_next_run_at(mut self, next_run_at: impl Into<String>) -> Self {
        self.next_run_at = Some(next_run_at.into());
        self
    }

    pub fn with_last_run_at(mut self, last_run_at: impl Into<String>) -> Self {
        self.last_run_at = Some(last_run_at.into());
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_retry_delay_seconds(mut self, retry_delay_seconds: u64) -> Self {
        self.retry_delay_seconds = retry_delay_seconds;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_user_ref(mut self, user_ref: impl Into<String>) -> Self {
        self.user_ref = Some(user_ref.into());
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

    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.schedule_id.trim().is_empty() {
            return Err(KernelError::validation("schedule_id must not be empty"));
        }
        if self.task_id.trim().is_empty() {
            return Err(KernelError::validation("task_id must not be empty"));
        }
        if self.session_id.trim().is_empty() {
            return Err(KernelError::validation("session_id must not be empty"));
        }
        if self.instruction.trim().is_empty() {
            return Err(KernelError::validation("instruction must not be empty"));
        }
        self.trigger.validate()?;
        Ok(())
    }

    pub fn activate(mut self) -> KernelResult<Self> {
        if self.state != ScheduleState::Created {
            return Err(KernelError::validation(format!(
                "cannot activate schedule from state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Scheduled;
        Ok(self)
    }

    pub fn queue(mut self) -> KernelResult<Self> {
        if !matches!(self.state, ScheduleState::Scheduled) {
            return Err(KernelError::validation(format!(
                "cannot queue schedule from state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Queued;
        Ok(self)
    }

    pub fn start(mut self) -> KernelResult<Self> {
        if !matches!(self.state, ScheduleState::Queued | ScheduleState::Scheduled) {
            return Err(KernelError::validation(format!(
                "cannot start schedule from state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Running;
        Ok(self)
    }

    pub fn complete(mut self) -> KernelResult<Self> {
        if self.state != ScheduleState::Running {
            return Err(KernelError::validation(format!(
                "cannot complete schedule from state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Completed;
        Ok(self)
    }

    pub fn fail(mut self) -> KernelResult<Self> {
        if self.state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot fail schedule from terminal state {:?}",
                self.state
            )));
        }
        if self.can_retry() {
            self.retry_count = self.retry_count.saturating_add(1);
            self.state = ScheduleState::Scheduled;
        } else {
            self.state = ScheduleState::Failed;
        }
        Ok(self)
    }

    pub fn pause(mut self) -> KernelResult<Self> {
        if !self.state.is_active() {
            return Err(KernelError::validation(format!(
                "cannot pause schedule from state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Paused;
        Ok(self)
    }

    pub fn resume(mut self) -> KernelResult<Self> {
        if self.state != ScheduleState::Paused {
            return Err(KernelError::validation(format!(
                "cannot resume schedule from state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Scheduled;
        Ok(self)
    }

    pub fn cancel(mut self) -> KernelResult<Self> {
        if self.state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot cancel schedule from terminal state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Cancelled;
        Ok(self)
    }

    pub fn expire(mut self) -> KernelResult<Self> {
        if self.state.is_terminal() {
            return Err(KernelError::validation(format!(
                "cannot expire schedule from terminal state {:?}",
                self.state
            )));
        }
        self.state = ScheduleState::Expired;
        Ok(self)
    }
}

// ============================================================================
// Schedule Query - filter for querying scheduled tasks
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduleQuery {
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub trigger_kind: Option<TriggerKind>,
    pub tenant_id: Option<String>,
    pub user_ref: Option<String>,
    pub priority: Option<TaskPriority>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl ScheduleQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn in_state(mut self, state: ScheduleState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_trigger_kind(mut self, kind: TriggerKind) -> Self {
        self.trigger_kind = Some(kind);
        self
    }

    pub fn for_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn for_user(mut self, user_ref: impl Into<String>) -> Self {
        self.user_ref = Some(user_ref.into());
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn matches(&self, task: &ScheduledTask) -> bool {
        if let Some(session_id) = &self.session_id {
            if &task.session_id != session_id {
                return false;
            }
        }
        if let Some(agent_id) = &self.agent_id {
            if task.agent_id.as_deref() != Some(agent_id.as_str()) {
                return false;
            }
        }
        if let Some(state) = self.state {
            if task.state != state {
                return false;
            }
        }
        if let Some(trigger_kind) = self.trigger_kind {
            if task.trigger.kind != trigger_kind {
                return false;
            }
        }
        if let Some(tenant_id) = &self.tenant_id {
            if task.tenant_id.as_deref() != Some(tenant_id.as_str()) {
                return false;
            }
        }
        if let Some(user_ref) = &self.user_ref {
            if task.user_ref.as_deref() != Some(user_ref.as_str()) {
                return false;
            }
        }
        if let Some(priority) = self.priority {
            if task.priority != priority {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Schedule Result - result of a scheduling operation
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleResult {
    pub schedule_id: String,
    pub accepted: bool,
    pub next_run_at: Option<String>,
    pub message: Option<String>,
}

impl ScheduleResult {
    pub fn accepted(schedule_id: impl Into<String>, next_run_at: Option<String>) -> Self {
        Self {
            schedule_id: schedule_id.into(),
            accepted: true,
            next_run_at,
            message: None,
        }
    }

    pub fn rejected(schedule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schedule_id: schedule_id.into(),
            accepted: false,
            next_run_at: None,
            message: Some(message.into()),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

// ============================================================================
// Task Scheduling Provider - the SPI trait
// ============================================================================

pub trait TaskSchedulingProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.task_scheduling.unspecified",
            "task_scheduling",
            "task-scheduling-provider",
            "0.0.0",
            vec![
                "task.schedule".to_string(),
                "task.cancel".to_string(),
                "task.list".to_string(),
                "task.pause".to_string(),
                "task.resume".to_string(),
                "task.get_due".to_string(),
            ],
        )
    }

    fn schedule(&mut self, task: ScheduledTask) -> KernelResult<ScheduleResult>;

    fn cancel(&mut self, schedule_id: &str) -> KernelResult<ScheduleResult>;

    fn pause(&mut self, schedule_id: &str) -> KernelResult<ScheduleResult>;

    fn resume(&mut self, schedule_id: &str) -> KernelResult<ScheduleResult>;

    fn get(&self, schedule_id: &str) -> KernelResult<ScheduledTask>;

    fn list(&self, query: &ScheduleQuery) -> KernelResult<Vec<ScheduledTask>>;

    fn get_due(&self, current_time: &str) -> KernelResult<Vec<ScheduledTask>>;

    fn health(&self) -> ProviderHealth;
}
