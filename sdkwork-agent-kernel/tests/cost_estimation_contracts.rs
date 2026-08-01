//! Contract tests for cost estimation and usage detail accounting.
//!
//! `ModelUsage` carries cached/reasoning tokens and duration; the
//! `ModelCostCalculator` derives integer-cent costs from a per-model price
//! table, and streaming execution surfaces `CostEvent` before the terminal
//! result when prices are registered.

use sdkwork_agent_kernel::{
    AgentExecutionRequest, AgentExecutionService, AgentManifest, AgentStreamEvent, AgentStreamSink,
    InMemoryAgentStreamSink, KernelResult, ModelPrice, ModelProvider, ModelRequest, ModelResponse,
    ModelUsage, ProviderHealth, ProviderManifest, RuntimeBuilder,
};

const COST_AGENT_MANIFEST_JSON: &str = r#"
{
  "schema_version": "0.1.0",
  "manifest_type": "agent",
  "agent_id": "agent.intelligence.cost",
  "name": "sdkwork-cost-agent",
  "display_name": "SDKWork Cost Agent",
  "description": "Agent used to prove cost accounting contracts.",
  "version": "0.1.0",
  "domain": "intelligence",
  "required_capabilities": [
    {
      "capability_id": "model.chat",
      "min_version": "0.1.0"
    },
    {
      "capability_id": "policy.evaluate",
      "min_version": "0.1.0"
    }
  ],
  "optional_capabilities": [],
  "event_families": ["agent.runtime.*", "agent.stream.*"],
  "owner": {
    "name": "sdkwork-platform"
  },
  "status": "candidate"
}
"#;

#[derive(Clone)]
struct CostModelProvider {
    provider_id: String,
}

impl ModelProvider for CostModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "cost-model",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            "costed response",
        )
        .with_model_id("claude-sonnet-4")
        .with_usage(
            ModelUsage::new(100_000, 10_000)
                .with_cached_input_tokens(900_000)
                .with_reasoning_tokens(2_000)
                .with_duration_ms(1_234),
        ))
    }
}

#[derive(Clone)]
struct AllowPolicyProvider;

impl sdkwork_agent_kernel::PolicyProvider for AllowPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.policy.cost",
            "policy",
            "allow-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn evaluate(
        &self,
        request: sdkwork_agent_kernel::PolicyRequest,
    ) -> KernelResult<sdkwork_agent_kernel::PolicyDecision> {
        Ok(sdkwork_agent_kernel::PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            "provider.policy.cost",
        ))
    }
}

fn cost_runtime() -> sdkwork_agent_kernel::AgentRuntime {
    RuntimeBuilder::new(
        "runtime.cost",
        AgentManifest::from_json(COST_AGENT_MANIFEST_JSON).expect("cost manifest parses"),
    )
    .with_generated_at("2026-08-01T00:00:00Z")
    .with_model_prices(vec![
        ModelPrice::new("claude-sonnet-4", 3.0, 15.0, "USD").with_cached_input_per_1m(0.3)
    ])
    .register_model_provider(
        "provider.model.cost",
        "0.1.0",
        CostModelProvider {
            provider_id: "provider.model.cost".to_string(),
        },
    )
    .register_policy_provider("provider.policy.cost", "0.1.0", AllowPolicyProvider)
    .bootstrap()
    .expect("cost runtime bootstraps")
    .runtime
}

#[test]
fn model_usage_carries_detail_accounting_fields() {
    let usage = ModelUsage::new(10, 20)
        .with_cached_input_tokens(5)
        .with_reasoning_tokens(3)
        .with_duration_ms(250);

    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 20);
    assert_eq!(usage.cached_input_tokens, 5);
    assert_eq!(usage.reasoning_tokens, 3);
    assert_eq!(usage.duration_ms, Some(250));
    assert_eq!(usage.total_tokens(), 30);
}

