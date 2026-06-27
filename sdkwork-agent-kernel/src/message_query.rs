use crate::{
    AgentMessage, AgentMessageRole, KernelError, KernelResult, ProviderHealth, ProviderManifest,
};

// ============================================================================
// Message Query Sort
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageSortField {
    #[default]
    CreatedAt,
    Role,
    MessageId,
}

impl MessageSortField {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreatedAt => "created_at",
            Self::Role => "role",
            Self::MessageId => "message_id",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "created_at" => Some(Self::CreatedAt),
            "role" => Some(Self::Role),
            "message_id" => Some(Self::MessageId),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageSortOrder {
    #[default]
    Ascending,
    Descending,
}

impl MessageSortOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "asc" | "ascending" => Some(Self::Ascending),
            "desc" | "descending" => Some(Self::Descending),
            _ => None,
        }
    }
}

// ============================================================================
// Message Query Filter
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageQueryFilter {
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub role: Option<AgentMessageRole>,
    pub content_contains: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub untrusted_only: Option<bool>,
    pub tenant_id: Option<String>,
    pub user_ref: Option<String>,
}

impl MessageQueryFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn for_run(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn for_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    pub fn with_role(mut self, role: AgentMessageRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn containing_content(mut self, content: impl Into<String>) -> Self {
        self.content_contains = Some(content.into());
        self
    }

    pub fn created_after(mut self, timestamp: impl Into<String>) -> Self {
        self.created_after = Some(timestamp.into());
        self
    }

    pub fn created_before(mut self, timestamp: impl Into<String>) -> Self {
        self.created_before = Some(timestamp.into());
        self
    }

    pub fn untrusted_only(mut self) -> Self {
        self.untrusted_only = Some(true);
        self
    }

    pub fn trusted_only(mut self) -> Self {
        self.untrusted_only = Some(false);
        self
    }

    pub fn for_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn for_user(mut self, user_ref: impl Into<String>) -> Self {
        self.user_ref = Some(user_ref.into());
        self
    }

    pub fn matches(&self, message: &AgentMessage) -> bool {
        if let Some(session_id) = &self.session_id {
            if message.session_id.as_deref() != Some(session_id.as_str()) {
                return false;
            }
        }
        if let Some(task_id) = &self.task_id {
            if message.task_id.as_deref() != Some(task_id.as_str()) {
                return false;
            }
        }
        if let Some(run_id) = &self.run_id {
            if message.run_id.as_deref() != Some(run_id.as_str()) {
                return false;
            }
        }
        if let Some(step_id) = &self.step_id {
            if message.step_id.as_deref() != Some(step_id.as_str()) {
                return false;
            }
        }
        if let Some(role) = self.role {
            if message.role != role {
                return false;
            }
        }
        if let Some(content) = &self.content_contains {
            let has_content = message.parts.iter().any(|part| {
                part.text
                    .as_deref()
                    .is_some_and(|text| text.contains(content))
            });
            if !has_content {
                return false;
            }
        }
        if let Some(after) = &self.created_after {
            if message
                .created_at
                .as_deref()
                .is_none_or(|created| created < after.as_str())
            {
                return false;
            }
        }
        if let Some(before) = &self.created_before {
            if message
                .created_at
                .as_deref()
                .is_none_or(|created| created > before.as_str())
            {
                return false;
            }
        }
        if let Some(untrusted_only) = self.untrusted_only {
            if message.untrusted != untrusted_only {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Message Query
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageQuery {
    pub filter: MessageQueryFilter,
    pub sort_field: MessageSortField,
    pub sort_order: MessageSortOrder,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl MessageQuery {
    pub fn new() -> Self {
        Self {
            filter: MessageQueryFilter::new(),
            sort_field: MessageSortField::default(),
            sort_order: MessageSortOrder::default(),
            limit: None,
            offset: None,
        }
    }

    pub fn with_filter(mut self, filter: MessageQueryFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn sort_by(mut self, field: MessageSortField, order: MessageSortOrder) -> Self {
        self.sort_field = field;
        self.sort_order = order;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn for_session(session_id: impl Into<String>) -> Self {
        Self::new().with_filter(MessageQueryFilter::new().for_session(session_id))
    }

    pub fn for_task(task_id: impl Into<String>) -> Self {
        Self::new().with_filter(MessageQueryFilter::new().for_task(task_id))
    }

    pub fn for_run(run_id: impl Into<String>) -> Self {
        Self::new().with_filter(MessageQueryFilter::new().for_run(run_id))
    }

    pub fn validate(&self) -> KernelResult<()> {
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err(KernelError::validation(
                    "message query limit must be a positive integer",
                ));
            }
        }
        Ok(())
    }
}

impl Default for MessageQuery {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Message Query Result
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageQueryResult {
    pub messages: Vec<AgentMessage>,
    pub total_count: usize,
    pub has_more: bool,
    pub query_id: Option<String>,
}

impl MessageQueryResult {
    pub fn new(messages: Vec<AgentMessage>) -> Self {
        let total_count = messages.len();
        Self {
            messages,
            total_count,
            has_more: false,
            query_id: None,
        }
    }

    pub fn with_total_count(mut self, total_count: usize) -> Self {
        self.total_count = total_count;
        self
    }

    pub fn with_has_more(mut self, has_more: bool) -> Self {
        self.has_more = has_more;
        self
    }

    pub fn with_query_id(mut self, query_id: impl Into<String>) -> Self {
        self.query_id = Some(query_id.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

// ============================================================================
// Session Summary - lightweight session info from message queries
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub session_id: String,
    pub message_count: usize,
    pub first_message_at: Option<String>,
    pub last_message_at: Option<String>,
    pub roles: Vec<AgentMessageRole>,
}

impl SessionSummary {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            message_count: 0,
            first_message_at: None,
            last_message_at: None,
            roles: Vec::new(),
        }
    }

    pub fn with_message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    pub fn with_first_message_at(mut self, timestamp: impl Into<String>) -> Self {
        self.first_message_at = Some(timestamp.into());
        self
    }

    pub fn with_last_message_at(mut self, timestamp: impl Into<String>) -> Self {
        self.last_message_at = Some(timestamp.into());
        self
    }

    pub fn with_roles(mut self, roles: Vec<AgentMessageRole>) -> Self {
        self.roles = roles;
        self
    }
}

// ============================================================================
// Message Query Provider - the SPI trait
// ============================================================================

pub trait MessageQueryProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            "provider.message_query.unspecified",
            "message_query",
            "message-query-provider",
            "0.0.0",
            vec![
                "message.query".to_string(),
                "message.count".to_string(),
                "message.list_sessions".to_string(),
                "message.search".to_string(),
            ],
        )
    }

    fn query(&self, query: &MessageQuery) -> KernelResult<MessageQueryResult>;

    fn count(&self, filter: &MessageQueryFilter) -> KernelResult<usize>;

    fn list_sessions(&self) -> KernelResult<Vec<SessionSummary>>;

    fn search_content(&self, search_term: &str, limit: usize) -> KernelResult<MessageQueryResult> {
        let query = MessageQuery::new()
            .with_filter(MessageQueryFilter::new().containing_content(search_term))
            .with_limit(limit);
        self.query(&query)
    }

    fn get_message(&self, message_id: &str) -> KernelResult<AgentMessage>;

    fn list_by_session(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> KernelResult<MessageQueryResult> {
        let mut query = MessageQuery::for_session(session_id);
        if let Some(limit) = limit {
            query = query.with_limit(limit);
        }
        self.query(&query)
    }

    fn list_by_role(
        &self,
        role: AgentMessageRole,
        limit: Option<usize>,
    ) -> KernelResult<MessageQueryResult> {
        let mut query = MessageQuery::new().with_filter(MessageQueryFilter::new().with_role(role));
        if let Some(limit) = limit {
            query = query.with_limit(limit);
        }
        self.query(&query)
    }

    fn health(&self) -> ProviderHealth;
}
