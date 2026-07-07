//! Multi-Agent Orchestration primitives for coordinating multiple agents.
//!
//! This module extends AgentCollaborationProvider with:
//! - Agent task decomposition
//! - Parallel and sequential execution
//! - Result aggregation
//! - Workflow management

use std::collections::HashMap;

use crate::{Action, ActionKind, KernelError, KernelResult, Plan, SideEffectLevel};

/// Agent task to be executed in a multi-agent orchestration plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationTask {
    /// Task identifier.
    pub task_id: String,
    /// Agent to execute this task.
    pub agent_id: String,
    /// Task objective/description.
    pub objective: String,
    /// Input data for the task.
    pub inputs: HashMap<String, String>,
    /// Expected outputs.
    pub expected_outputs: Vec<String>,
    /// Dependencies (task IDs that must complete first).
    pub dependencies: Vec<String>,
    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Priority (0-100, higher = more important).
    pub priority: u8,
    /// Retry count on failure.
    pub retry_count: u32,
}

impl OrchestrationTask {
    pub fn new(
        task_id: impl Into<String>,
        agent_id: impl Into<String>,
        objective: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            agent_id: agent_id.into(),
            objective: objective.into(),
            inputs: HashMap::new(),
            expected_outputs: Vec::new(),
            dependencies: Vec::new(),
            timeout_ms: None,
            priority: 50,
            retry_count: 0,
        }
    }

    pub fn with_input(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inputs.insert(key.into(), value.into());
        self
    }

    pub fn with_expected_output(mut self, output: impl Into<String>) -> Self {
        self.expected_outputs.push(output.into());
        self
    }

    pub fn with_dependency(mut self, task_id: impl Into<String>) -> Self {
        self.dependencies.push(task_id.into());
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(100);
        self
    }

    pub fn with_retry(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    pub fn is_ready(&self, completed_tasks: &[String]) -> bool {
        self.dependencies
            .iter()
            .all(|dep| completed_tasks.contains(dep))
    }
}

/// Agent graph representing agent relationships and capabilities.
#[derive(Debug, Clone)]
pub struct AgentGraph {
    /// Agents in the graph.
    pub agents: HashMap<String, AgentNode>,
    /// Edges (agent_id -> [dependent_agent_ids]).
    pub edges: HashMap<String, Vec<String>>,
}

impl AgentGraph {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_agent(mut self, agent_id: impl Into<String>, node: AgentNode) -> Self {
        self.agents.insert(agent_id.into(), node);
        self
    }

    pub fn add_edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        let from = from.into();
        self.edges
            .entry(from)
            .or_insert_with(Vec::new)
            .push(to.into());
        self
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentNode> {
        self.agents.get(agent_id)
    }

    pub fn get_dependencies(&self, agent_id: &str) -> Vec<&str> {
        self.edges
            .get(agent_id)
            .map(|deps| deps.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn topological_sort(&self) -> Vec<String> {
        // Kahn's algorithm for topological sort
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result: Vec<String> = Vec::new();
        let mut queue: Vec<String> = Vec::new();

        // Initialize in-degree for all agents
        for agent_id in self.agents.keys() {
            in_degree.insert(agent_id.clone(), 0);
        }

        // Calculate in-degree from edges
        for (_, deps) in &self.edges {
            for dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg += 1;
                }
            }
        }

        // Find all nodes with 0 in-degree
        for (agent_id, deg) in &in_degree {
            if *deg == 0 {
                queue.push(agent_id.clone());
            }
        }

        // Process queue
        while let Some(agent_id) = queue.pop() {
            result.push(agent_id.clone());

            if let Some(deps) = self.edges.get(&agent_id) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        result
    }
}

/// Agent node in the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNode {
    /// Agent identifier.
    pub agent_id: String,
    /// Capabilities.
    pub capabilities: Vec<String>,
    /// Max concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Current load (number of active tasks).
    pub current_load: usize,
}

