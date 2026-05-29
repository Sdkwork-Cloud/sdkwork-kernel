use crate::{EventRecorder, KernelError, KernelEvent, KernelEventSeverity, KernelResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Active,
    Paused,
    Closed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    pub session_id: String,
    pub state: SessionState,
}

impl AgentSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            state: SessionState::Created,
        }
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
