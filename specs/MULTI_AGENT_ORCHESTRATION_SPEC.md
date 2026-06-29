# SDKWork Multi-Agent Orchestration Specification

- **Version**: 0.1.0
- **Status**: Core Primitives Implemented
- **Date**: 2025-06-28
- **Scope**: Multi-agent task orchestration and coordination
- **Domain**: `intelligence`
- **Capability**: `agent-kernel.multi-agent-orchestration`
- **Implementation**: `sdkwork-agent-kernel/src/orchestration.rs`
- **Test Coverage**: 15/15 tests passing (100%)

## 1. Overview

Multi-Agent Orchestration provides coordination primitives for executing tasks across multiple agents with:

- **Task Decomposition**: Break complex tasks into subtasks
- **Dependency Management**: Handle task dependencies and ordering
- **Parallel Execution**: Execute independent tasks concurrently
- **Result Aggregation**: Combine results from multiple agents
- **Workflow Management**: Sequential/parallel/priority/dependency strategies

### Key Features

1. **AgentTask**: Individual task with dependencies, priority, timeout
2. **AgentGraph**: Agent relationship graph with topological ordering
3. **OrchestrationPlan**: Complete execution plan with strategy
4. **OrchestrationResult**: Execution result with aggregation

## 2. Architecture

### Component Structure

```text
AgentTask
  ├── task_id: String
  ├── agent_id: String
  ├── objective: String
  ├── inputs: HashMap<String, String>
  ├── expected_outputs: Vec<String>
  ├── dependencies: Vec<String>
  ├── timeout_ms: Option<u64>
  ├── priority: u8
  └── retry_count: u32

AgentGraph
  ├── agents: HashMap<String, AgentNode>
  ├── edges: HashMap<String, Vec<String>>
  └── topological_sort() -> Vec<String>

AgentNode
  ├── agent_id: String
  ├── capabilities: Vec<String>
  ├── max_concurrent_tasks: usize
  └── current_load: usize

OrchestrationPlan
  ├── plan_id: String
  ├── tasks: Vec<AgentTask>
  ├── strategy: ExecutionStrategy
  └── aggregation: AggregationStrategy

OrchestrationResult
  ├── plan_id: String
  ├── status: OrchestrationStatus
  ├── task_results: HashMap<String, TaskResult>
  ├── aggregated_result: Option<String>
  └── metrics (time, success/fail counts)
```

## 3. Agent Task

### Definition

```rust
pub struct AgentTask {
    pub task_id: String,
    pub agent_id: String,
    pub objective: String,
    pub inputs: HashMap<String, String>,
    pub expected_outputs: Vec<String>,
    pub dependencies: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub priority: u8,
    pub retry_count: u32,
}
```

### Creation

```rust
let task = AgentTask::new("task-1", "code-generator", "Generate Python code")
    .with_input("spec", "API spec document")
    .with_input("language", "python")
    .with_expected_output("code_file")
    .with_timeout(60000) // 60 seconds
    .with_priority(80) // High priority
    .with_retry(3); // Retry 3 times on failure
```

### Dependencies

```rust
let analysis_task = AgentTask::new("analysis", "analyzer", "Analyze requirements");

let generation_task = AgentTask::new("generation", "code-generator", "Generate code")
    .with_dependency("analysis"); // Must wait for analysis

let review_task = AgentTask::new("review", "code-reviewer", "Review generated code")
    .with_dependency("generation"); // Must wait for generation
```

### Dependency Checking

```rust
// Check if task is ready to execute
let completed_tasks = vec!["analysis".to_string()];
if task.is_ready(&completed_tasks) {
    // Execute task
}
```

## 4. Agent Graph

### Definition

```rust
pub struct AgentGraph {
    pub agents: HashMap<String, AgentNode>,
    pub edges: HashMap<String, Vec<String>>,
}
```

### Agent Node

```rust
pub struct AgentNode {
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub max_concurrent_tasks: usize,
    pub current_load: usize,
}
```

### Graph Construction