impl AgentNode {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            capabilities: Vec::new(),
            max_concurrent_tasks: 1,
            current_load: 0,
        }
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_tasks = max;
        self
    }

    pub fn can_accept_task(&self) -> bool {
        self.current_load < self.max_concurrent_tasks
    }

    pub fn increment_load(&mut self) {
        self.current_load += 1;
    }

    pub fn decrement_load(&mut self) {
        self.current_load = self.current_load.saturating_sub(1);
    }
}

/// Orchestration plan for executing multiple agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationPlan {
    /// Plan identifier.
    pub plan_id: String,
    /// Tasks to execute.
    pub tasks: Vec<OrchestrationTask>,
    /// Execution strategy.
    pub strategy: ExecutionStrategy,
    /// Aggregation strategy.
    pub aggregation: AggregationStrategy,
}

impl OrchestrationPlan {
    pub fn new(plan_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            tasks: Vec::new(),
            strategy: ExecutionStrategy::Sequential,
            aggregation: AggregationStrategy::Merge,
        }
    }

    pub fn with_task(mut self, task: OrchestrationTask) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn with_strategy(mut self, strategy: ExecutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_aggregation(mut self, aggregation: AggregationStrategy) -> Self {
        self.aggregation = aggregation;
        self
    }

    pub fn get_ready_tasks(&self, completed_tasks: &[String]) -> Vec<&OrchestrationTask> {
        self.tasks
            .iter()
            .filter(|task| task.is_ready(completed_tasks))
            .collect()
    }

    pub fn get_task(&self, task_id: &str) -> Option<&OrchestrationTask> {
        self.tasks.iter().find(|task| task.task_id == task_id)
    }

    /// Convert this orchestration plan into a kernel [`Plan`] for the planning provider loop.
    pub fn into_planning_plan(
        self,
        task_id: impl Into<String>,
        run_id: impl Into<String>,
    ) -> KernelResult<Plan> {
        if self.tasks.is_empty() {
            return Err(KernelError::validation(
                "orchestration plan requires at least one task",
            ));
        }

        let task_id = task_id.into();
        let run_id = run_id.into();
        let summary = format!(
            "orchestration {} / {}",
            self.strategy.as_str(),
            self.aggregation.as_str()
        );
        let mut plan = Plan::new(self.plan_id, task_id, run_id, summary);

        for task in self.tasks {
            let mut action = Action::new(
                format!("{}.handoff", task.task_id),
                ActionKind::Handoff,
                task.objective,
            )
            .with_required_capabilities(vec!["collaboration.delegate".to_string()])
            .with_side_effect_level(SideEffectLevel::SideEffectful)
            .with_policy_categories(vec!["orchestration".to_string()]);

            if !task.dependencies.is_empty() {
                action.depends_on = task.dependencies;
            }

            plan = plan.add_action(action);
        }

        plan.validate()?;
        Ok(plan)
    }
}

/// Execution strategy for orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Execute tasks sequentially.
    Sequential,
    /// Execute independent tasks in parallel.
    Parallel,
    /// Execute tasks with priority ordering.
    Priority,
    /// Execute tasks following dependencies (dependency graph).
    Dependency,
}

impl ExecutionStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Parallel => "parallel",
            Self::Priority => "priority",
            Self::Dependency => "dependency",
        }
    }
}

/// Aggregation strategy for combining results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// Merge all results into a single result.
    Merge,
    /// Aggregate results by type.
    AggregateByType,
    /// Select the best result.
    SelectBest,
    /// Return all results separately.
    All,
}

impl AggregationStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::AggregateByType => "aggregate_by_type",
            Self::SelectBest => "select_best",
            Self::All => "all",
        }
    }
}

/// Orchestration result from executing a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationResult {
    /// Plan identifier.
    pub plan_id: String,
    /// Status of the orchestration.
    pub status: OrchestrationStatus,
    /// Task results (task_id -> result).
    pub task_results: HashMap<String, TaskResult>,
    /// Aggregated result.
    pub aggregated_result: Option<String>,
    /// Total execution time (ms).
    pub total_time_ms: u64,
    /// Number of successful tasks.
    pub successful_tasks: usize,
    /// Number of failed tasks.
    pub failed_tasks: usize,
}

