use sdkwork_agent_kernel::{
    KernelResult, ProviderHealth, ScheduleQuery, ScheduleResult, ScheduleState, ScheduledTask,
    TaskPriority, TaskSchedulingProvider, TaskTrigger, TriggerKind,
};

// ============================================================================
// Schedule State contracts
// ============================================================================

#[test]
fn schedule_state_str_roundtrip_preserves_identity() {
    let all_states = [
        ScheduleState::Created,
        ScheduleState::Scheduled,
        ScheduleState::Queued,
        ScheduleState::Running,
        ScheduleState::Paused,
        ScheduleState::Completed,
        ScheduleState::Failed,
        ScheduleState::Cancelled,
        ScheduleState::Expired,
    ];

    for state in &all_states {
        let label = state.as_str();
        assert_eq!(ScheduleState::from_str(label), Some(*state));
    }

    assert_eq!(ScheduleState::from_str("nonexistent"), None);
}

#[test]
fn schedule_state_active_and_terminal_classifications_are_disjoint() {
    assert!(ScheduleState::Scheduled.is_active());
    assert!(ScheduleState::Queued.is_active());
    assert!(ScheduleState::Running.is_active());
    assert!(!ScheduleState::Created.is_active());
    assert!(!ScheduleState::Paused.is_active());

    assert!(ScheduleState::Completed.is_terminal());
    assert!(ScheduleState::Failed.is_terminal());
    assert!(ScheduleState::Cancelled.is_terminal());
    assert!(ScheduleState::Expired.is_terminal());
    assert!(!ScheduleState::Scheduled.is_terminal());
    assert!(!ScheduleState::Running.is_terminal());
}

// ============================================================================
// Task Trigger contracts
// ============================================================================

#[test]
fn task_trigger_one_shot_requires_scheduled_at() {
    let valid = TaskTrigger::one_shot("2026-06-27T10:00:00Z");
    assert_eq!(valid.kind, TriggerKind::OneShot);
    assert!(valid.validate().is_ok());

    let invalid = TaskTrigger::one_shot("");
    assert!(invalid.validate().is_err());
}

#[test]
fn task_trigger_cron_requires_cron_expression() {
    let valid = TaskTrigger::cron("0 */5 * * * *");
    assert_eq!(valid.kind, TriggerKind::Cron);
    assert!(valid.validate().is_ok());

    let invalid = TaskTrigger::cron("");
    assert!(invalid.validate().is_err());
}

#[test]
fn task_trigger_interval_requires_positive_seconds() {
    let valid = TaskTrigger::interval(60);
    assert_eq!(valid.kind, TriggerKind::Interval);
    assert!(valid.validate().is_ok());

    let invalid = TaskTrigger::interval(0);
    assert!(invalid.validate().is_err());
}

#[test]
fn task_trigger_event_requires_event_name() {
    let valid = TaskTrigger::event("agent.task.completed");
    assert_eq!(valid.kind, TriggerKind::Event);
    assert!(valid.validate().is_ok());

    let invalid = TaskTrigger::event("");
    assert!(invalid.validate().is_err());
}

#[test]
fn task_trigger_supports_timezone_offset() {
    let trigger = TaskTrigger::cron("0 0 * * * *").with_timezone_offset(480);
    assert_eq!(trigger.timezone_offset_minutes, 480);
}

#[test]
fn trigger_kind_str_roundtrip_preserves_identity() {
    assert_eq!(TriggerKind::OneShot.as_str(), "one_shot");
    assert_eq!(TriggerKind::Cron.as_str(), "cron");
    assert_eq!(TriggerKind::Interval.as_str(), "interval");
    assert_eq!(TriggerKind::Event.as_str(), "event");

    for kind in [
        TriggerKind::OneShot,
        TriggerKind::Cron,
        TriggerKind::Interval,
        TriggerKind::Event,
    ] {
        assert_eq!(TriggerKind::from_str(kind.as_str()), Some(kind));
    }
    assert_eq!(TriggerKind::from_str("unknown"), None);
}

// ============================================================================
// Task Priority contracts
// ============================================================================

#[test]
fn task_priority_ordering_is_monotonic() {
    assert!(TaskPriority::Critical > TaskPriority::High);
    assert!(TaskPriority::High > TaskPriority::Normal);
    assert!(TaskPriority::Normal > TaskPriority::Low);

    assert_eq!(TaskPriority::default(), TaskPriority::Normal);
    assert_eq!(TaskPriority::Low.numeric_value(), 0);
    assert_eq!(TaskPriority::Critical.numeric_value(), 3);
}

#[test]
fn task_priority_str_roundtrip_preserves_identity() {
    for priority in [
        TaskPriority::Low,
        TaskPriority::Normal,
        TaskPriority::High,
        TaskPriority::Critical,
    ] {
        assert_eq!(TaskPriority::from_str(priority.as_str()), Some(priority));
    }
    assert_eq!(TaskPriority::from_str("urgent"), None);
}

