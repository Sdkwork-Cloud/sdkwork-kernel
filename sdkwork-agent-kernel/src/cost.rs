//! Model cost estimation: per-model price tables and usage-derived cost
//! calculation.
//!
//! The kernel stays provider-neutral: prices are supplied by the runtime
//! (catalog-driven) and costs are derived from `ModelUsage`. `CostEvent`
//! carries the result to stream consumers, aligning with the agent SDK
//! `result.total_cost_usd` conventions while keeping integer-cents
//! accounting for billing.

use crate::{CostEvent, ModelUsage};

/// Per-million-token price for a model in a single currency.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPrice {
    pub model_id: String,
    /// Price per 1M input tokens.
    pub input_per_1m: f64,
    /// Price per 1M output tokens.
    pub output_per_1m: f64,
    /// Price per 1M cached input tokens (usually a discount).
    pub cached_input_per_1m: f64,
    pub currency: String,
}

impl Eq for ModelPrice {}

impl ModelPrice {
    pub fn new(
        model_id: impl Into<String>,
        input_per_1m: f64,
        output_per_1m: f64,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            input_per_1m,
            output_per_1m,
            cached_input_per_1m: input_per_1m,
            currency: currency.into(),
        }
    }

    /// Set the cached-input discount price explicitly.
    pub fn with_cached_input_per_1m(mut self, cached_input_per_1m: f64) -> Self {
        self.cached_input_per_1m = cached_input_per_1m;
        self
    }
}

/// Cost breakdown for a usage snapshot, in integer cents plus an optional
/// USD conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimate {
    pub model_id: String,
    /// Total cost in integer cents.
    pub cost_cents: u64,
    pub currency: String,
    /// USD total when a USD conversion rate is supplied.
    pub total_cost_usd: Option<f64>,
    pub input_cost_cents: u64,
    pub output_cost_cents: u64,
    pub cached_input_cost_cents: u64,
}

impl CostEstimate {
    /// Build the kernel `CostEvent` for a stream.
    pub fn to_cost_event(&self, event_id: impl Into<String>) -> CostEvent {
        let mut event = CostEvent::new(event_id, self.cost_cents, self.currency.clone());
        if let Some(total_cost_usd) = self.total_cost_usd {
            event = event.with_total_cost_usd(total_cost_usd);
        }
        event
    }
}

/// Price table and calculator. `estimate` returns `None` when the model has
/// no registered price.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelCostCalculator {
    prices: Vec<ModelPrice>,
}

// Prices are exact configured decimals (e.g. 2.0, 0.3), so structural
// equality is meaningful for runtime snapshots.
impl Eq for ModelCostCalculator {}

impl ModelCostCalculator {
    pub fn new(prices: Vec<ModelPrice>) -> Self {
        Self { prices }
    }

    pub fn register(mut self, price: ModelPrice) -> Self {
        if !self.prices.iter().any(|p| p.model_id == price.model_id) {
            self.prices.push(price);
        }
        self
    }

    pub fn price_for(&self, model_id: &str) -> Option<&ModelPrice> {
        self.prices.iter().find(|price| price.model_id == model_id)
    }

    pub fn has_price_for(&self, model_id: &str) -> bool {
        self.price_for(model_id).is_some()
    }

    /// Derive the cost for a usage snapshot. Cents are computed with
    /// half-up rounding on the fractional cent.
    pub fn estimate(&self, model_id: &str, usage: &ModelUsage) -> Option<CostEstimate> {
        let price = self.price_for(model_id)?;
        let input_cost = cost_cents(usage.input_tokens, price.input_per_1m);
        let cached_input_cost = cost_cents(usage.cached_input_tokens, price.cached_input_per_1m);
        let output_cost = cost_cents(usage.output_tokens, price.output_per_1m);
        Some(CostEstimate {
            model_id: model_id.to_string(),
            cost_cents: input_cost + cached_input_cost + output_cost,
            currency: price.currency.clone(),
            total_cost_usd: None,
            input_cost_cents: input_cost,
            output_cost_cents: output_cost,
            cached_input_cost_cents: cached_input_cost,
        })
    }
}

/// Convert a token count at a per-1M price into integer cents (half-up).
fn cost_cents(tokens: u32, per_1m: f64) -> u64 {
    let cents = (tokens as f64 / 1_000_000.0) * per_1m * 100.0;
    cents.round().max(0.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_computes_integer_cents() {
        let calculator =
            ModelCostCalculator::new(vec![ModelPrice::new("gpt-4.1", 2.0, 8.0, "USD")]);
        let usage = ModelUsage::new(1_000_000, 500_000);
        let estimate = calculator
            .estimate("gpt-4.1", &usage)
            .expect("priced model estimates");
        assert_eq!(estimate.input_cost_cents, 200);
        assert_eq!(estimate.output_cost_cents, 400);
        assert_eq!(estimate.cost_cents, 600);
    }

    #[test]
    fn estimate_applies_cached_discount() {
        let calculator =
            ModelCostCalculator::new(vec![
                ModelPrice::new("claude-sonnet", 3.0, 15.0, "USD").with_cached_input_per_1m(0.3)
            ]);
        let usage = ModelUsage::new(100_000, 10_000).with_cached_input_tokens(900_000);
        let estimate = calculator.estimate("claude-sonnet", &usage).unwrap();
        assert_eq!(estimate.cached_input_cost_cents, 27);
        assert_eq!(estimate.cost_cents, 30 + 15 + 27);
    }

    #[test]
    fn estimate_returns_none_for_unpriced_models() {
        let calculator = ModelCostCalculator::new(vec![]);
        assert!(calculator
            .estimate("unknown-model", &ModelUsage::new(1, 1))
            .is_none());
    }

    #[test]
    fn cost_event_round_trip() {
        let calculator = ModelCostCalculator::new(vec![ModelPrice::new("m", 1.0, 2.0, "CNY")]);
        let estimate = calculator
            .estimate("m", &ModelUsage::new(1000, 1000))
            .unwrap();
        let event = estimate.to_cost_event("cost.1");
        assert_eq!(event.cost_cents, estimate.cost_cents);
        assert_eq!(event.currency, "CNY");
    }
}