impl OrchestrationResult {
    pub fn new(plan_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            status: OrchestrationStatus::Pending,
            task_results: HashMap::new(),
            aggregated_result: None,
            total_time_ms: 0,
            successful_tasks: 0,
            failed_tasks: 0,
        }
    }

    pub fn add_task_result(mut self, task_id: impl Into<String>, result: TaskResult) -> Self {
        if result.success {
            self.successful_tasks += 1;
        } else {
            self.failed_tasks += 1;
        }
        self.task_results.insert(task_id.into(), result);
        self
    }

    pub fn with_aggregated_result(mut self, result: impl Into<String>) -> Self {
        self.aggregated_result = Some(result.into());
        self
    }

    pub fn with_total_time(mut self, time_ms: u64) -> Self {
        self.total_time_ms = time_ms;
        self
    }

    pub fn finalize(mut self) -> Self {
        self.status = if self.failed_tasks == 0 {
            OrchestrationStatus::Completed
        } else if self.successful_tasks > 0 {
            OrchestrationStatus::PartialSuccess
        } else {
            OrchestrationStatus::Failed
        };
        self
    }

    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            OrchestrationStatus::Completed | OrchestrationStatus::PartialSuccess
        )
    }
}

/// Status of an orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestrationStatus {
    /// Orchestration is pending execution.
    Pending,
    /// Orchestration is in progress.
    InProgress,
    /// All tasks completed successfully.
    Completed,
    /// Some tasks succeeded, some failed.
    PartialSuccess,
    /// All tasks failed.
    Failed,
    /// Orchestration was cancelled.
    Cancelled,
}

impl OrchestrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::PartialSuccess => "partial_success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Result of a single task execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    /// Task identifier.
    pub task_id: String,
    /// Agent that executed the task.
    pub agent_id: String,
    /// Whether execution succeeded.
    pub success: bool,
    /// Result output.
    pub output: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Execution time (ms).
    pub execution_time_ms: u64,
    /// Retry attempts made.
    pub retry_attempts: u32,
}