// ============================================================================
// Scheduled Task contracts
// ============================================================================

#[test]
fn scheduled_task_builder_preserves_metadata_and_priority() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "Run nightly code review",
        TaskTrigger::cron("0 0 * * * *"),
    )
    .with_agent_id("agent.codex")
    .with_priority(TaskPriority::High)
    .with_max_retries(5)
    .with_retry_delay_seconds(60)
    .with_timeout_ms(30_000)
    .with_tenant_id("tenant.sdkwork")
    .with_user_ref("user.1")
    .with_metadata("env", "production")
    .with_next_run_at("2026-06-27T00:00:00Z");

    assert_eq!(task.schedule_id, "schedule.1");
    assert_eq!(task.task_id, "task.1");
    assert_eq!(task.session_id, "session.1");
    assert_eq!(task.agent_id.as_deref(), Some("agent.codex"));
    assert_eq!(task.priority, TaskPriority::High);
    assert_eq!(task.max_retries, 5);
    assert_eq!(task.retry_delay_seconds, 60);
    assert_eq!(task.timeout_ms, Some(30_000));
    assert_eq!(task.tenant_id.as_deref(), Some("tenant.sdkwork"));
    assert_eq!(task.user_ref.as_deref(), Some("user.1"));
    assert_eq!(task.metadata_value("env"), Some("production"));
    assert_eq!(task.next_run_at.as_deref(), Some("2026-06-27T00:00:00Z"));
    assert_eq!(task.state, ScheduleState::Created);
}

#[test]
fn scheduled_task_validate_rejects_empty_required_fields() {
    let task = ScheduledTask::new(
        "",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    );
    assert!(task.validate().is_err());

    let task = ScheduledTask::new(
        "schedule.1",
        "",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    );
    assert!(task.validate().is_err());

    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "",
        "do work",
        TaskTrigger::interval(60),
    );
    assert!(task.validate().is_err());

    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "",
        TaskTrigger::interval(60),
    );
    assert!(task.validate().is_err());
}

#[test]
fn scheduled_task_validate_rejects_invalid_trigger() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::cron(""),
    );
    assert!(task.validate().is_err());
}

#[test]
fn scheduled_task_lifecycle_transitions_follow_state_machine() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    );

    // Created -> Scheduled
    let task = task.activate().expect("activate from Created");
    assert_eq!(task.state, ScheduleState::Scheduled);
    assert!(task.is_active());

    // Scheduled -> Queued
    let task = task.queue().expect("queue from Scheduled");
    assert_eq!(task.state, ScheduleState::Queued);

    // Queued -> Running
    let task = task.start().expect("start from Queued");
    assert_eq!(task.state, ScheduleState::Running);

    // Running -> Completed
    let task = task.complete().expect("complete from Running");
    assert_eq!(task.state, ScheduleState::Completed);
    assert!(task.is_terminal());
}

#[test]
fn scheduled_task_pause_and_resume_round_trip() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    )
    .activate()
    .expect("activate");

    let task = task.pause().expect("pause from Scheduled");
    assert_eq!(task.state, ScheduleState::Paused);
    assert!(!task.is_active());

    let task = task.resume().expect("resume from Paused");
    assert_eq!(task.state, ScheduleState::Scheduled);
}

#[test]
fn scheduled_task_cancel_from_active_state() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    )
    .activate()
    .expect("activate");

    let task = task.cancel().expect("cancel from Scheduled");
    assert_eq!(task.state, ScheduleState::Cancelled);
    assert!(task.is_terminal());
}

#[test]
fn scheduled_task_cancel_from_terminal_state_is_rejected() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    )
    .activate()
    .expect("activate")
    .cancel()
    .expect("cancel");

    assert!(task.cancel().is_err());
}

#[test]
fn scheduled_task_fail_with_retry_reschedules() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    )
    .with_max_retries(2)
    .activate()
    .expect("activate")
    .start()
    .expect("start");

    let retry_count_before = task.retry_count;
    let task = task.fail().expect("fail with retries available");
    assert_eq!(task.retry_count, retry_count_before + 1);
    assert_eq!(task.state, ScheduleState::Scheduled);
    assert!(task.can_retry());
}

#[test]
fn scheduled_task_fail_without_retries_marks_failed() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    )
    .with_max_retries(0)
    .activate()
    .expect("activate")
    .start()
    .expect("start");

    let task = task.fail().expect("fail without retries");
    assert_eq!(task.state, ScheduleState::Failed);
    assert!(task.is_terminal());
    assert!(!task.can_retry());
}

// ============================================================================
// Schedule Query contracts
// ============================================================================

