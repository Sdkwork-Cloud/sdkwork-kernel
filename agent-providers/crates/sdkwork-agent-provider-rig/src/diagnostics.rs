use crate::backend::{
    RigBackendBootstrapState, RigBackendConfig, RigBackendExecutionStatus, RigBackendMode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigBackendBootstrapReadiness {
    pub backend_mode: RigBackendMode,
    pub state: RigBackendBootstrapState,
    pub provider_id: Option<String>,
    pub required_secret_refs: Vec<String>,
    pub missing_secret_refs: Vec<String>,
    pub policy_categories: Vec<String>,
    pub fail_closed: bool,
    pub safe_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigPluginDiagnostics {
    pub backend_mode: RigBackendMode,
    pub live_backend_configured: bool,
    pub fail_closed: bool,
    pub missing_secret_refs: Vec<String>,
}

impl RigPluginDiagnostics {
    pub fn fail_closed() -> Self {
        Self {
            backend_mode: RigBackendMode::FailClosed,
            live_backend_configured: false,
            fail_closed: true,
            missing_secret_refs: Vec::new(),
        }
    }

    pub fn from_backend_config(config: &RigBackendConfig) -> Self {
        Self {
            backend_mode: config.mode,
            live_backend_configured: config.mode == RigBackendMode::Live,
            fail_closed: config.execution_status().fail_closed,
            missing_secret_refs: missing_secret_refs(config),
        }
    }

    pub fn backend_execution_status(&self) -> RigBackendExecutionStatus {
        RigBackendConfig {
            mode: self.backend_mode,
            provider_id: None,
            api_key_secret_ref: None,
            base_url: None,
        }
        .execution_status()
    }

    pub fn backend_bootstrap_readiness_from_config(
        config: &RigBackendConfig,
    ) -> RigBackendBootstrapReadiness {
        Self::readiness_from_config(config)
    }

    pub fn backend_bootstrap_readiness(&self) -> RigBackendBootstrapReadiness {
        Self::readiness_from_config(&RigBackendConfig {
            mode: self.backend_mode,
            provider_id: None,
            api_key_secret_ref: None,
            base_url: None,
        })
        .with_missing_secret_refs(self.missing_secret_refs.clone())
    }

    fn readiness_from_config(config: &RigBackendConfig) -> RigBackendBootstrapReadiness {
        let bootstrap_plan = config.bootstrap_plan();

        RigBackendBootstrapReadiness {
            backend_mode: bootstrap_plan.backend_mode,
            state: bootstrap_plan.state,
            provider_id: bootstrap_plan.provider_id,
            required_secret_refs: bootstrap_plan.required_secret_refs,
            missing_secret_refs: missing_secret_refs(config),
            policy_categories: bootstrap_plan.policy_categories,
            fail_closed: bootstrap_plan.fail_closed,
            safe_summary: bootstrap_plan.safe_summary,
        }
    }
}

impl RigBackendBootstrapReadiness {
    fn with_missing_secret_refs(mut self, missing_secret_refs: Vec<String>) -> Self {
        self.missing_secret_refs = missing_secret_refs;
        self
    }
}

fn missing_secret_refs(_config: &RigBackendConfig) -> Vec<String> {
    // Required secret refs are declared only when the configuration actually
    // binds them (`llm.rig.api_key` for API-key-backed executors); the default
    // cloud router dual-token mode requires no local secret, so a live
    // backend without a bound key is not "missing" anything.
    Vec::new()
}
