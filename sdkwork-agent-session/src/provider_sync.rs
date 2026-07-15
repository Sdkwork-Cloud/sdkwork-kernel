use crate::UnifiedSessionManager;
use sdkwork_agent_database::{
    AgentDatabase, EventRepository, MessageRepository, RuntimeSessionWrites, SessionRepository,
    TaskRepository,
};
use sdkwork_agent_provider_core::{
    ProviderSessionChange, ProviderSessionChangeKind, SessionLifecycleProvider, SessionListQuery,
};
use std::collections::{HashMap, HashSet};

const DEFAULT_INVENTORY_PAGE_SIZE: usize = 100;
const MAX_INVENTORY_PAGE_SIZE: usize = 200;
const MAX_INVENTORY_PAGES: usize = 10_000;
const DEFAULT_CHANGE_PAGE_SIZE: usize = 20;
const MAX_CHANGE_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProviderSessionSyncReport {
    pub next_cursor: u64,
    pub synchronized: usize,
    pub deleted: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProviderSessionInventorySyncReport {
    pub pages: usize,
    pub discovered: usize,
    pub deleted: usize,
}

/// Applies one bounded provider change page to the unified transient session store.
pub struct ProviderSessionSynchronizer;

impl ProviderSessionSynchronizer {
    /// Imports a provider's persisted session inventory using stable keyset
    /// pagination. Re-running this method after process restart is safe:
    /// `UnifiedSessionManager` rejects stale snapshots and avoids writes and
    /// events for snapshots that are already current.
    pub fn synchronize_inventory<DB, P>(
        manager: &UnifiedSessionManager<DB>,
        provider_id: &str,
        bridge_id: Option<&str>,
        provider: &P,
        page_size: Option<usize>,
    ) -> Result<ProviderSessionInventorySyncReport, String>
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
        let page_size = page_size
            .unwrap_or(DEFAULT_INVENTORY_PAGE_SIZE)
            .clamp(1, MAX_INVENTORY_PAGE_SIZE);
        let mut report = ProviderSessionInventorySyncReport::default();
        let mut after_updated_at = None;
        let mut after_session_id = None;
        let mut discovered_ids = HashSet::new();

        loop {
            if report.pages >= MAX_INVENTORY_PAGES {
                return Err(format!(
                    "provider {provider_id} session inventory exceeded {MAX_INVENTORY_PAGES} pages"
                ));
            }
            let page = provider
                .list_sessions(&SessionListQuery {
                    limit: Some(page_size),
                    after_updated_at: after_updated_at.clone(),
                    after_session_id: after_session_id.clone(),
                    ..SessionListQuery::default()
                })
                .map_err(|error| format!("failed to load provider session inventory: {error}"))?;
            report.pages += 1;
            if page.len() > page_size {
                return Err(format!(
                    "provider {provider_id} returned {} sessions for page size {page_size}",
                    page.len()
                ));
            }
            if page.is_empty() {
                break;
            }

            let mut page_ids = HashSet::with_capacity(page.len());
            let mut previous_key = after_updated_at.clone().zip(after_session_id.clone());
            let mut last_key = None;
            for session in &page {
                let key = provider_inventory_session_key(provider_id, session)?;
                if previous_key
                    .as_ref()
                    .is_some_and(|previous| key >= *previous)
                {
                    return Err(format!(
                        "provider {provider_id} returned an out-of-order session inventory key ({}, {}) after ({}, {})",
                        key.0,
                        key.1,
                        previous_key.as_ref().map(|value| value.0.as_str()).unwrap_or_default(),
                        previous_key.as_ref().map(|value| value.1.as_str()).unwrap_or_default()
                    ));
                }
                if discovered_ids.contains(&session.session_id)
                    || !page_ids.insert(session.session_id.clone())
                {
                    return Err(format!(
                        "provider {provider_id} repeated session {} across inventory pages",
                        session.session_id
                    ));
                }
                previous_key = Some(key.clone());
                last_key = Some(key);
            }

            for session in &page {
                manager.synchronize_provider_session(provider_id, bridge_id, session)?;
                discovered_ids.insert(session.session_id.clone());
                report.discovered += 1;
            }

            if page.len() < page_size {
                break;
            }
            let (next_updated_at, next_session_id) =
                last_key.expect("non-empty provider inventory page has a validated key");
            if after_updated_at.as_deref() == Some(next_updated_at.as_str())
                && after_session_id.as_deref() == Some(next_session_id.as_str())
            {
                return Err(format!(
                    "provider {provider_id} did not advance its session inventory cursor"
                ));
            }
            after_updated_at = Some(next_updated_at);
            after_session_id = Some(next_session_id);
        }

        report.deleted = reconcile_missing_inventory_sessions(
            manager,
            provider_id,
            bridge_id,
            provider,
            &discovered_ids,
        )?;

        Ok(report)
    }

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
        let page_size = limit
            .unwrap_or(DEFAULT_CHANGE_PAGE_SIZE)
            .clamp(1, MAX_CHANGE_PAGE_SIZE);
        let batch = provider
            .session_changes(after_sequence, Some(page_size))
            .map_err(|error| format!("failed to load provider session changes: {error}"))?;
        if batch.changes.len() > page_size {
            return Err(format!(
                "provider {provider_id} returned {} session changes for page size {page_size}",
                batch.changes.len()
            ));
        }
        let mut previous_sequence = after_sequence;
        for change in &batch.changes {
            if change.sequence <= previous_sequence {
                return Err(format!(
                    "provider {provider_id} returned non-increasing session change sequence {} after {}",
                    change.sequence, previous_sequence
                ));
            }
            previous_sequence = change.sequence;
        }
        if batch.next_cursor != previous_sequence {
            return Err(format!(
                "provider {provider_id} session change cursor {} does not match last applied sequence {previous_sequence}",
                batch.next_cursor
            ));
        }
        if batch.has_more && batch.next_cursor == after_sequence {
            return Err(format!(
                "provider {provider_id} reported more session changes without advancing its cursor"
            ));
        }

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

fn provider_inventory_session_key(
    provider_id: &str,
    session: &sdkwork_agent_kernel::AgentSession,
) -> Result<(String, String), String> {
    if session.session_id.trim().is_empty() {
        return Err(format!(
            "provider {provider_id} returned a session with an empty session_id"
        ));
    }
    let raw_timestamp = session
        .updated_at
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            session
                .created_at
                .as_deref()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| {
            format!(
                "provider {provider_id} session {} has no inventory sort timestamp",
                session.session_id
            )
        })?;
    let timestamp = sdkwork_utils_rust::parse_datetime(raw_timestamp, None)
        .map(|parsed| sdkwork_utils_rust::format_datetime(parsed, Some("%Y-%m-%dT%H:%M:%S%.9fZ")))
        .ok_or_else(|| {
            format!(
                "provider {provider_id} session {} has an invalid inventory sort timestamp",
                session.session_id
            )
        })?;
    Ok((timestamp, session.session_id.clone()))
}