#[test]
fn schedule_query_matches_by_session_and_state() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::interval(60),
    )
    .activate()
    .expect("activate");

    let query = ScheduleQuery::new()
        .for_session("session.1")
        .in_state(ScheduleState::Scheduled);

    assert!(query.matches(&task));

    let wrong_session = ScheduleQuery::new().for_session("session.2");
    assert!(!wrong_session.matches(&task));

    let wrong_state = ScheduleQuery::new().in_state(ScheduleState::Completed);
    assert!(!wrong_state.matches(&task));
}

#[test]
fn schedule_query_matches_by_trigger_kind_and_priority() {
    let task = ScheduledTask::new(
        "schedule.1",
        "task.1",
        "session.1",
        "do work",
        TaskTrigger::cron("0 * * * * *"),
    )
    .with_priority(TaskPriority::High)
    .with_tenant_id("tenant.1");

    let query = ScheduleQuery::new()
        .with_trigger_kind(TriggerKind::Cron)
        .with_priority(TaskPriority::High)
        .for_tenant("tenant.1");

    assert!(query.matches(&task));

    let mismatch = ScheduleQuery::new().with_trigger_kind(TriggerKind::OneShot);
    assert!(!mismatch.matches(&task));
}

// ============================================================================
// Schedule Result contracts
// ============================================================================

#[test]
fn schedule_result_accepted_carries_next_run_at() {
    let result = ScheduleResult::accepted("schedule.1", Some("2026-06-27T00:00:00Z".to_string()));

    assert!(result.accepted);
    assert_eq!(result.schedule_id, "schedule.1");
    assert_eq!(result.next_run_at.as_deref(), Some("2026-06-27T00:00:00Z"));
    assert!(result.message.is_none());
}

#[test]
fn schedule_result_rejected_carries_message() {
    let result =
        ScheduleResult::rejected("schedule.1", "duplicate schedule id").with_message("conflict");

    assert!(!result.accepted);
    assert_eq!(result.schedule_id, "schedule.1");
    assert_eq!(result.message.as_deref(), Some("conflict"));
    assert!(result.next_run_at.is_none());
}

// ============================================================================
// Task Scheduling Provider SPI contracts
// ============================================================================

#[test]
fn task_scheduling_provider_manifest_declares_standard_capabilities() {
    let provider = FakeTaskSchedulingProvider::default();
    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_family, "task_scheduling");
    assert!(manifest.capabilities.contains(&"task.schedule".to_string()));
    assert!(manifest.capabilities.contains(&"task.cancel".to_string()));
    assert!(manifest.capabilities.contains(&"task.get_due".to_string()));
}

#[test]
fn task_scheduling_provider_schedule_accepts_and_stores_task() {
    let mut provider = FakeTaskSchedulingProvider::default();
    let task = ScheduledTask::new(
        "schedule.provider.1",
        "task.1",
        "session.1",
        "run code review",
        TaskTrigger::cron("0 0 * * * *"),
    )
    .activate()
    .expect("activate");

    let result = provider.schedule(task).expect("schedule succeeds");
    assert!(result.accepted);
    assert_eq!(result.schedule_id, "schedule.provider.1");

    let retrieved = provider.get("schedule.provider.1").expect("get succeeds");
    assert_eq!(retrieved.schedule_id, "schedule.provider.1");
    assert_eq!(retrieved.state, ScheduleState::Scheduled);
}

#[test]
fn task_scheduling_provider_cancel_pauses_and_resumes() {
    let mut provider = FakeTaskSchedulingProvider::default();
    let task = ScheduledTask::new(
        "schedule.provider.2",
        "task.2",
        "session.2",
        "run tests",
        TaskTrigger::interval(120),
    )
    .activate()
    .expect("activate");

    provider.schedule(task).expect("schedule succeeds");

    let pause_result = provider
        .pause("schedule.provider.2")
        .expect("pause succeeds");
    assert!(pause_result.accepted);

    let paused = provider.get("schedule.provider.2").expect("get succeeds");
    assert_eq!(paused.state, ScheduleState::Paused);

    let resume_result = provider
        .resume("schedule.provider.2")
        .expect("resume succeeds");
    assert!(resume_result.accepted);

    let resumed = provider.get("schedule.provider.2").expect("get succeeds");
    assert_eq!(resumed.state, ScheduleState::Scheduled);
}

#[test]
fn task_scheduling_provider_list_filters_by_query() {
    let mut provider = FakeTaskSchedulingProvider::default();

    for i in 1..=3 {
        let task = ScheduledTask::new(
            format!("schedule.list.{i}"),
            format!("task.{i}"),
            "session.list",
            format!("instruction {i}"),
            TaskTrigger::interval(60),
        )
        .activate()
        .expect("activate");
        provider.schedule(task).expect("schedule succeeds");
    }

    // Different session
    let other = ScheduledTask::new(
        "schedule.other",
        "task.other",
        "session.other",
        "other instruction",
        TaskTrigger::interval(60),
    )
    .activate()
    .expect("activate");
    provider.schedule(other).expect("schedule succeeds");

    let query = ScheduleQuery::new().for_session("session.list");
    let results = provider.list(&query).expect("list succeeds");
    assert_eq!(results.len(), 3);
}

