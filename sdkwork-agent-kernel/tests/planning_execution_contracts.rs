use sdkwork_agent_kernel::{
    Action, ActionKind, ActionStatus, Observation, Plan, PlanningProvider, SideEffectLevel,
};

#[test]
fn plan_contains_ordered_actions_with_required_capabilities() {
    let plan = Plan::new("plan.1", "task.1", "run.1", "Answer with a tool")
        .add_action(Action::new(
            "action.1",
            ActionKind::ModelCall,
            "Reason about the request",
        ))
        .add_action(
            Action::new("action.2", ActionKind::ToolCall, "Call search tool")
                .with_required_capabilities(vec!["tool.invoke".to_string()])
                .with_side_effect_level(SideEffectLevel::SideEffectful)
                .with_policy_categories(vec!["tool.invoke".to_string()]),
        );

    assert_eq!(plan.actions.len(), 2);
    assert_eq!(plan.actions[0].action_id, "action.1");
    assert_eq!(plan.actions[1].required_capabilities, ["tool.invoke"]);
}

#[test]
fn side_effectful_action_without_policy_category_fails_validation() {
    let action = Action::new("action.1", ActionKind::ToolCall, "Run risky tool")
        .with_side_effect_level(SideEffectLevel::SideEffectful);

    let error = action.validate().expect_err("policy category is required");

    assert!(error.to_string().contains("policy category"));
}

#[test]
fn action_can_transition_to_waiting_for_approval() {
    let action = Action::new("action.1", ActionKind::HostOperation, "Write file")
        .with_side_effect_level(SideEffectLevel::Destructive)
        .with_policy_categories(vec!["host.filesystem.write".to_string()])
        .mark_waiting_for_approval("permission.1")
        .unwrap();

    assert_eq!(action.status, ActionStatus::AwaitingApproval);
    assert_eq!(
        action.permission_request_id.as_deref(),
        Some("permission.1")
    );
}

#[test]
fn observation_preserves_action_step_and_provenance() {
    let observation = Observation::new("observation.1", "action.1", "step.1", "tool returned data")
        .with_provenance("tool-call.1");

    assert_eq!(observation.action_id, "action.1");
    assert_eq!(observation.step_id, "step.1");
    assert_eq!(observation.provenance.as_deref(), Some("tool-call.1"));
}

#[test]
fn plan_revision_increments_revision_and_preserves_previous_plan_id() {
    let plan = Plan::new("plan.1", "task.1", "run.1", "Initial plan");
    let revised = plan.revise_as("plan.2", "Revised plan");

    assert_eq!(revised.plan_id, "plan.2");
    assert_eq!(revised.revision, 2);
    assert_eq!(revised.previous_plan_id.as_deref(), Some("plan.1"));
}

#[test]
fn planning_provider_trait_can_create_and_validate_plan() {
    let provider = FakePlanningProvider;
    let plan = provider.create_plan("task.1", "run.1", "do work");

    provider.validate_plan(&plan).expect("plan validates");
    assert_eq!(plan.task_id, "task.1");
}

struct FakePlanningProvider;

impl PlanningProvider for FakePlanningProvider {
    fn create_plan(&self, task_id: &str, run_id: &str, summary: &str) -> Plan {
        Plan::new("plan.fake", task_id, run_id, summary).add_action(Action::new(
            "action.fake",
            ActionKind::Internal,
            "internal action",
        ))
    }
}