```rust
let graph = AgentGraph::new()
    .add_agent("analyzer", AgentNode::new("analyzer")
        .with_capability("code-analysis")
        .with_max_concurrent(2))
    .add_agent("generator", AgentNode::new("generator")
        .with_capability("code-generation")
        .with_max_concurrent(1))
    .add_agent("reviewer", AgentNode::new("reviewer")
        .with_capability("code-review")
        .with_max_concurrent(3))
    .add_edge("analyzer", "generator") // analyzer -> generator
    .add_edge("generator", "reviewer"); // generator -> reviewer
```

### Topological Sort

```rust
let sorted = graph.topological_sort();
// Returns: ["analyzer", "generator", "reviewer"]
// (order respects dependencies)
```

### Load Management

```rust
let mut node = graph.get_agent("generator").unwrap().clone();
assert!(node.can_accept_task());

node.increment_load();
node.increment_load();
assert!(!node.can_accept_task()); // At capacity

node.decrement_load();
assert!(node.can_accept_task());
```

## 5. Orchestration Plan

### Definition

```rust
pub struct OrchestrationPlan {
    pub plan_id: String,
    pub tasks: Vec<AgentTask>,
    pub strategy: ExecutionStrategy,
    pub aggregation: AggregationStrategy,
}
```

### Plan Creation

```rust
let plan = OrchestrationPlan::new("code-generation-plan")
    .with_task(AgentTask::new("analysis", "analyzer", "Analyze requirements"))
    .with_task(AgentTask::new("generation", "generator", "Generate code")
        .with_dependency("analysis"))
    .with_task(AgentTask::new("review", "reviewer", "Review code")
        .with_dependency("generation"))
    .with_strategy(ExecutionStrategy::Dependency)
    .with_aggregation(AggregationStrategy::Merge);
```

### Ready Task Detection

```rust
let completed_tasks: Vec<String> = vec!["analysis".to_string()];
let ready_tasks = plan.get_ready_tasks(&completed_tasks);
// Returns tasks with all dependencies satisfied
```

## 6. Execution Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| `Sequential` | Execute tasks one by one | Simple linear workflows |
| `Parallel` | Execute all tasks concurrently | Independent tasks |
| `Priority` | Execute by priority order | Prioritized workloads |
| `Dependency` | Execute based on dependencies | Complex workflows |

### Example: Parallel Execution

```rust
let plan = OrchestrationPlan::new("parallel-analysis")
    .with_task(AgentTask::new("analyze-1", "analyzer", "Analyze module A"))
    .with_task(AgentTask::new("analyze-2", "analyzer", "Analyze module B"))
    .with_task(AgentTask::new("analyze-3", "analyzer", "Analyze module C"))
    .with_strategy(ExecutionStrategy::Parallel);
```

### Example: Dependency Execution

```rust
let plan = OrchestrationPlan::new("pipeline")
    .with_task(AgentTask::new("fetch", "fetcher", "Fetch data"))
    .with_task(AgentTask::new("process", "processor", "Process data")
        .with_dependency("fetch"))
    .with_task(AgentTask::new("store", "storer", "Store results")
        .with_dependency("process"))
    .with_strategy(ExecutionStrategy::Dependency);
```

## 7. Aggregation Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| `Merge` | Merge all results | Combine outputs |
| `AggregateByType` | Group by result type | Categorized results |
| `SelectBest` | Select best result | Quality-based selection |
| `All` | Return all results separately | Complete history |

### Example: Select Best

```rust
let plan = OrchestrationPlan::new("best-solution")
    .with_task(AgentTask::new("solver-1", "solver", "Solve with approach A"))
    .with_task(AgentTask::new("solver-2", "solver", "Solve with approach B"))
    .with_task(AgentTask::new("solver-3", "solver", "Solve with approach C"))
    .with_strategy(ExecutionStrategy::Parallel)
    .with_aggregation(AggregationStrategy::SelectBest);
```

## 8. Orchestration Result

### Definition

```rust
pub struct OrchestrationResult {
    pub plan_id: String,
    pub status: OrchestrationStatus,
    pub task_results: HashMap<String, TaskResult>,
    pub aggregated_result: Option<String>,
    pub total_time_ms: u64,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
}
```

### Task Result

```rust
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub retry_attempts: u32,
}
```

### Result Construction

```rust
let result = OrchestrationResult::new("plan-1")
    .add_task_result("task-1", TaskResult::success(
        "task-1", "agent-1", "output", 100
    ))
    .add_task_result("task-2", TaskResult::success(
        "task-2", "agent-1", "output", 150
    ))
    .with_aggregated_result("Combined output")
    .with_total_time(250)
    .finalize();

assert_eq!(result.status, OrchestrationStatus::Completed);
assert!(result.is_complete());
```