#[test]
fn task_scheduling_provider_get_due_returns_tasks_with_next_run_before_current_time() {
    let mut provider = FakeTaskSchedulingProvider::default();

    let due_task = ScheduledTask::new(
        "schedule.due.1",
        "task.due.1",
        "session.due",
        "due instruction",
        TaskTrigger::one_shot("2026-06-27T00:00:00Z"),
    )
    .activate()
    .expect("activate")
    .with_next_run_at("2026-06-27T00:00:00Z");
    provider.schedule(due_task).expect("schedule succeeds");

    let future_task = ScheduledTask::new(
        "schedule.due.2",
        "task.due.2",
        "session.due",
        "future instruction",
        TaskTrigger::one_shot("2026-06-28T00:00:00Z"),
    )
    .activate()
    .expect("activate")
    .with_next_run_at("2026-06-28T00:00:00Z");
    provider.schedule(future_task).expect("schedule succeeds");

    let due = provider
        .get_due("2026-06-27T12:00:00Z")
        .expect("get_due succeeds");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].schedule_id, "schedule.due.1");
}

#[test]
fn task_scheduling_provider_cancel_removes_task() {
    let mut provider = FakeTaskSchedulingProvider::default();
    let task = ScheduledTask::new(
        "schedule.cancel.1",
        "task.cancel",
        "session.cancel",
        "to be cancelled",
        TaskTrigger::interval(60),
    )
    .activate()
    .expect("activate");
    provider.schedule(task).expect("schedule succeeds");

    let result = provider
        .cancel("schedule.cancel.1")
        .expect("cancel succeeds");
    assert!(result.accepted);

    let cancelled = provider.get("schedule.cancel.1").expect("get succeeds");
    assert_eq!(cancelled.state, ScheduleState::Cancelled);
}

// ============================================================================
// Fake Task Scheduling Provider
// ============================================================================

#[derive(Default)]
struct FakeTaskSchedulingProvider {
    tasks: Vec<ScheduledTask>,
}

impl TaskSchedulingProvider for FakeTaskSchedulingProvider {
    fn schedule(&mut self, task: ScheduledTask) -> KernelResult<ScheduleResult> {
        let schedule_id = task.schedule_id.clone();
        let next_run = task.next_run_at.clone();
        self.tasks.push(task);
        Ok(ScheduleResult::accepted(schedule_id, next_run))
    }

    fn cancel(&mut self, schedule_id: &str) -> KernelResult<ScheduleResult> {
        for task in &mut self.tasks {
            if task.schedule_id == schedule_id {
                *task = task.clone().cancel()?;
                return Ok(ScheduleResult::accepted(schedule_id, None));
            }
        }
        Err(sdkwork_agent_kernel::KernelError::validation(
            "schedule not found",
        ))
    }

    fn pause(&mut self, schedule_id: &str) -> KernelResult<ScheduleResult> {
        for task in &mut self.tasks {
            if task.schedule_id == schedule_id {
                *task = task.clone().pause()?;
                return Ok(ScheduleResult::accepted(schedule_id, None));
            }
        }
        Err(sdkwork_agent_kernel::KernelError::validation(
            "schedule not found",
        ))
    }

    fn resume(&mut self, schedule_id: &str) -> KernelResult<ScheduleResult> {
        for task in &mut self.tasks {
            if task.schedule_id == schedule_id {
                *task = task.clone().resume()?;
                return Ok(ScheduleResult::accepted(schedule_id, None));
            }
        }
        Err(sdkwork_agent_kernel::KernelError::validation(
            "schedule not found",
        ))
    }

    fn get(&self, schedule_id: &str) -> KernelResult<ScheduledTask> {
        self.tasks
            .iter()
            .find(|task| task.schedule_id == schedule_id)
            .cloned()
            .ok_or_else(|| sdkwork_agent_kernel::KernelError::validation("schedule not found"))
    }

    fn list(&self, query: &ScheduleQuery) -> KernelResult<Vec<ScheduledTask>> {
        Ok(self
            .tasks
            .iter()
            .filter(|task| query.matches(task))
            .cloned()
            .collect())
    }

    fn get_due(&self, current_time: &str) -> KernelResult<Vec<ScheduledTask>> {
        Ok(self
            .tasks
            .iter()
            .filter(|task| {
                task.state.is_active()
                    && task
                        .next_run_at
                        .as_deref()
                        .is_some_and(|next_run| next_run <= current_time)
            })
            .cloned()
            .collect())
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
