use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, KernelResult, MessageQuery, MessageQueryFilter,
    MessageQueryProvider, MessageQueryResult, MessageSortField, MessageSortOrder, ProviderHealth,
    SessionSummary,
};

// ============================================================================
// Helper: create a test message
// ============================================================================

fn make_message(
    message_id: &str,
    role: AgentMessageRole,
    text: &str,
    session_id: Option<&str>,
    task_id: Option<&str>,
    created_at: Option<&str>,
) -> AgentMessage {
    let mut msg = AgentMessage::new(message_id, role, vec![AgentPart::text("part.1", text)]);
    if let Some(sid) = session_id {
        msg = msg.for_session(sid);
    }
    if let Some(tid) = task_id {
        msg = msg.for_task(tid);
    }
    if let Some(ts) = created_at {
        msg = msg.created_at(ts);
    }
    msg
}

// ============================================================================
// Message Sort Field and Order contracts
// ============================================================================

#[test]
fn message_sort_field_str_roundtrip_preserves_identity() {
    for field in [
        MessageSortField::CreatedAt,
        MessageSortField::Role,
        MessageSortField::MessageId,
    ] {
        assert_eq!(MessageSortField::from_str(field.as_str()), Some(field));
    }
    assert_eq!(MessageSortField::from_str("unknown"), None);
}

#[test]
fn message_sort_field_default_is_created_at() {
    assert_eq!(MessageSortField::default(), MessageSortField::CreatedAt);
}

#[test]
fn message_sort_order_str_roundtrip_preserves_identity() {
    assert_eq!(
        MessageSortOrder::from_str("asc"),
        Some(MessageSortOrder::Ascending)
    );
    assert_eq!(
        MessageSortOrder::from_str("ascending"),
        Some(MessageSortOrder::Ascending)
    );
    assert_eq!(
        MessageSortOrder::from_str("desc"),
        Some(MessageSortOrder::Descending)
    );
    assert_eq!(
        MessageSortOrder::from_str("descending"),
        Some(MessageSortOrder::Descending)
    );
    assert_eq!(MessageSortOrder::from_str("unknown"), None);
}

#[test]
fn message_sort_order_default_is_ascending() {
    assert_eq!(MessageSortOrder::default(), MessageSortOrder::Ascending);
}

// ============================================================================
// Message Query Filter contracts
// ============================================================================

#[test]
fn message_query_filter_matches_by_session() {
    let msg = make_message(
        "msg.1",
        AgentMessageRole::User,
        "hello",
        Some("session.1"),
        None,
        None,
    );

    let filter = MessageQueryFilter::new().for_session("session.1");
    assert!(filter.matches(&msg));

    let filter = MessageQueryFilter::new().for_session("session.2");
    assert!(!filter.matches(&msg));
}

#[test]
fn message_query_filter_matches_by_task_and_run_and_step() {
    let msg = make_message(
        "msg.1",
        AgentMessageRole::Agent,
        "response",
        Some("session.1"),
        Some("task.1"),
        None,
    )
    .for_run("run.1")
    .for_step("step.1");

    assert!(MessageQueryFilter::new().for_task("task.1").matches(&msg));
    assert!(MessageQueryFilter::new().for_run("run.1").matches(&msg));
    assert!(MessageQueryFilter::new().for_step("step.1").matches(&msg));
    assert!(!MessageQueryFilter::new().for_task("task.2").matches(&msg));
}

#[test]
fn message_query_filter_matches_by_role() {
    let user_msg = make_message(
        "msg.1",
        AgentMessageRole::User,
        "question",
        Some("session.1"),
        None,
        None,
    );
    let agent_msg = make_message(
        "msg.2",
        AgentMessageRole::Agent,
        "answer",
        Some("session.1"),
        None,
        None,
    );

    assert!(MessageQueryFilter::new()
        .with_role(AgentMessageRole::User)
        .matches(&user_msg));
    assert!(!MessageQueryFilter::new()
        .with_role(AgentMessageRole::User)
        .matches(&agent_msg));
}

#[test]
fn message_query_filter_matches_by_content_contains() {
    let msg = make_message(
        "msg.1",
        AgentMessageRole::User,
        "please review the Rust code",
        Some("session.1"),
        None,
        None,
    );

    assert!(MessageQueryFilter::new()
        .containing_content("Rust")
        .matches(&msg));
    assert!(MessageQueryFilter::new()
        .containing_content("review")
        .matches(&msg));
    assert!(!MessageQueryFilter::new()
        .containing_content("Python")
        .matches(&msg));
}