#[test]
fn cost_estimate_derives_integer_cents() {
    let calculator = sdkwork_agent_kernel::ModelCostCalculator::new(vec![ModelPrice::new(
        "model.a", 2.0, 8.0, "USD",
    )]);

    let estimate = calculator
        .estimate("model.a", &ModelUsage::new(1_000_000, 500_000))
        .expect("priced model estimates");

    assert_eq!(estimate.input_cost_cents, 200);
    assert_eq!(estimate.output_cost_cents, 400);
    assert_eq!(estimate.cost_cents, 600);
    assert_eq!(estimate.currency, "USD");
}

#[test]
fn cost_estimate_applies_cached_discount() {
    let calculator = sdkwork_agent_kernel::ModelCostCalculator::new(vec![ModelPrice::new(
        "model.b", 3.0, 15.0, "USD",
    )
    .with_cached_input_per_1m(0.3)]);

    let usage = ModelUsage::new(100_000, 10_000).with_cached_input_tokens(900_000);
    let estimate = calculator.estimate("model.b", &usage).unwrap();

    assert_eq!(estimate.cached_input_cost_cents, 27);
    assert_eq!(estimate.cost_cents, 30 + 27 + 15);
}

#[test]
fn cost_estimate_returns_none_for_unpriced_models() {
    let calculator = sdkwork_agent_kernel::ModelCostCalculator::new(vec![]);
    assert!(calculator
        .estimate("model.unknown", &ModelUsage::new(1, 1))
        .is_none());
}

#[test]
fn price_registration_is_idempotent() {
    let calculator = sdkwork_agent_kernel::ModelCostCalculator::default()
        .register(ModelPrice::new("model.a", 1.0, 2.0, "CNY"))
        .register(ModelPrice::new("model.a", 9.0, 9.0, "USD"));

    let price = calculator.price_for("model.a").expect("first price wins");
    assert_eq!(price.currency, "CNY");
    assert_eq!(price.input_per_1m, 1.0);
}

#[test]
fn cost_event_round_trips_from_estimate() {
    let calculator = sdkwork_agent_kernel::ModelCostCalculator::new(vec![ModelPrice::new(
        "model.c", 1.0, 2.0, "CNY",
    )]);
    let estimate = calculator
        .estimate("model.c", &ModelUsage::new(100_000, 100_000))
        .unwrap();
    let event = estimate.to_cost_event("cost.1");

    assert_eq!(event.event_id, "cost.1");
    assert_eq!(event.cost_cents, 30);
    assert_eq!(event.currency, "CNY");
}

#[test]
fn streaming_execution_emits_cost_event_from_price_table() {
    let runtime = cost_runtime();
    let mut sink = InMemoryAgentStreamSink::new();

    AgentExecutionService::new()
        .execute_streaming(
            &runtime,
            AgentExecutionRequest::new("exec.cost.1", vec!["hello".to_string()])
                .for_session("session.cost"),
            &mut sink,
        )
        .expect("cost execution stream succeeds");

    let types: Vec<&str> = sink.events().iter().map(|e| e.event_type()).collect();
    assert!(types.contains(&"agent.stream.cost"));
    assert!(types.contains(&"agent.stream.usage"));
    assert!(types.contains(&"agent.stream.result"));

    let cost = sink
        .events()
        .iter()
        .find_map(|event| match event {
            AgentStreamEvent::Cost(cost) => Some(cost),
            _ => None,
        })
        .expect("cost event present");

    // 100k input @3.00/1M = 30c; 900k cached @0.30/1M = 27c; 10k output
    // @15.00/1M = 15c; total 72c.
    assert_eq!(cost.cost_cents, 72);
    assert_eq!(cost.currency, "USD");

    // Usage carries the detail accounting fields.
    let usage = sink
        .events()
        .iter()
        .find_map(|event| match event {
            AgentStreamEvent::Usage(usage) => Some(usage),
            _ => None,
        })
        .expect("usage event present");
    assert_eq!(usage.cached_input_tokens, 900_000);
    assert_eq!(usage.reasoning_tokens, 2_000);
}
