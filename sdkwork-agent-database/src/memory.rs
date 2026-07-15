use crate::error::{DatabaseError, DatabaseResult};
use crate::pagination::{resolve_history_limit, resolve_list_limit, resolve_list_offset};
use crate::traits::*;
use crate::types::*;
use sdkwork_utils_rust::offset_limit_page_from_iter;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// In-memory database implementation for testing
#[derive(Clone)]
pub struct InMemoryDatabase {
    sessions: Arc<Mutex<HashMap<String, SessionRow>>>,
    messages: Arc<Mutex<Vec<MessageRow>>>,
    tasks: Arc<Mutex<HashMap<String, TaskRow>>>,
    events: Arc<Mutex<Vec<EventRow>>>,
    permissions: Arc<Mutex<HashMap<String, PermissionRow>>>,
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            permissions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDatabase for InMemoryDatabase {
    fn execute(&self, sql: &str, _params: &[&dyn DatabaseParam]) -> DatabaseResult<usize> {
        Err(DatabaseError::Query(format!(
            "InMemoryDatabase supports typed repository tests only; raw SQL execute is unsupported: {sql}"
        )))
    }

    fn query_many(
        &self,
        sql: &str,
        _params: &[&dyn DatabaseParam],
    ) -> DatabaseResult<Vec<Box<dyn DatabaseRow>>> {
        Err(DatabaseError::Query(format!(
            "InMemoryDatabase supports typed repository tests only; raw SQL query_many is unsupported: {sql}"
        )))
    }

    fn health(&self) -> DatabaseResult<bool> {
        Ok(true)
    }
}

fn is_terminal_state(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "closed"
            | "completed"
            | "complete"
            | "failed"
            | "cancelled"
            | "canceled"
            | "terminated"
            | "expired"
            | "orphaned"
            | "rejected"
            | "denied"
            | "approved"
    )
}

impl RuntimeMaintenance for InMemoryDatabase {
    fn purge_expired(&self, cutoff: &str, batch_size: i64) -> DatabaseResult<RuntimePurgeCounts> {
        if !(1..=10_000).contains(&batch_size) {
            return Err(DatabaseError::Query(
                "runtime purge batch_size must be between 1 and 10000".to_string(),
            ));
        }
        let limit = batch_size as usize;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;

        let expired_sessions: Vec<String> = sessions
            .values()
            .filter(|row| {
                row.created_at.as_str() < cutoff
                    && row.updated_at.as_deref().unwrap_or(row.created_at.as_str()) < cutoff
                    && is_terminal_state(&row.state)
            })
            .take(limit)
            .map(|row| row.session_id.clone())
            .collect();
        let mut counts = RuntimePurgeCounts {
            sessions: expired_sessions.len() as u64,
            ..RuntimePurgeCounts::default()
        };
        for session_id in &expired_sessions {
            sessions.remove(session_id);
            messages.retain(|row| row.session_id != *session_id);
            tasks.retain(|_, row| row.session_id != *session_id);
            events.retain(|row| row.session_id.as_deref() != Some(session_id));
            permissions.retain(|_, row| row.session_id.as_deref() != Some(session_id));
        }

        let mut affected_sessions = HashSet::new();
        let mut removed_messages = 0usize;
        messages.retain(|row| {
            if removed_messages >= limit || row.created_at.as_str() >= cutoff {
                return true;
            }
            removed_messages += 1;
            affected_sessions.insert(row.session_id.clone());
            false
        });
        counts.messages = removed_messages as u64;
        for session_id in affected_sessions {
            if let Some(session) = sessions.get_mut(&session_id) {
                session.message_count = messages
                    .iter()
                    .filter(|row| row.session_id == session_id)
                    .count() as i64;
            }
        }

        let mut removed_tasks = 0usize;
        tasks.retain(|_, row| {
            if removed_tasks >= limit
                || row.updated_at.as_deref().unwrap_or(row.created_at.as_str()) >= cutoff
                || !is_terminal_state(&row.state)
            {
                return true;
            }
            removed_tasks += 1;
            false
        });
        counts.tasks = removed_tasks as u64;

        let mut removed_events = 0usize;
        events.retain(|row| {
            if removed_events >= limit || row.created_at.as_str() >= cutoff {
                return true;
            }
            removed_events += 1;
            false
        });
        counts.events = removed_events as u64;

        let mut removed_permissions = 0usize;
        permissions.retain(|_, row| {
            if removed_permissions >= limit
                || row.updated_at.as_deref().unwrap_or(row.created_at.as_str()) >= cutoff
                || !is_terminal_state(&row.status)
            {
                return true;
            }
            removed_permissions += 1;
            false
        });
        counts.permissions = removed_permissions as u64;
        Ok(counts)
    }