#[test]
fn message_query_filter_matches_by_created_after_and_before() {
    let msg = make_message(
        "msg.1",
        AgentMessageRole::User,
        "hello",
        Some("session.1"),
        None,
        Some("2026-06-27T12:00:00Z"),
    );

    assert!(MessageQueryFilter::new()
        .created_after("2026-06-27T00:00:00Z")
        .matches(&msg));
    assert!(!MessageQueryFilter::new()
        .created_after("2026-06-28T00:00:00Z")
        .matches(&msg));

    assert!(MessageQueryFilter::new()
        .created_before("2026-06-28T00:00:00Z")
        .matches(&msg));
    assert!(!MessageQueryFilter::new()
        .created_before("2026-06-27T00:00:00Z")
        .matches(&msg));
}

#[test]
fn message_query_filter_untrusted_only_matches_untrusted_messages() {
    let trusted = make_message(
        "msg.1",
        AgentMessageRole::System,
        "system message",
        Some("session.1"),
        None,
        None,
    );
    let untrusted = make_message(
        "msg.2",
        AgentMessageRole::User,
        "user input",
        Some("session.1"),
        None,
        None,
    )
    .mark_untrusted();

    assert!(MessageQueryFilter::new()
        .untrusted_only()
        .matches(&untrusted));
    assert!(!MessageQueryFilter::new().untrusted_only().matches(&trusted));

    assert!(MessageQueryFilter::new().trusted_only().matches(&trusted));
    assert!(!MessageQueryFilter::new().trusted_only().matches(&untrusted));
}

#[test]
fn message_query_filter_combines_multiple_criteria() {
    let msg = make_message(
        "msg.1",
        AgentMessageRole::Agent,
        "code review completed",
        Some("session.1"),
        Some("task.1"),
        Some("2026-06-27T12:00:00Z"),
    );

    let filter = MessageQueryFilter::new()
        .for_session("session.1")
        .for_task("task.1")
        .with_role(AgentMessageRole::Agent)
        .containing_content("review")
        .created_after("2026-06-27T00:00:00Z")
        .created_before("2026-06-28T00:00:00Z");

    assert!(filter.matches(&msg));

    // Failing any single criterion should fail the match
    assert!(!filter.for_session("session.2").matches(&msg));
}

// ============================================================================
// Message Query contracts
// ============================================================================

#[test]
fn message_query_for_session_creates_filtered_query() {
    let query = MessageQuery::for_session("session.1");
    assert_eq!(query.filter.session_id.as_deref(), Some("session.1"));
    assert_eq!(query.sort_field, MessageSortField::CreatedAt);
    assert_eq!(query.sort_order, MessageSortOrder::Ascending);
}

#[test]
fn message_query_for_task_creates_filtered_query() {
    let query = MessageQuery::for_task("task.1");
    assert_eq!(query.filter.task_id.as_deref(), Some("task.1"));
}

#[test]
fn message_query_for_run_creates_filtered_query() {
    let query = MessageQuery::for_run("run.1");
    assert_eq!(query.filter.run_id.as_deref(), Some("run.1"));
}

#[test]
fn message_query_sort_by_sets_field_and_order() {
    let query = MessageQuery::new().sort_by(MessageSortField::Role, MessageSortOrder::Descending);
    assert_eq!(query.sort_field, MessageSortField::Role);
    assert_eq!(query.sort_order, MessageSortOrder::Descending);
}

#[test]
fn message_query_with_limit_and_offset() {
    let query = MessageQuery::new().with_limit(10).with_offset(20);
    assert_eq!(query.limit, Some(10));
    assert_eq!(query.offset, Some(20));
}

#[test]
fn message_query_validate_rejects_zero_limit() {
    let query = MessageQuery::new().with_limit(0);
    assert!(query.validate().is_err());
}

#[test]
fn message_query_validate_accepts_positive_limit() {
    let query = MessageQuery::new().with_limit(1);
    assert!(query.validate().is_ok());
}

#[test]
fn message_query_validate_accepts_no_limit() {
    let query = MessageQuery::new();
    assert!(query.validate().is_ok());
}

// ============================================================================
// Message Query Result contracts
// ============================================================================

