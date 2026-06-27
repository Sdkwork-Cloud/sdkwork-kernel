use sdkwork_agent_kernel::{
    PolicyDecision, PolicyProvider, PolicyRequest, ProviderHealth, ProviderManifest,
    SideEffectLevel,
};

/// Fail-closed policy provider shared by external SDK runtime plugins.
#[derive(Debug, Clone, Default)]
pub struct SdkStandardPolicyProvider {
    provider_id: String,
}

impl SdkStandardPolicyProvider {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
        }
    }

    pub fn provider_manifest_for(&self) -> ProviderManifest {
        ProviderManifest::new(
            &self.provider_id,
            "policy",
            "sdk-standard-fail-closed-policy",
            "0.1.0",
            vec!["policy.evaluate".to_string()],
        )
    }
}

impl PolicyProvider for SdkStandardPolicyProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        self.provider_manifest_for()
    }

    fn evaluate(
        &self,
        request: PolicyRequest,
    ) -> sdkwork_agent_kernel::KernelResult<PolicyDecision> {
        if requires_local_approval(&request) {
            return Ok(PolicyDecision::needs_approval(
                format!("decision.{}", request.policy_request_id),
                request.policy_request_id,
                self.provider_id.clone(),
                "sdk.standard.approval_required",
            )
            .with_safe_reason("SDK standard policy requires approval for side-effectful actions")
            .require_audit());
        }

        Ok(PolicyDecision::allow(
            format!("decision.{}", request.policy_request_id),
            request.policy_request_id,
            self.provider_id.clone(),
        ))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

fn requires_local_approval(request: &PolicyRequest) -> bool {
    if matches!(
        request.side_effect_level,
        Some(
            SideEffectLevel::SideEffectful
                | SideEffectLevel::Destructive
                | SideEffectLevel::Privileged
                | SideEffectLevel::ExternalSend
        )
    ) {
        return true;
    }

    matches!(
        request.category.as_str(),
        "model.send_sensitive_context"
            | "tool.invoke"
            | "tool.external_send"
            | "host.secrets.read"
            | "host.filesystem.write"
            | "host.process.execute"
            | "host.network.connect"
            | "memory.write"
            | "memory.delete"
            | "protocol.send"
            | "skill.invoke"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_kernel::PolicyCategory;

    #[test]
    fn side_effectful_tool_requires_approval() {
        let provider = SdkStandardPolicyProvider::new("provider.policy.test");
        let request = PolicyRequest::new("req-1", PolicyCategory::ToolInvoke.as_str(), "tool.test")
            .with_side_effect_level(SideEffectLevel::SideEffectful);
        let decision = provider.evaluate(request).expect("evaluate");
        assert!(decision.is_needs_approval());
    }
}