    fn schema_status(&self) -> DatabaseResult<RuntimeSchemaStatus> {
        Ok(RuntimeSchemaStatus {
            version: CURRENT_SCHEMA_VERSION,
            expected_version: CURRENT_SCHEMA_VERSION,
            drift_free: true,
        })
    }

    fn run_maintenance(&self) -> DatabaseResult<()> {
        Ok(())
    }
}

impl SessionRepository for InMemoryDatabase {
    fn save_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        if let Some(existing) = sessions.get_mut(&session.session_id) {
            let message_count = existing.message_count;
            let owner_tenant_id = existing.owner_tenant_id.clone();
            let owner_user_ref = existing.owner_user_ref.clone();
            let created_at = existing.created_at.clone();
            *existing = session.clone();
            existing.message_count = message_count;
            existing.owner_tenant_id = owner_tenant_id;
            existing.owner_user_ref = owner_user_ref;
            existing.created_at = created_at;
        } else {
            sessions.insert(session.session_id.clone(), session.clone());
        }
        Ok(())
    }

    fn load_session(&self, session_id: &str) -> DatabaseResult<Option<SessionRow>> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        Ok(sessions.get(session_id).cloned())
    }

    fn list_sessions(&self, query: &SessionQuery) -> DatabaseResult<Vec<SessionRow>> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let mut results: Vec<SessionRow> = sessions
            .values()
            .filter(|s| {
                if let Some(ref agent_id) = query.agent_id {
                    if s.agent_id != *agent_id {
                        return false;
                    }
                }
                if let Some(ref state) = query.state {
                    if s.state != *state {
                        return false;
                    }
                }
                if let Some(ref kind) = query.kind {
                    if s.kind != *kind {
                        return false;
                    }
                }
                if let Some(ref provider_id) = query.provider_id {
                    if s.provider_id.as_ref() != Some(provider_id) {
                        return false;
                    }
                }
                if let Some(ref bridge_id) = query.bridge_id {
                    if s.bridge_id.as_ref() != Some(bridge_id) {
                        return false;
                    }
                }
                if let Some(ref owner_tenant_id) = query.owner_tenant_id {
                    if s.owner_tenant_id.as_ref() != Some(owner_tenant_id) {
                        return false;
                    }
                }
                if let Some(ref owner_user_ref) = query.owner_user_ref {
                    if s.owner_user_ref.as_ref() != Some(owner_user_ref) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        results.sort_by(|left, right| {
            let left_ts = left
                .updated_at
                .as_deref()
                .unwrap_or(left.created_at.as_str());
            let right_ts = right
                .updated_at
                .as_deref()
                .unwrap_or(right.created_at.as_str());
            right_ts
                .cmp(left_ts)
                .then_with(|| right.session_id.cmp(&left.session_id))
        });
        if let Some(after_session_id) = query
            .after_session_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if let Some(after_sort_at) = query
                .after_session_sort_at
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                results.retain(|row| {
                    let row_sort_at = row.updated_at.as_deref().unwrap_or(&row.created_at);
                    (row_sort_at, row.session_id.as_str()) < (after_sort_at, after_session_id)
                });
            } else {
                let Some(cursor_index) = results
                    .iter()
                    .position(|row| row.session_id == after_session_id)
                else {
                    return Ok(Vec::new());
                };
                results = results.into_iter().skip(cursor_index + 1).collect();
            }
        }

        let limit = resolve_list_limit(query.limit) as usize;
        let offset = resolve_list_offset(query.offset) as usize;
        Ok(offset_limit_page_from_iter(results.into_iter(), limit, offset).items)
    }

    fn update_session(&self, session: &SessionRow) -> DatabaseResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        let existing = sessions.get_mut(&session.session_id).ok_or_else(|| {
            DatabaseError::NotFound(format!("session not found: {}", session.session_id))
        })?;
        let message_count = existing.message_count;
        let owner_tenant_id = existing.owner_tenant_id.clone();
        let owner_user_ref = existing.owner_user_ref.clone();
        let created_at = existing.created_at.clone();
        *existing = session.clone();
        existing.message_count = message_count;
        existing.owner_tenant_id = owner_tenant_id;
        existing.owner_user_ref = owner_user_ref;
        existing.created_at = created_at;
        Ok(())
    }

    fn delete_session(&self, session_id: &str) -> DatabaseResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        sessions.remove(session_id);
        Ok(())
    }

    fn delete_session_cascade(&self, session_id: &str) -> DatabaseResult<()> {
        self.delete_events(session_id)?;
        self.delete_messages(session_id)?;
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        tasks.retain(|_, task| task.session_id != session_id);
        let mut permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        permissions.retain(|_, permission| permission.session_id.as_deref() != Some(session_id));
        self.delete_session(session_id)
    }

    fn increment_session_message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| DatabaseError::NotFound(format!("session not found: {session_id}")))?;
        session.message_count += 1;
        session.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(session.message_count)
    }
}