#[test]
fn message_query_result_tracks_total_count_and_has_more() {
    let messages = vec![
        make_message("msg.1", AgentMessageRole::User, "a", None, None, None),
        make_message("msg.2", AgentMessageRole::Agent, "b", None, None, None),
    ];

    let result = MessageQueryResult::new(messages)
        .with_total_count(100)
        .with_has_more(true)
        .with_query_id("query.1");

    assert_eq!(result.len(), 2);
    assert_eq!(result.total_count, 100);
    assert!(result.has_more);
    assert_eq!(result.query_id.as_deref(), Some("query.1"));
    assert!(!result.is_empty());
}

#[test]
fn message_query_result_empty_detection() {
    let result = MessageQueryResult::new(vec![]);
    assert!(result.is_empty());
    assert_eq!(result.len(), 0);
    assert_eq!(result.total_count, 0);
}

// ============================================================================
// Session Summary contracts
// ============================================================================

#[test]
fn session_summary_tracks_message_metadata() {
    let summary = SessionSummary::new("session.1")
        .with_message_count(42)
        .with_first_message_at("2026-06-27T08:00:00Z")
        .with_last_message_at("2026-06-27T17:00:00Z")
        .with_roles(vec![AgentMessageRole::User, AgentMessageRole::Agent]);

    assert_eq!(summary.session_id, "session.1");
    assert_eq!(summary.message_count, 42);
    assert_eq!(
        summary.first_message_at.as_deref(),
        Some("2026-06-27T08:00:00Z")
    );
    assert_eq!(
        summary.last_message_at.as_deref(),
        Some("2026-06-27T17:00:00Z")
    );
    assert_eq!(summary.roles.len(), 2);
}

// ============================================================================
// Message Query Provider SPI contracts
// ============================================================================

#[test]
fn message_query_provider_manifest_declares_standard_capabilities() {
    let provider = FakeMessageQueryProvider::with_samples();
    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_family, "message_query");
    assert!(manifest.capabilities.contains(&"message.query".to_string()));
    assert!(manifest.capabilities.contains(&"message.count".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"message.list_sessions".to_string()));
}

#[test]
fn message_query_provider_query_returns_filtered_messages() {
    let provider = FakeMessageQueryProvider::with_samples();

    let result = provider
        .query(&MessageQuery::for_session("session.1"))
        .expect("query succeeds");

    assert!(!result.is_empty());
    assert!(result
        .messages
        .iter()
        .all(|m| m.session_id.as_deref() == Some("session.1")));
}

#[test]
fn message_query_provider_count_returns_matching_count() {
    let provider = FakeMessageQueryProvider::with_samples();

    let count = provider
        .count(&MessageQueryFilter::new().for_session("session.1"))
        .expect("count succeeds");

    assert!(count > 0);
}

#[test]
fn message_query_provider_list_sessions_returns_all_sessions() {
    let provider = FakeMessageQueryProvider::with_samples();

    let sessions = provider.list_sessions().expect("list_sessions succeeds");
    assert!(!sessions.is_empty());
    assert!(sessions.iter().any(|s| s.session_id == "session.1"));
    assert!(sessions.iter().any(|s| s.session_id == "session.2"));
}

#[test]
fn message_query_provider_get_message_returns_by_id() {
    let provider = FakeMessageQueryProvider::with_samples();

    let message = provider.get_message("msg.1").expect("get_message succeeds");
    assert_eq!(message.message_id, "msg.1");
}

#[test]
fn message_query_provider_get_message_rejects_unknown_id() {
    let provider = FakeMessageQueryProvider::with_samples();
    assert!(provider.get_message("nonexistent").is_err());
}

#[test]
fn message_query_provider_list_by_session_uses_default_impl() {
    let provider = FakeMessageQueryProvider::with_samples();

    let result = provider
        .list_by_session("session.1", Some(2))
        .expect("list_by_session succeeds");

    assert!(!result.is_empty());
    assert!(result.messages.len() <= 2);
}

#[test]
fn message_query_provider_list_by_role_uses_default_impl() {
    let provider = FakeMessageQueryProvider::with_samples();

    let result = provider
        .list_by_role(AgentMessageRole::User, None)
        .expect("list_by_role succeeds");

    assert!(!result.is_empty());
    assert!(result
        .messages
        .iter()
        .all(|m| m.role == AgentMessageRole::User));
}

#[test]
fn message_query_provider_search_content_uses_default_impl() {
    let provider = FakeMessageQueryProvider::with_samples();

    let result = provider
        .search_content("review", 10)
        .expect("search_content succeeds");

    assert!(!result.is_empty());
    assert!(result.messages.iter().any(|m| m.parts.iter().any(|p| p
        .text
        .as_deref()
        .unwrap_or("")
        .contains("review"))));
}