### Partial Success

```rust
let result = OrchestrationResult::new("plan-1")
    .add_task_result("task-1", TaskResult::success(
        "task-1", "agent-1", "output", 100
    ))
    .add_task_result("task-2", TaskResult::failure(
        "task-2", "agent-1", "error", 50
    ))
    .finalize();

assert_eq!(result.status, OrchestrationStatus::PartialSuccess);
assert!(result.is_complete());
```

## 9. Orchestration Status

| Status | Description |
|--------|-------------|
| `Pending` | Orchestration not yet started |
| `InProgress` | Orchestration executing |
| `Completed` | All tasks succeeded |
| `PartialSuccess` | Some tasks succeeded, some failed |
| `Failed` | All tasks failed |
| `Cancelled` | Orchestration cancelled |

## 10. Conformance Tests

### Test Coverage (15 tests)

| Test Name | Coverage |
|-----------|----------|
| `test_agent_task_new` | Task creation |
| `test_agent_task_with_dependencies` | Dependencies |
| `test_agent_task_priority` | Priority clamping |
| `test_agent_graph_new` | Graph creation |
| `test_agent_graph_add_agent` | Agent addition |
| `test_agent_graph_dependencies` | Edge dependencies |
| `test_agent_graph_topological_sort` | Topological ordering |
| `test_agent_node_capacity` | Load management |
| `test_orchestration_plan_new` | Plan creation |
| `test_orchestration_plan_ready_tasks` | Ready task detection |
| `test_execution_strategy_as_str` | Strategy strings |
| `test_orchestration_result_new` | Result creation |
| `test_orchestration_result_finalize` | Result finalization |
| `test_task_result_success` | Success result |
| `test_task_result_failure` | Failure result |

### Test Execution

```bash
cargo test --package sdkwork-agent-kernel --lib orchestration::tests
```

### Expected Result

```
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured
```

## 11. Integration Points

### AgentCollaborationProvider Extension

```rust
pub trait AgentCollaborationProvider {
    // Existing methods
    fn list_agents(&self) -> Vec<AgentCard>;
    fn handoff(&self, request: AgentHandoffRequest) -> KernelResult<AgentHandoffResult>;
    fn delegate(&self, request: AgentDelegationRequest) -> KernelResult<AgentDelegationResult>;

    // New orchestration methods (extension)
    fn orchestrate(&self, plan: OrchestrationPlan) -> KernelResult<OrchestrationResult>;
    fn build_graph(&self) -> AgentGraph;
}
```

### PlanningProvider Integration

```rust
pub trait PlanningProvider {
    // New method: decompose task into orchestration plan
    fn decompose_to_plan(
        &self,
        task: PlanningTask,
        agents: &[AgentCard],
    ) -> KernelResult<OrchestrationPlan>;
}
```

### TelemetryProvider Integration

```rust
// Record orchestration metrics
telemetry.counter("orchestration.executed", 1, &[
    ("plan_id", plan.plan_id),
    ("strategy", plan.strategy.as_str()),
    ("status", result.status.as_str()),
]);

telemetry.histogram("orchestration.total_time", result.total_time_ms);
telemetry.histogram("orchestration.success_rate", 
    result.successful_tasks as f64 / plan.tasks.len() as f64);
```

### PolicyProvider Integration

```rust
pub trait PolicyProvider {
    // Check if orchestration is allowed
    fn approve_orchestration(
        &self,
        plan: &OrchestrationPlan,
    ) -> KernelResult<bool>;
}
```

## 12. Usage Patterns

### Pattern 1: Sequential Pipeline

```rust
let plan = OrchestrationPlan::new("pipeline")
    .with_task(AgentTask::new("step-1", "agent-A", "Extract data"))
    .with_task(AgentTask::new("step-2", "agent-B", "Transform data")
        .with_dependency("step-1"))
    .with_task(AgentTask::new("step-3", "agent-C", "Load data")
        .with_dependency("step-2"))
    .with_strategy(ExecutionStrategy::Dependency);

let result = collaboration_provider.orchestrate(plan)?;
```