impl MessageRepository for InMemoryDatabase {
    fn save_message(&self, message: &MessageRow) -> DatabaseResult<()> {
        let mut messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        if let Some(existing) = messages
            .iter()
            .find(|row| row.message_id == message.message_id)
        {
            crate::message_identity::ensure_message_retry_matches(existing, message)?;
            return Ok(());
        }
        messages.push(message.clone());
        Ok(())
    }

    fn load_messages(
        &self,
        session_id: &str,
        query: &MessageQuery,
    ) -> DatabaseResult<Vec<MessageRow>> {
        let messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let mut results: Vec<MessageRow> = messages
            .iter()
            .filter(|m| m.session_id == session_id)
            .cloned()
            .collect();
        results.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.message_id.cmp(&right.message_id))
        });
        if let Some(after_message_id) = query
            .after_message_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if let Some(after_created_at) = query
                .after_message_created_at
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                results.retain(|row| {
                    (row.created_at.as_str(), row.message_id.as_str())
                        > (after_created_at, after_message_id)
                });
            } else {
                let Some(cursor_index) = results
                    .iter()
                    .position(|row| row.message_id == after_message_id)
                else {
                    return Ok(Vec::new());
                };
                results = results.into_iter().skip(cursor_index + 1).collect();
            }
        }
        let limit = resolve_list_limit(query.limit) as usize;
        let offset = resolve_list_offset(query.offset) as usize;
        Ok(offset_limit_page_from_iter(results.into_iter(), limit, offset).items)
    }

    fn load_recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> DatabaseResult<Vec<MessageRow>> {
        let limit = resolve_history_limit(limit)? as usize;
        let messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut recent = std::collections::BTreeMap::new();
        for message in messages.iter().filter(|row| row.session_id == session_id) {
            recent.insert(
                (message.created_at.clone(), message.message_id.clone()),
                message.clone(),
            );
            if recent.len() > limit {
                let Some(oldest) = recent.keys().next().cloned() else {
                    continue;
                };
                recent.remove(&oldest);
            }
        }
        Ok(recent.into_values().collect())
    }

    fn message_count(&self, session_id: &str) -> DatabaseResult<i64> {
        let messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        Ok(messages
            .iter()
            .filter(|m| m.session_id == session_id)
            .count() as i64)
    }

    fn delete_messages(&self, session_id: &str) -> DatabaseResult<()> {
        let mut messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        messages.retain(|m| m.session_id != session_id);
        Ok(())
    }
}

impl TaskRepository for InMemoryDatabase {
    fn save_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        tasks.insert(task.task_id.clone(), task.clone());
        Ok(())
    }

    fn load_task(&self, task_id: &str) -> DatabaseResult<Option<TaskRow>> {
        let tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        Ok(tasks.get(task_id).cloned())
    }

    fn load_tasks(&self, session_id: &str, query: &TaskQuery) -> DatabaseResult<Vec<TaskRow>> {
        let tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        let mut results: Vec<TaskRow> = tasks
            .values()
            .filter(|t| t.session_id == session_id)
            .cloned()
            .collect();
        results.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
        if let Some(after_task_id) = query
            .after_task_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if let Some(after_created_at) = query
                .after_task_created_at
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                results.retain(|row| {
                    (row.created_at.as_str(), row.task_id.as_str())
                        > (after_created_at, after_task_id)
                });
            } else {
                let Some(cursor_index) =
                    results.iter().position(|row| row.task_id == after_task_id)
                else {
                    return Ok(Vec::new());
                };
                results = results.into_iter().skip(cursor_index + 1).collect();
            }
        }
        let limit = resolve_list_limit(query.limit) as usize;
        let offset = resolve_list_offset(query.offset) as usize;
        Ok(offset_limit_page_from_iter(results.into_iter(), limit, offset).items)
    }

    fn update_task(&self, task: &TaskRow) -> DatabaseResult<()> {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        tasks.insert(task.task_id.clone(), task.clone());
        Ok(())
    }

    fn delete_task(&self, task_id: &str) -> DatabaseResult<()> {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        tasks.remove(task_id);
        Ok(())
    }
}

impl EventRepository for InMemoryDatabase {
    fn save_event(&self, event: &EventRow) -> DatabaseResult<()> {
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        events.push(event.clone());
        Ok(())
    }

