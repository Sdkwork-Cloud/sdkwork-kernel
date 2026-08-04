use sdkwork_agent_kernel::{AgentSession, KernelResult, SessionKind, SessionSource, SessionState};
use sdkwork_agent_provider_core::{
    create_session_from_config, finalize_provider_session_snapshot, SessionAdapter, SessionConfig,
};

/// Provider-native Rig execution state projected into the kernel session model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigSessionSnapshot {
    pub execution_id: String,
    pub parent_execution_id: Option<String>,
    pub agent_id: Option<String>,
    pub user_ref: Option<String>,
    pub tenant_id: Option<String>,
    pub title: Option<String>,
    pub preview: Option<String>,
    pub summary: Option<String>,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub cwd: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub state: SessionState,
    /// Execution configuration preserved verbatim so downstream turn
    /// execution can re-apply the same policies without data loss.
    pub instructions: Option<String>,
    pub personality: Option<String>,
    pub reasoning_effort: Option<String>,
    pub approval_policy: Option<String>,
    pub permission_profile: Option<String>,
    pub timeout_ms: Option<u64>,
}

pub struct RigSessionAdapter;

impl RigSessionAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RigSessionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionAdapter for RigSessionAdapter {
    type ExternalSession = RigSessionSnapshot;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        let kind = if external.parent_execution_id.is_some() {
            SessionKind::Subagent
        } else {
            SessionKind::Main
        };
        let mut config = SessionConfig::new()
            .with_source(SessionSource::Api)
            .with_kind(kind)
            .with_metadata("rig.session_kind", "rig_execution");
        if let Some(title) = external.title.as_deref() {
            config = config.with_title(title);
        }
        if let Some(model) = external.model.as_deref() {
            config = config.with_model(model);
        }
        if let Some(provider) = external.model_provider.as_deref() {
            config = config.with_model_provider(provider);
        }
        if let Some(cwd) = external.cwd.as_deref() {
            config = config.with_cwd(cwd);
        }
        if let Some(instructions) = external.instructions.as_deref() {
            config = config.with_instructions(instructions);
        }
        if let Some(personality) = external.personality.as_deref() {
            config = config.with_personality(personality);
        }
        if let Some(reasoning_effort) = external.reasoning_effort.as_deref() {
            config = config.with_reasoning_effort(reasoning_effort);
        }
        if let Some(approval_policy) = external.approval_policy.as_deref() {
            config = config.with_approval_policy(approval_policy);
        }
        if let Some(permission_profile) = external.permission_profile.as_deref() {
            config = config.with_permission_profile(permission_profile);
        }
        if let Some(timeout_ms) = external.timeout_ms {
            config = config.with_timeout_ms(timeout_ms);
        }

        let mut session = create_session_from_config(
            &external.execution_id,
            external.agent_id.clone(),
            external.user_ref.clone(),
            external.tenant_id.clone(),
            config,
            external.created_at.as_deref().unwrap_or(""),
        );
        session.parent_session_id = external.parent_execution_id.clone();
        session.preview = external.preview.clone();
        session.summary = external.summary.clone();
        session.updated_at = external.updated_at.clone();
        session.state = external.state;
        finalize_provider_session_snapshot("rig", session)
    }
}

sdkwork_agent_provider_core::define_provider_lifecycle_provider!(RigLifecycleProvider, "rig");

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_provider_core::SessionLifecycleProvider;

    #[test]
    fn maps_rig_execution_to_unified_session() {
        let snapshot = RigSessionSnapshot {
            execution_id: "rig.execution.1".to_string(),
            parent_execution_id: Some("rig.execution.parent".to_string()),
            agent_id: Some("agent.rig".to_string()),
            user_ref: Some("user.1".to_string()),
            tenant_id: Some("tenant.1".to_string()),
            title: Some("Rig task".to_string()),
            preview: Some("executing the task".to_string()),
            summary: Some("task summary".to_string()),
            model: Some("gpt-5".to_string()),
            model_provider: Some("openai".to_string()),
            cwd: Some("/workspace".to_string()),
            created_at: Some("2026-07-15T00:00:00Z".to_string()),
            updated_at: Some("2026-07-15T00:01:00Z".to_string()),
            state: SessionState::Working,
            instructions: Some("follow the runbook".to_string()),
            personality: Some("concise".to_string()),
            reasoning_effort: Some("medium".to_string()),
            approval_policy: Some("never".to_string()),
            permission_profile: Some("readonly".to_string()),
            timeout_ms: Some(120_000),
        };

        let session = RigSessionAdapter::new()
            .to_agent_session(&snapshot)
            .expect("mapped");
        assert_eq!(session.session_id, "rig.execution.1");
        assert_eq!(session.kind, SessionKind::Subagent);
        assert_eq!(session.state, SessionState::Working);
        assert_eq!(
            session.parent_session_id.as_deref(),
            Some("rig.execution.parent")
        );
        assert_eq!(session.preview.as_deref(), Some("executing the task"));
        assert_eq!(session.summary.as_deref(), Some("task summary"));
        assert_eq!(session.instructions.as_deref(), Some("follow the runbook"));
        assert_eq!(session.personality.as_deref(), Some("concise"));
        assert_eq!(session.reasoning_effort.as_deref(), Some("medium"));
        assert_eq!(session.approval_policy.as_deref(), Some("never"));
        assert_eq!(session.permission_profile.as_deref(), Some("readonly"));
        assert_eq!(session.timeout_ms, Some(120_000));
        assert!(session
            .metadata
            .iter()
            .any(|(key, value)| key == "rig.session_kind" && value == "rig_execution"));
    }

    #[test]
    fn lifecycle_supports_crud_and_incremental_sync() {
        let provider = RigLifecycleProvider::new();
        let created = provider
            .create_session("agent.rig", None, SessionConfig::new())
            .expect("created");
        assert_eq!(
            provider
                .get_session(&created.session_id)
                .unwrap()
                .session_id,
            created.session_id
        );

        let mut updated = created.clone();
        updated.title = Some("updated".to_string());
        provider.update_session(updated).expect("updated");
        provider.close_session(&created.session_id).expect("closed");
        provider
            .delete_session(&created.session_id)
            .expect("deleted");

        let changes = provider.session_changes(0, Some(20)).expect("changes");
        assert_eq!(changes.changes.len(), 4);
        assert!(changes
            .changes
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence));
    }
}