// ============================================================================
// Fake Message Query Provider
// ============================================================================

struct FakeMessageQueryProvider {
    messages: Vec<AgentMessage>,
}

impl FakeMessageQueryProvider {
    fn with_samples() -> Self {
        let messages = vec![
            make_message(
                "msg.1",
                AgentMessageRole::User,
                "please review the code",
                Some("session.1"),
                Some("task.1"),
                Some("2026-06-27T08:00:00Z"),
            ),
            make_message(
                "msg.2",
                AgentMessageRole::Agent,
                "code review completed successfully",
                Some("session.1"),
                Some("task.1"),
                Some("2026-06-27T08:05:00Z"),
            ),
            make_message(
                "msg.3",
                AgentMessageRole::User,
                "run the test suite",
                Some("session.1"),
                Some("task.2"),
                Some("2026-06-27T09:00:00Z"),
            ),
            make_message(
                "msg.4",
                AgentMessageRole::User,
                "hello from session 2",
                Some("session.2"),
                None,
                Some("2026-06-27T10:00:00Z"),
            ),
            make_message(
                "msg.5",
                AgentMessageRole::Agent,
                "response from session 2",
                Some("session.2"),
                None,
                Some("2026-06-27T10:05:00Z"),
            ),
        ];

        Self { messages }
    }
}

impl MessageQueryProvider for FakeMessageQueryProvider {
    fn query(&self, query: &MessageQuery) -> KernelResult<MessageQueryResult> {
        let mut filtered: Vec<AgentMessage> = self
            .messages
            .iter()
            .filter(|msg| query.filter.matches(msg))
            .cloned()
            .collect();

        match query.sort_field {
            MessageSortField::CreatedAt => {
                filtered.sort_by(|a, b| {
                    a.created_at
                        .as_deref()
                        .unwrap_or("")
                        .cmp(b.created_at.as_deref().unwrap_or(""))
                });
            }
            MessageSortField::Role => {
                filtered.sort_by(|a, b| a.role.as_str().cmp(b.role.as_str()));
            }
            MessageSortField::MessageId => {
                filtered.sort_by(|a, b| a.message_id.cmp(&b.message_id));
            }
        }

        if query.sort_order == MessageSortOrder::Descending {
            filtered.reverse();
        }

        let total_count = filtered.len();

        if let Some(offset) = query.offset {
            if offset >= filtered.len() {
                filtered.clear();
            } else {
                filtered = filtered.split_off(offset);
            }
        }

        if let Some(limit) = query.limit {
            filtered.truncate(limit);
        }

        let has_more = total_count > filtered.len();

        Ok(MessageQueryResult::new(filtered)
            .with_total_count(total_count)
            .with_has_more(has_more))
    }

    fn count(&self, filter: &MessageQueryFilter) -> KernelResult<usize> {
        Ok(self
            .messages
            .iter()
            .filter(|msg| filter.matches(msg))
            .count())
    }

    fn list_sessions(&self) -> KernelResult<Vec<SessionSummary>> {
        let mut session_ids: Vec<String> = self
            .messages
            .iter()
            .filter_map(|msg| msg.session_id.clone())
            .collect();
        session_ids.sort();
        session_ids.dedup();

        let summaries = session_ids
            .into_iter()
            .map(|session_id| {
                let session_messages: Vec<&AgentMessage> = self
                    .messages
                    .iter()
                    .filter(|msg| msg.session_id.as_deref() == Some(session_id.as_str()))
                    .collect();

                let mut roles: Vec<AgentMessageRole> =
                    session_messages.iter().map(|m| m.role).collect();
                roles.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                roles.dedup();

                SessionSummary::new(session_id)
                    .with_message_count(session_messages.len())
                    .with_first_message_at(
                        session_messages
                            .iter()
                            .filter_map(|m| m.created_at.as_deref())
                            .min()
                            .unwrap_or(""),
                    )
                    .with_last_message_at(
                        session_messages
                            .iter()
                            .filter_map(|m| m.created_at.as_deref())
                            .max()
                            .unwrap_or(""),
                    )
                    .with_roles(roles)
            })
            .collect();

        Ok(summaries)
    }

    fn get_message(&self, message_id: &str) -> KernelResult<AgentMessage> {
        self.messages
            .iter()
            .find(|msg| msg.message_id == message_id)
            .cloned()
            .ok_or_else(|| sdkwork_agent_kernel::KernelError::validation("message not found"))
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}