    fn load_events(&self, session_id: &str, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let mut results: Vec<EventRow> = events
            .iter()
            .filter(|e| {
                e.session_id.as_deref() == Some(session_id)
                    && sessions.get(session_id).is_some_and(|session| {
                        query
                            .owner_tenant_id
                            .as_ref()
                            .is_none_or(|tenant| session.owner_tenant_id.as_ref() == Some(tenant))
                            && query
                                .owner_user_ref
                                .as_ref()
                                .is_none_or(|user| session.owner_user_ref.as_ref() == Some(user))
                    })
                    && (query.event_type.is_none()
                        || e.event_type == *query.event_type.as_ref().unwrap())
                    && (query.severity.is_none() || e.severity == *query.severity.as_ref().unwrap())
            })
            .cloned()
            .collect();
        results.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        if let Some(after_event_id) = query
            .after_event_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            let Some(cursor_index) = results
                .iter()
                .position(|row| row.event_id == after_event_id)
            else {
                return Ok(Vec::new());
            };
            results = results.into_iter().skip(cursor_index + 1).collect();
        }
        let limit = resolve_list_limit(query.limit) as usize;
        let offset = resolve_list_offset(query.offset) as usize;
        Ok(offset_limit_page_from_iter(results.into_iter(), limit, offset).items)
    }

    fn list_recent_events(&self, query: &EventQuery) -> DatabaseResult<Vec<EventRow>> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;

        let mut results: Vec<EventRow> = events
            .iter()
            .filter(|event| {
                (query.event_type.is_none()
                    || event.event_type == *query.event_type.as_ref().unwrap())
                    && (query.severity.is_none()
                        || event.severity == *query.severity.as_ref().unwrap())
                    && if query.owner_tenant_id.is_some() || query.owner_user_ref.is_some() {
                        event
                            .session_id
                            .as_deref()
                            .and_then(|session_id| sessions.get(session_id))
                            .is_some_and(|session| {
                                query.owner_tenant_id.as_ref().is_none_or(|tenant| {
                                    session.owner_tenant_id.as_ref() == Some(tenant)
                                }) && query.owner_user_ref.as_ref().is_none_or(|user| {
                                    session.owner_user_ref.as_ref() == Some(user)
                                })
                            })
                    } else {
                        true
                    }
            })
            .cloned()
            .collect();
        results.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.event_id.cmp(&left.event_id))
        });
        let limit = resolve_list_limit(query.limit) as usize;
        let offset = resolve_list_offset(query.offset) as usize;
        Ok(offset_limit_page_from_iter(results.into_iter(), limit, offset).items)
    }

    fn delete_events(&self, session_id: &str) -> DatabaseResult<()> {
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        events.retain(|e| e.session_id.as_deref() != Some(session_id));
        Ok(())
    }
}

impl PermissionRepository for InMemoryDatabase {
    fn create_permission_if_absent(&self, permission: &PermissionRow) -> DatabaseResult<bool> {
        let mut permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        if permissions.contains_key(&permission.permission_request_id) {
            return Ok(false);
        }
        permissions.insert(permission.permission_request_id.clone(), permission.clone());
        Ok(true)
    }

    fn save_permission(&self, permission: &PermissionRow) -> DatabaseResult<()> {
        let mut permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        permissions.insert(permission.permission_request_id.clone(), permission.clone());
        Ok(())
    }

    fn load_permission(
        &self,
        permission_request_id: &str,
    ) -> DatabaseResult<Option<PermissionRow>> {
        let permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        Ok(permissions.get(permission_request_id).cloned())
    }

    fn list_permissions(&self, query: &PermissionQuery) -> DatabaseResult<Vec<PermissionRow>> {
        let permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        let mut results: Vec<PermissionRow> = permissions
            .values()
            .filter(|p| {
                query
                    .status
                    .as_deref()
                    .is_none_or(|status| p.status == status)
                    && query
                        .owner_tenant_id
                        .as_ref()
                        .is_none_or(|tenant| p.owner_tenant_id.as_ref() == Some(tenant))
                    && query
                        .owner_user_ref
                        .as_ref()
                        .is_none_or(|user| p.owner_user_ref.as_ref() == Some(user))
            })
            .cloned()
            .collect();
        results.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.permission_request_id.cmp(&left.permission_request_id))
        });
        let limit = resolve_list_limit(query.limit) as usize;
        let offset = resolve_list_offset(query.offset) as usize;
        Ok(offset_limit_page_from_iter(results.into_iter(), limit, offset).items)
    }

    fn update_permission_status(
        &self,
        permission_request_id: &str,
        status: &str,
    ) -> DatabaseResult<()> {
        if !matches!(status, "allow" | "deny") {
            return Err(DatabaseError::ConstraintViolation(
                "permission status must be allow or deny".to_string(),
            ));
        }
        let mut permissions = self
            .permissions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {}", e)))?;
        let permission = permissions
            .get_mut(permission_request_id)
            .ok_or_else(|| DatabaseError::NotFound("permission request not found".to_string()))?;
        if permission.status != "pending" && permission.status != status {
            return Err(DatabaseError::ConstraintViolation(
                "permission request state conflict".to_string(),
            ));
        }
        if permission.status == "pending" {
            permission.status = status.to_string();
            permission.updated_at = Some(chrono::Utc::now().to_rfc3339());
        }
        Ok(())
    }
}

