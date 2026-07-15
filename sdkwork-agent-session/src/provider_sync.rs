use crate::UnifiedSessionManager;
use sdkwork_agent_database::{
    AgentDatabase, EventRepository, MessageRepository, RuntimeSessionWrites, SessionRepository,
    TaskRepository,
};
use sdkwork_agent_provider_core::{
    ProviderSessionChange, ProviderSessionChangeKind, SessionLifecycleProvider,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProviderSessionSyncReport {
    pub next_cursor: u64,
    pub synchronized: usize,
    pub deleted: usize,
    pub has_more: bool,
}

/// Applies one bounded provider change page to the unified transient session store.
pub struct ProviderSessionSynchronizer;

impl ProviderSessionSynchronizer {
    pub fn synchronize_once<DB, P>(
        manager: &UnifiedSessionManager<DB>,
        provider_id: &str,
        bridge_id: Option<&str>,
        provider: &P,
        after_sequence: u64,
        limit: Option<usize>,
    ) -> Result<ProviderSessionSyncReport, String>
    where
        DB: AgentDatabase
            + SessionRepository
            + MessageRepository
            + TaskRepository
            + EventRepository
            + RuntimeSessionWrites
            + Clone,
        P: SessionLifecycleProvider + ?Sized,
    {
        let batch = provider
            .session_changes(after_sequence, limit)
            .map_err(|error| format!("failed to load provider session changes: {error}"))?;

        // Only the latest mutation for each session matters in one provider snapshot.
        let mut latest_by_session: HashMap<String, ProviderSessionChange> = HashMap::new();
        for change in batch.changes {
            if change.provider_id != provider_id {
                return Err(format!(
                    "provider session change for {} belongs to {}, expected {}",
                    change.session_id, change.provider_id, provider_id
                ));
            }
            latest_by_session.insert(change.session_id.clone(), change);
        }
        let mut latest: Vec<_> = latest_by_session.into_values().collect();
        latest.sort_by_key(|change| change.sequence);

        let mut report = ProviderSessionSyncReport {
            next_cursor: batch.next_cursor,
            synchronized: 0,
            deleted: 0,
            has_more: batch.has_more,
        };
        for change in latest {
            if change.kind == ProviderSessionChangeKind::Deleted {
                if delete_provider_session(manager, provider_id, &change.session_id)? {
                    report.deleted += 1;
                }
                continue;
            }

            let session = match provider.find_session(&change.session_id) {
                Ok(Some(session)) => session,
                Ok(None) => {
                    // The provider may delete the session after the change page
                    // was read. Optional lookup distinguishes that race from a
                    // transport failure and keeps deletion ownership-safe.
                    if delete_provider_session(manager, provider_id, &change.session_id)? {
                        report.deleted += 1;
                    }
                    continue;
                }
                Err(error) => {
                    return Err(format!("failed to load provider session snapshot: {error}"))
                }
            };
            manager.synchronize_provider_session(provider_id, bridge_id, &session)?;
            report.synchronized += 1;
        }
        Ok(report)
    }
}

fn delete_provider_session<DB>(
    manager: &UnifiedSessionManager<DB>,
    provider_id: &str,
    session_id: &str,
) -> Result<bool, String>
where
    DB: AgentDatabase
        + SessionRepository
        + MessageRepository
        + TaskRepository
        + EventRepository
        + RuntimeSessionWrites
        + Clone,
{
    let Some(existing) = manager.find_session(session_id)? else {
        return Ok(false);
    };
    match existing.provider_id.as_deref() {
        Some(owner) if owner == provider_id => {
            manager.delete_session(session_id)?;
            Ok(true)
        }
        Some(owner) => Err(format!(
            "session {session_id} already belongs to provider {owner}"
        )),
        None => Err(format!(
            "session {session_id} is not owned by provider {provider_id}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_agent_database::InMemoryDatabase;
    use sdkwork_agent_kernel::{SessionKind, SessionSource};
    use sdkwork_agent_provider_core::SessionConfig;

    sdkwork_agent_provider_core::define_provider_lifecycle_provider!(TestLifecycleProvider, "test");

    #[test]
    fn collapses_changes_and_synchronizes_latest_provider_state() {
        let provider = TestLifecycleProvider::new();
        let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
        let created = provider
            .create_session(
                "agent.test",
                None,
                SessionConfig::new()
                    .with_source(SessionSource::Api)
                    .with_kind(SessionKind::Main),
            )
            .expect("created");
        let mut updated = created.clone();
        updated.title = Some("latest".to_string());
        provider.update_session(updated).expect("updated");

        let report = ProviderSessionSynchronizer::synchronize_once(
            &manager,
            "test",
            Some("bridge.test"),
            &provider,
            0,
            Some(20),
        )
        .expect("synchronized");
        assert_eq!(report.synchronized, 1);
        assert_eq!(report.deleted, 0);
        assert_eq!(
            manager
                .get_session(&created.session_id)
                .expect("session")
                .title
                .as_deref(),
            Some("latest")
        );

        provider
            .delete_session(&created.session_id)
            .expect("provider delete");
        let deleted = ProviderSessionSynchronizer::synchronize_once(
            &manager,
            "test",
            Some("bridge.test"),
            &provider,
            report.next_cursor,
            Some(20),
        )
        .expect("delete synchronized");
        assert_eq!(deleted.deleted, 1);
        assert!(manager.get_session(&created.session_id).is_err());
    }

    struct MissingSnapshotProvider {
        session_id: String,
    }

    impl SessionLifecycleProvider for MissingSnapshotProvider {
        fn create_session(
            &self,
            _agent_id: &str,
            _user_ref: Option<&str>,
            _config: SessionConfig,
        ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
            Err(sdkwork_agent_kernel::KernelError::validation("unsupported"))
        }

        fn resume_session(
            &self,
            _session_id: &str,
        ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
            Err(sdkwork_agent_kernel::KernelError::validation("unsupported"))
        }

        fn close_session(
            &self,
            _session_id: &str,
        ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
            Err(sdkwork_agent_kernel::KernelError::validation("unsupported"))
        }

        fn list_active_sessions(
            &self,
        ) -> sdkwork_agent_kernel::KernelResult<Vec<sdkwork_agent_kernel::AgentSession>> {
            Ok(Vec::new())
        }

        fn get_session(
            &self,
            _session_id: &str,
        ) -> sdkwork_agent_kernel::KernelResult<sdkwork_agent_kernel::AgentSession> {
            Err(sdkwork_agent_kernel::KernelError::validation(
                "session not found",
            ))
        }

        fn find_session(
            &self,
            _session_id: &str,
        ) -> sdkwork_agent_kernel::KernelResult<Option<sdkwork_agent_kernel::AgentSession>>
        {
            Ok(None)
        }

        fn session_changes(
            &self,
            _after_sequence: u64,
            _limit: Option<usize>,
        ) -> sdkwork_agent_kernel::KernelResult<
            sdkwork_agent_provider_core::ProviderSessionChangeBatch,
        > {
            Ok(sdkwork_agent_provider_core::ProviderSessionChangeBatch {
                changes: vec![sdkwork_agent_provider_core::ProviderSessionChange {
                    sequence: 1,
                    provider_id: "test".to_string(),
                    session_id: self.session_id.clone(),
                    kind: ProviderSessionChangeKind::Updated,
                    state: Some(sdkwork_agent_kernel::SessionState::Working),
                    occurred_at: "2026-07-15T00:00:00Z".to_string(),
                }],
                next_cursor: 1,
                has_more: false,
            })
        }
    }

    #[test]
    fn missing_snapshot_after_change_converges_to_delete() {
        let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
        let mut native_session = sdkwork_agent_kernel::AgentSession::new("test.session.missing")
            .with_agent_id("agent.test")
            .created_at("2026-07-15T00:00:00Z");
        native_session.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        let session = manager
            .synchronize_provider_session("test", None, &native_session)
            .expect("session");
        let provider = MissingSnapshotProvider {
            session_id: session.session_id.clone(),
        };

        let report = ProviderSessionSynchronizer::synchronize_once(
            &manager,
            "test",
            None,
            &provider,
            0,
            Some(20),
        )
        .expect("converged");
        assert_eq!(report.deleted, 1);
        assert!(manager.get_session(&session.session_id).is_err());
    }

    #[test]
    fn missing_snapshot_cannot_delete_another_providers_session() {
        let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
        let mut native_session = sdkwork_agent_kernel::AgentSession::new("shared.session.id")
            .with_agent_id("agent.codex")
            .created_at("2026-07-15T00:00:00Z");
        native_session.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        manager
            .synchronize_provider_session("codex", None, &native_session)
            .expect("codex session");
        let provider = MissingSnapshotProvider {
            session_id: native_session.session_id.clone(),
        };

        let error = ProviderSessionSynchronizer::synchronize_once(
            &manager,
            "test",
            None,
            &provider,
            0,
            Some(20),
        )
        .expect_err("cross-provider delete");
        assert!(error.contains("already belongs to provider codex"));
        assert!(manager.get_session(&native_session.session_id).is_ok());
    }
}