fn reconcile_missing_inventory_sessions<DB, P>(
    manager: &UnifiedSessionManager<DB>,
    provider_id: &str,
    bridge_id: Option<&str>,
    provider: &P,
    discovered_ids: &HashSet<String>,
) -> Result<usize, String>
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
    let mut pages = 0usize;
    let mut after_session_id = None;
    let mut after_session_sort_at = None;
    let mut candidates = Vec::new();
    loop {
        if pages >= MAX_INVENTORY_PAGES {
            return Err(format!(
                "provider {provider_id} unified session reconciliation exceeded {MAX_INVENTORY_PAGES} pages"
            ));
        }
        let page = manager.list_sessions(crate::SessionQuery {
            provider_id: Some(provider_id.to_string()),
            after_session_id: after_session_id.clone(),
            after_session_sort_at: after_session_sort_at.clone(),
            limit: Some(MAX_INVENTORY_PAGE_SIZE as i64),
            ..crate::SessionQuery::default()
        })?;
        pages += 1;
        if page.is_empty() {
            break;
        }
        let last = page.last().expect("non-empty unified session page");
        let next_session_id = last.session_id.clone();
        let next_sort_at = last
            .updated_at
            .clone()
            .unwrap_or_else(|| last.created_at.clone());
        for session in &page {
            if !discovered_ids.contains(&session.session_id) {
                candidates.push(session.session_id.clone());
            }
        }
        if page.len() < MAX_INVENTORY_PAGE_SIZE {
            break;
        }
        if after_session_id.as_deref() == Some(next_session_id.as_str())
            && after_session_sort_at.as_deref() == Some(next_sort_at.as_str())
        {
            return Err(format!(
                "provider {provider_id} unified session reconciliation cursor did not advance"
            ));
        }
        after_session_id = Some(next_session_id);
        after_session_sort_at = Some(next_sort_at);
    }

    let mut confirmed_missing = Vec::new();
    for session_id in candidates {
        match provider.find_session(&session_id) {
            Ok(Some(session)) => {
                manager.synchronize_provider_session(provider_id, bridge_id, &session)?;
            }
            Ok(None) => confirmed_missing.push(session_id),
            Err(error) => {
                return Err(format!(
                    "failed to verify provider session {session_id} during inventory reconciliation: {error}"
                ));
            }
        }
    }

    let mut deleted = 0usize;
    for session_id in confirmed_missing {
        if delete_provider_session(manager, provider_id, &session_id)? {
            deleted = deleted.saturating_add(1);
        }
    }
    Ok(deleted)
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

    #[test]
    fn imports_complete_provider_inventory_with_keyset_pages_and_replays_without_writes() {
        let provider = TestLifecycleProvider::new();
        let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
        let mut session_ids = Vec::new();
        for index in 0..45 {
            let session = provider
                .create_session(
                    "agent.test",
                    None,
                    SessionConfig::new().with_title(format!("Session {index}")),
                )
                .expect("created provider session");
            session_ids.push(session.session_id);
        }

        let first = ProviderSessionSynchronizer::synchronize_inventory(
            &manager,
            "test",
            Some("bridge.test"),
            &provider,
            Some(7),
        )
        .expect("first inventory sync");
        assert_eq!(first.discovered, 45);
        assert_eq!(first.pages, 7);
        assert_eq!(
            manager
                .list_sessions(crate::SessionQuery {
                    limit: Some(100),
                    ..crate::SessionQuery::default()
                })
                .expect("unified sessions")
                .len(),
            45
        );

        let synchronized_event_count = || {
            session_ids
                .iter()
                .map(|session_id| {
                    manager
                        .load_session_events(session_id, Some(20), None)
                        .expect("session events")
                        .iter()
                        .filter(|event| event.event_type == "session.synchronized")
                        .count()
                })
                .sum::<usize>()
        };
        let synchronized_events_before = synchronized_event_count();
        let replay = ProviderSessionSynchronizer::synchronize_inventory(
            &manager,
            "test",
            Some("bridge.test"),
            &provider,
            Some(7),
        )
        .expect("replayed inventory sync");
        assert_eq!(replay.discovered, 45);
        assert_eq!(synchronized_event_count(), synchronized_events_before);
    }

    #[test]
    fn complete_inventory_reconciles_provider_sessions_missing_after_restart() {
        let provider = TestLifecycleProvider::new();
        let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
        let retained = provider
            .create_session(
                "agent.test",
                None,
                SessionConfig::new().with_title("Retained"),
            )
            .expect("retained");
        let removed = provider
            .create_session(
                "agent.test",
                None,
                SessionConfig::new().with_title("Removed"),
            )
            .expect("removed");
        let first = ProviderSessionSynchronizer::synchronize_inventory(
            &manager,
            "test",
            Some("bridge.test"),
            &provider,
            Some(1),
        )
        .expect("first inventory");
        assert_eq!(first.discovered, 2);
        assert_eq!(first.deleted, 0);

        provider
            .delete_session(&removed.session_id)
            .expect("native deletion");
        let reconciled = ProviderSessionSynchronizer::synchronize_inventory(
            &manager,
            "test",
            Some("bridge.test"),
            &provider,
            Some(1),
        )
        .expect("reconciled inventory");
        assert_eq!(reconciled.discovered, 1);
        assert_eq!(reconciled.deleted, 1);
        assert!(manager.get_session(&retained.session_id).is_ok());
        assert!(manager.get_session(&removed.session_id).is_err());
    }

    struct MissingSnapshotProvider {
        session_id: String,
        batch: Option<sdkwork_agent_provider_core::ProviderSessionChangeBatch>,
        inventory: Vec<sdkwork_agent_kernel::AgentSession>,
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

        fn list_sessions(
            &self,
            _query: &sdkwork_agent_provider_core::SessionListQuery,
        ) -> sdkwork_agent_kernel::KernelResult<Vec<sdkwork_agent_kernel::AgentSession>> {
            Ok(self.inventory.clone())
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
            if let Some(batch) = &self.batch {
                return Ok(batch.clone());
            }
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
            batch: None,
            inventory: Vec::new(),
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
            batch: None,
            inventory: Vec::new(),
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

    #[test]
    fn malformed_change_batches_cannot_advance_the_sync_cursor() {
        let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
        let change = |sequence| sdkwork_agent_provider_core::ProviderSessionChange {
            sequence,
            provider_id: "test".to_string(),
            session_id: format!("test.session.{sequence}"),
            kind: ProviderSessionChangeKind::Updated,
            state: Some(sdkwork_agent_kernel::SessionState::Working),
            occurred_at: "2026-07-15T00:00:00Z".to_string(),
        };
        let malformed = [
            sdkwork_agent_provider_core::ProviderSessionChangeBatch {
                changes: vec![change(2), change(1)],
                next_cursor: 1,
                has_more: false,
            },
            sdkwork_agent_provider_core::ProviderSessionChangeBatch {
                changes: Vec::new(),
                next_cursor: 1,
                has_more: false,
            },
            sdkwork_agent_provider_core::ProviderSessionChangeBatch {
                changes: (1..=MAX_CHANGE_PAGE_SIZE + 1)
                    .map(|sequence| change(sequence as u64))
                    .collect(),
                next_cursor: (MAX_CHANGE_PAGE_SIZE + 1) as u64,
                has_more: false,
            },
        ];
        for batch in malformed {
            let provider = MissingSnapshotProvider {
                session_id: "test.session.malformed".to_string(),
                batch: Some(batch),
                inventory: Vec::new(),
            };
            assert!(ProviderSessionSynchronizer::synchronize_once(
                &manager,
                "test",
                None,
                &provider,
                0,
                Some(MAX_CHANGE_PAGE_SIZE),
            )
            .is_err());
        }
    }

    #[test]
    fn malformed_inventory_page_is_rejected_before_unified_writes() {
        let mut older = sdkwork_agent_kernel::AgentSession::new("test.inventory.older")
            .created_at("2026-07-15T00:00:00Z");
        older.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        let mut newer = sdkwork_agent_kernel::AgentSession::new("test.inventory.newer")
            .created_at("2026-07-15T00:00:00Z");
        newer.updated_at = Some("2026-07-15T00:02:00Z".to_string());
        let mut invalid_timestamp =
            sdkwork_agent_kernel::AgentSession::new("test.inventory.invalid");
        invalid_timestamp.updated_at = Some("eventually".to_string());

        for inventory in [vec![older, newer], vec![invalid_timestamp]] {
            let manager = UnifiedSessionManager::new(InMemoryDatabase::new());
            let provider = MissingSnapshotProvider {
                session_id: "unused".to_string(),
                batch: None,
                inventory,
            };
            assert!(ProviderSessionSynchronizer::synchronize_inventory(
                &manager,
                "test",
                None,
                &provider,
                Some(20),
            )
            .is_err());
            assert!(manager
                .list_sessions(crate::SessionQuery {
                    limit: Some(20),
                    ..crate::SessionQuery::default()
                })
                .expect("unified sessions")
                .is_empty());
        }
    }
}