impl RuntimeSessionWrites for InMemoryDatabase {
    fn save_session_with_event(
        &self,
        session: &SessionRow,
        event: &EventRow,
    ) -> DatabaseResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        if let Some(existing) = sessions.get_mut(&session.session_id) {
            let message_count = existing.message_count;
            let owner_tenant_id = existing.owner_tenant_id.clone();
            let owner_user_ref = existing.owner_user_ref.clone();
            let created_at = existing.created_at.clone();
            *existing = session.clone();
            existing.message_count = message_count;
            existing.owner_tenant_id = owner_tenant_id;
            existing.owner_user_ref = owner_user_ref;
            existing.created_at = created_at;
        } else {
            sessions.insert(session.session_id.clone(), session.clone());
        }
        if let Some(existing) = events.iter_mut().find(|row| row.event_id == event.event_id) {
            *existing = event.clone();
        } else {
            events.push(event.clone());
        }
        Ok(())
    }

    fn save_session_with_event_if_newer(
        &self,
        session: &SessionRow,
        event: &EventRow,
    ) -> DatabaseResult<bool> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        if let Some(existing) = sessions.get(&session.session_id) {
            if crate::types::session_provider_conflicts(session, existing)
                || crate::types::session_snapshot_is_older(session, existing)
            {
                return Ok(false);
            }
        }
        if let Some(existing) = sessions.get_mut(&session.session_id) {
            let created_at = existing.created_at.clone();
            *existing = session.clone();
            existing.created_at = created_at;
        } else {
            sessions.insert(session.session_id.clone(), session.clone());
        }
        if let Some(existing) = events.iter_mut().find(|row| row.event_id == event.event_id) {
            *existing = event.clone();
        } else {
            events.push(event.clone());
        }
        Ok(true)
    }

    fn append_message_with_event(
        &self,
        message: &MessageRow,
        event: &EventRow,
    ) -> DatabaseResult<i64> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let session = sessions.get_mut(&message.session_id).ok_or_else(|| {
            DatabaseError::NotFound(format!("session not found: {}", message.session_id))
        })?;
        let existing_message = messages
            .iter()
            .find(|row| row.message_id == message.message_id)
            .cloned();
        if let Some(existing_message) = &existing_message {
            crate::message_identity::ensure_message_retry_matches(existing_message, message)?;
        }
        let message_is_new = existing_message.is_none();
        if message_is_new {
            messages.push(message.clone());
            session.message_count += 1;
            session.updated_at = Some(chrono::Utc::now().to_rfc3339());
            events.push(event.clone());
        }
        let count = session.message_count;
        Ok(count)
    }

    fn append_message_turn_with_events(
        &self,
        turn_messages: &[MessageRow],
        turn_events: &[EventRow],
    ) -> DatabaseResult<i64> {
        let session_id =
            crate::message_identity::validate_message_turn(turn_messages, turn_events)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| DatabaseError::NotFound(format!("session not found: {session_id}")))?;
        if !session.state.eq_ignore_ascii_case("active") {
            return Err(DatabaseError::ConstraintViolation(format!(
                "session {session_id} is not active"
            )));
        }

        let mut new_messages = Vec::with_capacity(turn_messages.len());
        for message in turn_messages {
            if let Some(existing) = messages
                .iter()
                .find(|row| row.message_id == message.message_id)
            {
                crate::message_identity::ensure_message_retry_matches(existing, message)?;
            } else {
                new_messages.push(message.clone());
            }
        }

        if !new_messages.is_empty() {
            let added = i64::try_from(new_messages.len()).map_err(|_| {
                DatabaseError::ConstraintViolation("message turn size overflow".to_string())
            })?;
            session.message_count = session.message_count.checked_add(added).ok_or_else(|| {
                DatabaseError::ConstraintViolation("session message count overflow".to_string())
            })?;
            session.updated_at = Some(chrono::Utc::now().to_rfc3339());
            messages.extend(new_messages);
            for event in turn_events {
                if let Some(existing) = events.iter_mut().find(|row| row.event_id == event.event_id)
                {
                    *existing = event.clone();
                } else {
                    events.push(event.clone());
                }
            }
        }

        Ok(session.message_count)
    }

    fn delete_messages_and_reset_count(
        &self,
        session_id: &str,
        updated_at: &str,
    ) -> DatabaseResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut messages = self
            .messages
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| DatabaseError::NotFound(format!("session not found: {session_id}")))?;
        messages.retain(|row| row.session_id != session_id);
        session.message_count = 0;
        session.updated_at = Some(updated_at.to_string());
        Ok(())
    }

    fn save_task_with_event(&self, task: &TaskRow, event: &EventRow) -> DatabaseResult<()> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        if !sessions.contains_key(&task.session_id) {
            return Err(DatabaseError::NotFound(format!(
                "session not found: {}",
                task.session_id
            )));
        }
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        tasks.insert(task.task_id.clone(), task.clone());
        if let Some(existing) = events.iter_mut().find(|row| row.event_id == event.event_id) {
            *existing = event.clone();
        } else {
            events.push(event.clone());
        }
        Ok(())
    }

    fn cancel_task_with_event(
        &self,
        task_id: &str,
        updated_at: &str,
        event: &EventRow,
    ) -> DatabaseResult<(TaskRow, bool)> {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let mut events = self
            .events
            .lock()
            .map_err(|e| DatabaseError::Internal(format!("failed to acquire lock: {e}")))?;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| DatabaseError::NotFound(format!("task not found: {task_id}")))?;
        if task.state == "cancelled" {
            return Ok((task.clone(), false));
        }
        if !matches!(task.state.as_str(), "created" | "pending" | "running") {
            return Err(DatabaseError::ConstraintViolation(format!(
                "task {task_id} is not active"
            )));
        }
        if event.session_id.as_deref() != Some(task.session_id.as_str()) {
            return Err(DatabaseError::ConstraintViolation(
                "task cancellation event session mismatch".to_string(),
            ));
        }
        task.state = "cancelled".to_string();
        task.updated_at = Some(updated_at.to_string());
        events.push(event.clone());
        Ok((task.clone(), true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_session() {
        let db = InMemoryDatabase::new();
        let session = SessionRow {
            session_id: "session.1".to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "cli".to_string(),
            state: "active".to_string(),
            title: Some("Test".to_string()),
            model: Some("gpt-4".to_string()),
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        };

        db.save_session(&session).expect("saved");
        let loaded = db.load_session("session.1").expect("loaded");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().session_id, "session.1");
    }

    #[test]
    fn conditional_session_write_rejects_stale_snapshot_and_event() {
        let db = InMemoryDatabase::new();
        let mut session = SessionRow {
            session_id: "session.conditional.memory".to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "provider".to_string(),
            state: "working".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: Some("codex".to_string()),
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-07-15T00:00:00Z".to_string(),
            updated_at: Some("2026-07-15T00:02:00Z".to_string()),
            metadata_json: None,
        };
        let applied = db
            .save_session_with_event_if_newer(
                &session,
                &EventRow {
                    event_id: "evt.memory.newer".to_string(),
                    session_id: Some(session.session_id.clone()),
                    event_type: "session.synchronized".to_string(),
                    severity: "info".to_string(),
                    payload: None,
                    created_at: "2026-07-15T00:02:00Z".to_string(),
                },
            )
            .expect("newer write");
        assert!(applied);

        session.message_count = 12;
        session.owner_tenant_id = Some("tenant.updated".to_string());
        session.owner_user_ref = Some("user.updated".to_string());
        session.updated_at = Some("2026-07-15T00:03:00Z".to_string());
        assert!(db
            .save_session_with_event_if_newer(
                &session,
                &EventRow {
                    event_id: "evt.memory.aggregate-update".to_string(),
                    session_id: Some(session.session_id.clone()),
                    event_type: "session.synchronized".to_string(),
                    severity: "info".to_string(),
                    payload: None,
                    created_at: "2026-07-15T00:03:00Z".to_string(),
                },
            )
            .expect("aggregate update"));
        let updated = db
            .load_session(&session.session_id)
            .expect("load updated")
            .expect("updated session");
        assert_eq!(updated.message_count, 12);
        assert_eq!(updated.owner_tenant_id.as_deref(), Some("tenant.updated"));

        let mut foreign = session.clone();
        foreign.provider_id = Some("claude-code".to_string());
        foreign.updated_at = Some("2026-07-15T00:04:00Z".to_string());
        assert!(!db
            .save_session_with_event_if_newer(
                &foreign,
                &EventRow {
                    event_id: "evt.memory.foreign-provider".to_string(),
                    session_id: Some(session.session_id.clone()),
                    event_type: "session.synchronized".to_string(),
                    severity: "info".to_string(),
                    payload: None,
                    created_at: "2026-07-15T00:04:00Z".to_string(),
                },
            )
            .expect("provider conflict"));

        session.state = "paused".to_string();
        session.updated_at = Some("2026-07-15T00:01:00Z".to_string());
        let applied = db
            .save_session_with_event_if_newer(
                &session,
                &EventRow {
                    event_id: "evt.memory.stale".to_string(),
                    session_id: Some(session.session_id.clone()),
                    event_type: "session.synchronized".to_string(),
                    severity: "info".to_string(),
                    payload: None,
                    created_at: "2026-07-15T00:01:00Z".to_string(),
                },
            )
            .expect("stale write");
        assert!(!applied);
        assert_eq!(
            db.load_session(&session.session_id)
                .expect("load")
                .expect("session")
                .state,
            "working"
        );
        let events = db
            .load_events(
                &session.session_id,
                &EventQuery {
                    limit: Some(20),
                    ..EventQuery::default()
                },
            )
            .expect("events");
        assert!(events
            .iter()
            .all(|event| event.event_id != "evt.memory.stale"));
    }

    #[test]
    fn list_sessions_with_filter() {
        let db = InMemoryDatabase::new();
        db.save_session(&SessionRow {
            session_id: "session.1".to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "cli".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        })
        .expect("saved");

        let query = SessionQuery {
            agent_id: Some("agent.1".to_string()),
            ..Default::default()
        };
        let sessions = db.list_sessions(&query).expect("listed");
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn save_and_load_messages() {
        let db = InMemoryDatabase::new();
        let message = MessageRow {
            message_id: "msg.1".to_string(),
            session_id: "session.1".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            metadata_json: None,
        };

        db.save_message(&message).expect("saved");
        let messages = db
            .load_messages("session.1", &MessageQuery::default())
            .expect("loaded");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");
    }

    #[test]
    fn save_message_is_idempotent_for_same_row() {
        let db = InMemoryDatabase::new();
        let message = MessageRow {
            message_id: "msg.memory.save.idempotent".to_string(),
            session_id: "session.memory.save".to_string(),
            role: "user".to_string(),
            content: "same payload".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            metadata_json: None,
        };

        db.save_message(&message).expect("first save");
        db.save_message(&message).expect("retry save");

        let messages = db
            .load_messages("session.memory.save", &MessageQuery::default())
            .expect("messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "same payload");
    }

    #[test]
    fn save_message_rejects_duplicate_message_id_with_different_content() {
        let db = InMemoryDatabase::new();
        let message = MessageRow {
            message_id: "msg.memory.save.conflict".to_string(),
            session_id: "session.memory.save".to_string(),
            role: "user".to_string(),
            content: "original payload".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            metadata_json: None,
        };
        db.save_message(&message).expect("first save");

        let conflicting = MessageRow {
            content: "changed payload".to_string(),
            ..message.clone()
        };
        let error = db
            .save_message(&conflicting)
            .expect_err("duplicate message id with changed payload must fail");
        assert!(matches!(error, DatabaseError::ConstraintViolation(_)));

        let messages = db
            .load_messages("session.memory.save", &MessageQuery::default())
            .expect("messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "original payload");
    }

    #[test]
    fn health_check() {
        let db = InMemoryDatabase::new();
        assert!(db.health().expect("health"));
    }

    #[test]
    fn raw_sql_methods_fail_closed() {
        let db = InMemoryDatabase::new();

        let execute_error = db
            .execute("CREATE TABLE sessions (id TEXT)", &[])
            .expect_err("in-memory typed repository must not fake raw SQL execution");
        assert!(matches!(execute_error, DatabaseError::Query(_)));

        let query_error = match db.query_many("SELECT * FROM sessions", &[]) {
            Ok(_) => panic!("in-memory typed repository must not fake raw SQL queries"),
            Err(error) => error,
        };
        assert!(matches!(query_error, DatabaseError::Query(_)));
    }

    #[test]
    fn append_message_with_event_is_idempotent_for_duplicate_message_id() {
        let db = InMemoryDatabase::new();
        let session_id = "session.memory.idempotent-append";
        db.save_session(&SessionRow {
            session_id: session_id.to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "test".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        })
        .expect("session saved");

        let message = MessageRow {
            message_id: "msg.memory.idempotent.1".to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: "retry-safe append".to_string(),
            created_at: "2026-01-01T00:00:01Z".to_string(),
            metadata_json: None,
        };
        let event = EventRow {
            event_id: "evt.memory.idempotent.1".to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "message.sent".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: "2026-01-01T00:00:01Z".to_string(),
        };

        let first_count = db
            .append_message_with_event(&message, &event)
            .expect("first append");
        let retry_count = db
            .append_message_with_event(&message, &event)
            .expect("retry append");

        assert_eq!(first_count, 1);
        assert_eq!(retry_count, 1);
        assert_eq!(db.message_count(session_id).expect("message count"), 1);
        assert_eq!(
            db.load_session(session_id)
                .expect("load session")
                .expect("session")
                .message_count,
            1
        );
        assert_eq!(
            db.load_events(session_id, &EventQuery::default())
                .expect("events")
                .len(),
            1
        );
    }

    #[test]
    fn append_message_with_event_does_not_write_event_for_duplicate_message_id() {
        let db = InMemoryDatabase::new();
        let session_id = "session.memory.duplicate-event";
        db.save_session(&SessionRow {
            session_id: session_id.to_string(),
            agent_id: "agent.1".to_string(),
            kind: "main".to_string(),
            source: "test".to_string(),
            state: "active".to_string(),
            title: None,
            model: None,
            cwd: None,
            provider_id: None,
            bridge_id: None,
            token_usage_json: None,
            message_count: 0,
            owner_tenant_id: None,
            owner_user_ref: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: None,
            metadata_json: None,
        })
        .expect("session saved");

        let message = MessageRow {
            message_id: "msg.memory.duplicate-event.1".to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: "retry-safe append".to_string(),
            created_at: "2026-01-01T00:00:01Z".to_string(),
            metadata_json: None,
        };
        let first_event = EventRow {
            event_id: "evt.memory.duplicate-event.1".to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "message.sent".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: "2026-01-01T00:00:01Z".to_string(),
        };
        let retry_event = EventRow {
            event_id: "evt.memory.duplicate-event.2".to_string(),
            created_at: "2026-01-01T00:00:02Z".to_string(),
            ..first_event.clone()
        };

        db.append_message_with_event(&message, &first_event)
            .expect("first append");
        let retry_count = db
            .append_message_with_event(&message, &retry_event)
            .expect("retry append");

        let events = db
            .load_events(session_id, &EventQuery::default())
            .expect("events");
        assert_eq!(retry_count, 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "evt.memory.duplicate-event.1");
    }

    #[test]
    fn append_message_with_event_rejects_duplicate_message_id_for_different_session() {
        let db = InMemoryDatabase::new();
        let first_session_id = "session.memory.conflict-a";
        let second_session_id = "session.memory.conflict-b";
        for session_id in [first_session_id, second_session_id] {
            db.save_session(&SessionRow {
                session_id: session_id.to_string(),
                agent_id: "agent.1".to_string(),
                kind: "main".to_string(),
                source: "test".to_string(),
                state: "active".to_string(),
                title: None,
                model: None,
                cwd: None,
                provider_id: None,
                bridge_id: None,
                token_usage_json: None,
                message_count: 0,
                owner_tenant_id: None,
                owner_user_ref: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: None,
                metadata_json: None,
            })
            .expect("session saved");
        }

        let message = MessageRow {
            message_id: "msg.memory.conflict.1".to_string(),
            session_id: first_session_id.to_string(),
            role: "user".to_string(),
            content: "original append".to_string(),
            created_at: "2026-01-01T00:00:01Z".to_string(),
            metadata_json: None,
        };
        let event = EventRow {
            event_id: "evt.memory.conflict.1".to_string(),
            session_id: Some(first_session_id.to_string()),
            event_type: "message.sent".to_string(),
            severity: "info".to_string(),
            payload: None,
            created_at: "2026-01-01T00:00:01Z".to_string(),
        };
        db.append_message_with_event(&message, &event)
            .expect("first append");

        let conflicting_message = MessageRow {
            session_id: second_session_id.to_string(),
            content: "conflicting append".to_string(),
            ..message
        };
        let conflicting_event = EventRow {
            event_id: "evt.memory.conflict.2".to_string(),
            session_id: Some(second_session_id.to_string()),
            ..event
        };

        let error = db
            .append_message_with_event(&conflicting_message, &conflicting_event)
            .expect_err("duplicate message id must not move across sessions");
        assert!(matches!(error, DatabaseError::ConstraintViolation(_)));
        assert_eq!(db.message_count(first_session_id).expect("first count"), 1);
        assert_eq!(
            db.message_count(second_session_id).expect("second count"),
            0
        );
        assert!(db
            .load_messages(second_session_id, &MessageQuery::default())
            .expect("second messages")
            .is_empty());
        assert!(db
            .load_events(second_session_id, &EventQuery::default())
            .expect("second events")
            .is_empty());
    }
}
