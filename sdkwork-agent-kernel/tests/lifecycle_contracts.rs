use sdkwork_agent_kernel::{
    AgentRun, AgentSession, AgentStep, AgentTask, EventRecorder, KernelError, RunState,
    SessionState, StepState, TaskState,
};

#[test]
fn session_task_run_and_step_start_in_created_states() {
    let session = AgentSession::new("session.1");
    let task = AgentTask::new("task.1", session.session_id.clone(), "summarize repository");
    let run = AgentRun::new("run.1", task.task_id.clone());
    let step = AgentStep::new("step.1", run.run_id.clone(), "model_call");

    assert_eq!(session.state, SessionState::Created);
    assert_eq!(task.state, TaskState::Created);
    assert_eq!(run.state, RunState::Created);
    assert_eq!(step.state, StepState::Created);
}

#[test]
fn task_retry_creates_a_new_run_without_overwriting_previous_run() {
    let task = AgentTask::new("task.1", "session.1", "do work");
    let first_run = AgentRun::new("run.1", task.task_id.clone())
        .complete()
        .unwrap();

    let retry = task.retry_as("run.2");

    assert_eq!(first_run.run_id, "run.1");
    assert_eq!(first_run.state, RunState::Completed);
    assert_eq!(retry.run_id, "run.2");
    assert_eq!(retry.task_id, "task.1");
    assert_eq!(retry.state, RunState::Created);
}

#[test]
fn invalid_task_transition_returns_validation_error() {
    let task = AgentTask::new("task.1", "session.1", "do work");
    let error = task
        .complete()
        .expect_err("created task cannot complete directly");

    assert!(matches!(error, KernelError::Validation { .. }));
}

#[test]
fn lifecycle_transitions_emit_kernel_events() {
    let mut recorder = EventRecorder::new();
    let task = AgentTask::new("task.1", "session.1", "do work")
        .accept(&mut recorder)
        .unwrap()
        .start(&mut recorder)
        .unwrap()
        .complete_with_events(&mut recorder)
        .unwrap();

    assert_eq!(task.state, TaskState::Completed);
    assert_eq!(recorder.events().len(), 3);
    assert_eq!(recorder.events()[0].event_type, "agent.task.accepted");
    assert_eq!(recorder.events()[1].event_type, "agent.task.started");
    assert_eq!(recorder.events()[2].event_type, "agent.task.completed");
}

#[test]
fn step_permission_denial_is_terminal_for_the_step() {
    let mut recorder = EventRecorder::new();
    let step = AgentStep::new("step.1", "run.1", "tool_call")
        .mark_ready()
        .unwrap()
        .deny_by_policy(&mut recorder, "tool.not_allowed")
        .unwrap();

    assert_eq!(step.state, StepState::Failed);
    assert_eq!(step.error_reason.as_deref(), Some("tool.not_allowed"));
    assert_eq!(recorder.events()[0].event_type, "agent.step.denied");
}