### Pattern 2: Parallel Analysis

```rust
let plan = OrchestrationPlan::new("parallel-analysis")
    .with_task(AgentTask::new("analyze-1", "analyzer", "Analyze file A"))
    .with_task(AgentTask::new("analyze-2", "analyzer", "Analyze file B"))
    .with_task(AgentTask::new("analyze-3", "analyzer", "Analyze file C"))
    .with_strategy(ExecutionStrategy::Parallel)
    .with_aggregation(AggregationStrategy::Merge);

let result = collaboration_provider.orchestrate(plan)?;
```

### Pattern 3: Best Solution Selection

```rust
let plan = OrchestrationPlan::new("best-approach")
    .with_task(AgentTask::new("approach-1", "solver", "Solve with ML"))
    .with_task(AgentTask::new("approach-2", "solver", "Solve with heuristic"))
    .with_task(AgentTask::new("approach-3", "solver", "Solve with optimization"))
    .with_strategy(ExecutionStrategy::Parallel)
    .with_aggregation(AggregationStrategy::SelectBest);

let result = collaboration_provider.orchestrate(plan)?;
let best_solution = result.aggregated_result.unwrap();
```

### Pattern 4: Priority-Based Execution

```rust
let plan = OrchestrationPlan::new("priority-work")
    .with_task(AgentTask::new("critical", "agent-A", "Critical task")
        .with_priority(100))
    .with_task(AgentTask::new("high", "agent-B", "High priority")
        .with_priority(80))
    .with_task(AgentTask::new("normal", "agent-C", "Normal priority")
        .with_priority(50))
    .with_strategy(ExecutionStrategy::Priority);

let result = collaboration_provider.orchestrate(plan)?;
```

## 13. Performance Characteristics

### Time Complexity

| Operation | Complexity |
|-----------|------------|
| Task creation | O(1) |
| Dependency check | O(d) where d = dependencies |
| Graph construction | O(n) where n = agents |
| Topological sort | O(n + e) where e = edges |
| Ready task detection | O(n × d) |

### Space Complexity

- **Per Task**: ~1KB
- **Per Graph**: O(n + e) where n = agents, e = edges
- **Per Plan**: O(t × k) where t = tasks, k = average task size

### Recommendations

- Limit tasks to <100 per plan
- Limit dependencies to <10 per task
- Use parallel execution for independent tasks
- Use dependency execution for complex workflows

## 14. Security Considerations

### Policy Approval

- All orchestrations should pass policy checks
- Policy can reject orchestration based on:
  - Agent trust levels
  - Task objectives
  - Input/output data classifications

### Data Flow

- Track data flow between agents
- Apply redaction based on data classification
- Prevent unauthorized data access

### Audit Trail

- Record all orchestration executions
- Track task start/end times
- Record success/failure status
- Link to policy decisions

## 15. Future Extensions

### Planned Extensions (Phase 6)

1. **Retry with Backoff**: Intelligent retry strategies
2. **Resource Limits**: CPU/memory limits per agent
3. **Time Budgets**: Maximum execution time per plan
4. **Conditional Execution**: Execute tasks based on conditions
5. **Dynamic Replanning**: Adapt plan during execution

### Extension Points

```rust
// Future: Conditional execution
pub struct ConditionalTask {
    pub condition: TaskCondition,
    pub if_true: AgentTask,
    pub if_false: Option<AgentTask>,
}

// Future: Dynamic replanning
pub trait DynamicOrchestrator {
    fn replan(
        &self,
        current_result: &OrchestrationResult,
        remaining_tasks: &[AgentTask],
    ) -> KernelResult<OrchestrationPlan>;
}
```

## 16. References

- `sdkwork-agent-kernel/src/orchestration.rs` - Implementation
- `sdkwork-agent-kernel/src/collaboration.rs` - AgentCollaborationProvider
- `sdkwork-agent-kernel/src/planning.rs` - PlanningProvider
- `specs/AGENT_KERNEL_SPEC.md` - Kernel specification
- `specs/AGENT_COLLABORATION_SPEC.md` - Collaboration specification

## 17. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-06-28 | Core primitives, 15/15 tests passing |

---

**Status**: ✅ Core Primitives Implemented
**Next Steps**: Integration with AgentCollaborationProvider and A2A Protocol Adapter