impl TaskResult {
    pub fn success(
        task_id: impl Into<String>,
        agent_id: impl Into<String>,
        output: impl Into<String>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            agent_id: agent_id.into(),
            success: true,
            output: Some(output.into()),
            error: None,
            execution_time_ms,
            retry_attempts: 0,
        }
    }

    pub fn failure(
        task_id: impl Into<String>,
        agent_id: impl Into<String>,
        error: impl Into<String>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            agent_id: agent_id.into(),
            success: false,
            output: None,
            error: Some(error.into()),
            execution_time_ms,
            retry_attempts: 0,
        }
    }

    pub fn with_retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_task_new() {
        let task = OrchestrationTask::new("task-1", "agent-1", "Analyze data");
        assert_eq!(task.task_id, "task-1");
        assert_eq!(task.agent_id, "agent-1");
        assert_eq!(task.objective, "Analyze data");
        assert_eq!(task.priority, 50);
    }

    #[test]
    fn test_agent_task_with_dependencies() {
        let task = OrchestrationTask::new("task-2", "agent-1", "Generate report")
            .with_dependency("task-1");

        assert_eq!(task.dependencies, vec!["task-1"]);
        assert!(!task.is_ready(&[]));
        assert!(task.is_ready(&["task-1".to_string()]));
    }

    #[test]
    fn test_agent_task_priority() {
        let task = OrchestrationTask::new("task-1", "agent-1", "Test").with_priority(150); // Over 100

        assert_eq!(task.priority, 100); // Clamped to max
    }

    #[test]
    fn test_agent_graph_new() {
        let graph = AgentGraph::new();
        assert!(graph.agents.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_agent_graph_add_agent() {
        let graph = AgentGraph::new().add_agent("agent-1", AgentNode::new("agent-1"));

        assert!(graph.get_agent("agent-1").is_some());
    }

    #[test]
    fn test_agent_graph_dependencies() {
        let graph = AgentGraph::new()
            .add_agent("agent-1", AgentNode::new("agent-1"))
            .add_agent("agent-2", AgentNode::new("agent-2"))
            .add_edge("agent-1", "agent-2");

        assert_eq!(graph.get_dependencies("agent-1"), vec!["agent-2"]);
    }

    #[test]
    fn test_agent_graph_topological_sort() {
        let graph = AgentGraph::new()
            .add_agent("agent-1", AgentNode::new("agent-1"))
            .add_agent("agent-2", AgentNode::new("agent-2"))
            .add_agent("agent-3", AgentNode::new("agent-3"))
            .add_edge("agent-1", "agent-2")
            .add_edge("agent-2", "agent-3");

        let sorted = graph.topological_sort();
        // agent-1 -> agent-2 -> agent-3
        assert!(sorted.contains(&"agent-1".to_string()));
        assert!(sorted.contains(&"agent-2".to_string()));
        assert!(sorted.contains(&"agent-3".to_string()));
    }

    #[test]
    fn test_agent_node_capacity() {
        let mut node = AgentNode::new("agent-1").with_max_concurrent(3);

        assert!(node.can_accept_task());
        node.increment_load();
        assert!(node.can_accept_task());
        node.increment_load();
        node.increment_load();
        assert!(!node.can_accept_task()); // Load = 3
    }

    #[test]
    fn test_orchestration_plan_new() {
        let plan = OrchestrationPlan::new("plan-1");
        assert_eq!(plan.plan_id, "plan-1");
        assert_eq!(plan.strategy, ExecutionStrategy::Sequential);
    }

    #[test]
    fn test_orchestration_plan_ready_tasks() {
        let plan = OrchestrationPlan::new("plan-1")
            .with_task(OrchestrationTask::new("task-1", "agent-1", "Step 1"))
            .with_task(
                OrchestrationTask::new("task-2", "agent-1", "Step 2").with_dependency("task-1"),
            );

        let ready = plan.get_ready_tasks(&[]);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].task_id, "task-1");
    }

    #[test]
    fn test_orchestration_plan_into_planning_plan() {
        let orchestration = OrchestrationPlan::new("plan-1")
            .with_strategy(ExecutionStrategy::Dependency)
            .with_task(OrchestrationTask::new("task-1", "agent-1", "Analyze"))
            .with_task(
                OrchestrationTask::new("task-2", "agent-2", "Implement").with_dependency("task-1"),
            );

        let plan = orchestration
            .into_planning_plan("task.root", "run-1")
            .expect("plan conversion should succeed");
        assert_eq!(plan.plan_id, "plan-1");
        assert_eq!(plan.actions.len(), 2);
        assert_eq!(plan.actions[1].depends_on, vec!["task-1"]);
    }

    #[test]
    fn test_execution_strategy_as_str() {
        assert_eq!(ExecutionStrategy::Sequential.as_str(), "sequential");
        assert_eq!(ExecutionStrategy::Parallel.as_str(), "parallel");
    }

    #[test]
    fn test_orchestration_result_new() {
        let result = OrchestrationResult::new("plan-1");
        assert_eq!(result.plan_id, "plan-1");
        assert_eq!(result.status, OrchestrationStatus::Pending);
        assert_eq!(result.successful_tasks, 0);
    }

    #[test]
    fn test_orchestration_result_finalize() {
        let result = OrchestrationResult::new("plan-1")
            .add_task_result(
                "task-1",
                TaskResult::success("task-1", "agent-1", "output", 100),
            )
            .finalize();

        assert_eq!(result.status, OrchestrationStatus::Completed);
        assert!(result.is_complete());
    }

    #[test]
    fn test_task_result_success() {
        let result = TaskResult::success("task-1", "agent-1", "output", 100);
        assert!(result.success);
        assert_eq!(result.output, Some("output".to_string()));
        assert_eq!(result.execution_time_ms, 100);
    }

    #[test]
    fn test_task_result_failure() {
        let result = TaskResult::failure("task-1", "agent-1", "error", 100);
        assert!(!result.success);
        assert_eq!(result.error, Some("error".to_string()));
    }
}
