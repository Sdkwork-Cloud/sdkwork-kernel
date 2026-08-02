use std::collections::BTreeMap;

use codex_app_server_protocol::{
    SessionSource as CodexSessionSource, Thread, ThreadActiveFlag, ThreadItem, ThreadItemEntry,
    ThreadStatus, ThreadTurnsListResponse, UserInput,
};
use sdkwork_agent_kernel::{
    AgentMessage, AgentMessageRole, AgentPart, AgentSession, KernelError, KernelEventRedaction,
    KernelResult, SessionActivityEvidenceKind, SessionActivityInteractionHint,
    SessionActivitySnapshot, SessionActivityState, SessionKind, SessionSource,
};
use sdkwork_agent_provider_core::{
    finalize_provider_session_snapshot, normalize_provider_session_path, now_iso,
    session_activity_from_provider_observation, MessageAdapter, ProviderSessionActivityAdapter,
    SessionAdapter, DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL,
};
use sdkwork_agent_provider_spi::{
    SdkRuntimeMessagePart, SdkRuntimeMessageRecord, SdkRuntimeSessionRecord,
};
use serde::Serialize;
use serde_json::Value;

const CODEX_PROVIDER_ID: &str = "codex";
const RAW_ITEM_SCHEMA: &str = "codex.app-server.v2.ThreadItem";

#[derive(Debug, Clone, PartialEq)]
pub struct CodexSessionRecord {
    pub session: SdkRuntimeSessionRecord,
    pub provider_thread: Thread,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexSessionPage {
    pub data: Vec<CodexSessionRecord>,
    pub next_cursor: Option<String>,
    pub backwards_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexMessageRecord {
    pub message: SdkRuntimeMessageRecord,
    pub provider_item: ThreadItemEntry,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexMessagePage {
    pub data: Vec<CodexMessageRecord>,
    pub next_cursor: Option<String>,
    pub backwards_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexThreadActivityObservation {
    pub provider_session_id: String,
    pub status: ThreadStatus,
    pub observed_at: String,
}

impl CodexThreadActivityObservation {
    pub fn from_protocol(provider_session_id: String, status: ThreadStatus) -> Self {
        Self {
            provider_session_id,
            status,
            observed_at: now_iso(),
        }
    }
}

#[derive(Debug, Default)]
pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    fn convert_thread(thread: &Thread) -> KernelResult<AgentSession> {
        let record = provider_session_record(thread)?;
        let mut session = AgentSession::new(&record.provider_session_id);
        session.source = map_session_source(&thread.source);
        session.kind = if thread.parent_thread_id.is_some()
            || matches!(thread.source, CodexSessionSource::SubAgent(_))
        {
            SessionKind::Subagent
        } else if thread.ephemeral {
            SessionKind::Ephemeral
        } else {
            SessionKind::Main
        };
        session.parent_session_id = record.parent_provider_session_id.clone();
        session.forked_from_id = thread.forked_from_id.clone();
        session.title = record.title.clone();
        session.summary = record.summary.clone();
        session.preview = record.preview.clone();
        session.model = record.model.clone();
        session.model_provider = record.model_provider.clone();
        session.cwd = record.cwd.clone();
        if let Some(cwd) = record.cwd.clone() {
            session.workspace_roots.push(cwd);
        }
        session.created_at = record.created_at.clone();
        session.updated_at = record.updated_at.clone();
        session.archived_at = record.archived_at.clone();
        session.agent_nickname = thread.agent_nickname.clone();
        session.agent_role = thread.agent_role.clone();
        session.message_count = record.message_count;
        session.tool_call_count = record.tool_call_count;
        session.compression_count = thread
            .turns
            .iter()
            .flat_map(|turn| &turn.items)
            .filter(|item| matches!(item, ThreadItem::ContextCompaction { .. }))
            .count()
            .try_into()
            .unwrap_or(u32::MAX);
        session.change_summary.additions = record.additions;
        session.change_summary.deletions = record.deletions;
        session.change_summary.files_changed = record.files_changed;
        session.metadata.extend(
            record
                .metadata
                .into_iter()
                .map(|(key, value)| (key, provider_value_string(value))),
        );

        let observation =
            CodexThreadActivityObservation::from_protocol(thread.id.clone(), thread.status.clone());
        session.apply_activity(CodexAdapter::new().to_session_activity(&observation)?)?;
        finalize_provider_session_snapshot(CODEX_PROVIDER_ID, session)
    }
}

fn provider_session_record(thread: &Thread) -> KernelResult<SdkRuntimeSessionRecord> {
    let message_count = thread
        .turns
        .iter()
        .flat_map(|turn| &turn.items)
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    let tool_call_count = thread
        .turns
        .iter()
        .flat_map(|turn| &turn.items)
        .filter(|item| is_tool_item(item))
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    let (additions, deletions, files_changed) = thread_change_summary(thread);
    let mut metadata = BTreeMap::from([
        (
            "codex.thread_id".to_string(),
            Value::String(thread.id.clone()),
        ),
        (
            "codex.session_id".to_string(),
            Value::String(thread.session_id.clone()),
        ),
        ("codex.ephemeral".to_string(), Value::Bool(thread.ephemeral)),
        (
            "codex.history_mode".to_string(),
            Value::String(provider_scalar(&thread.history_mode)),
        ),
        (
            "codex.cli_version".to_string(),
            Value::String(thread.cli_version.clone()),
        ),
        (
            "codex.source".to_string(),
            Value::String(provider_scalar(&thread.source)),
        ),
        (
            "codex.status".to_string(),
            Value::String(provider_scalar(&thread.status)),
        ),
        (
            "codex.turn_count".to_string(),
            serde_json::json!(thread.turns.len()),
        ),
    ]);
    insert_metadata_string(
        &mut metadata,
        "codex.recency_at",
        thread.recency_at.and_then(epoch_seconds_to_rfc3339),
    );
    if let Some(section) = &thread.section {
        insert_metadata_string(&mut metadata, "codex.section.id", Some(section.id.clone()));
        insert_metadata_string(
            &mut metadata,
            "codex.section.name",
            Some(section.name.clone()),
        );
    }
    insert_metadata_string(
        &mut metadata,
        "codex.section_entered_at",
        thread.section_entered_at.and_then(epoch_seconds_to_rfc3339),
    );
    if let Some(can_accept_direct_input) = thread.can_accept_direct_input {
        metadata.insert(
            "codex.can_accept_direct_input".to_string(),
            Value::Bool(can_accept_direct_input),
        );
    }
    if let Some(thread_source) = &thread.thread_source {
        metadata.insert(
            "codex.thread_source".to_string(),
            Value::String(provider_scalar(thread_source)),
        );
    }
    if let Some(git_info) = &thread.git_info {
        insert_metadata_string(&mut metadata, "codex.git.sha", git_info.sha.clone());
        insert_metadata_string(&mut metadata, "codex.git.branch", git_info.branch.clone());
        insert_metadata_string(
            &mut metadata,
            "codex.git.origin_url",
            git_info.origin_url.clone(),
        );
    }

    SdkRuntimeSessionRecord {
        provider_session_id: thread.id.clone(),
        parent_provider_session_id: thread.parent_thread_id.clone(),
        title: thread
            .name
            .clone()
            .or_else(|| non_empty(&thread.preview).map(str::to_string)),
        summary: None,
        preview: non_empty(&thread.preview).map(str::to_string),
        // Normalize to the SDKWork provider session path form (forward slashes,
        // lowercase drive letter) so project cwd selectors match regardless of
        // the native path separator the app-server stored.
        cwd: Some(normalize_provider_session_path(&thread.cwd.display().to_string())),
        created_at: epoch_seconds_to_rfc3339(thread.created_at),
        updated_at: epoch_seconds_to_rfc3339(thread.updated_at),
        archived_at: None,
        model: None,
        model_provider: non_empty(&thread.model_provider).map(str::to_string),
        message_count,
        tool_call_count,
        input_tokens: 0,
        output_tokens: 0,
        cached_tokens: 0,
        reasoning_tokens: 0,
        cost_cents: None,
        additions,
        deletions,
        files_changed,
        metadata,
    }
    .validated(CODEX_PROVIDER_ID)
}

impl SessionAdapter for CodexAdapter {
    type ExternalSession = Thread;

    fn to_agent_session(&self, external: &Self::ExternalSession) -> KernelResult<AgentSession> {
        Self::convert_thread(external)
    }
}

impl ProviderSessionActivityAdapter for CodexAdapter {
    type ExternalActivity = CodexThreadActivityObservation;

    fn to_session_activity(
        &self,
        external: &Self::ExternalActivity,
    ) -> KernelResult<SessionActivitySnapshot> {
        let (state, interaction_hint) = match &external.status {
            ThreadStatus::NotLoaded => {
                return Ok(SessionActivitySnapshot::unsupported_with_evidence(
                    &external.provider_session_id,
                    SessionActivityEvidenceKind::ProviderStatus,
                    external.observed_at.clone(),
                ));
            }
            ThreadStatus::Idle => (SessionActivityState::Idle, None),
            ThreadStatus::SystemError => (SessionActivityState::Failed, None),
            ThreadStatus::Active { active_flags } => {
                if active_flags.contains(&ThreadActiveFlag::WaitingOnApproval) {
                    (
                        SessionActivityState::Waiting,
                        Some(SessionActivityInteractionHint::ApprovalRequired),
                    )
                } else if active_flags.contains(&ThreadActiveFlag::WaitingOnUserInput) {
                    (
                        SessionActivityState::Waiting,
                        Some(SessionActivityInteractionHint::UserInputRequired),
                    )
                } else {
                    (SessionActivityState::Working, None)
                }
            }
        };

        session_activity_from_provider_observation(
            &external.provider_session_id,
            state,
            SessionActivityEvidenceKind::ProviderStatus,
            interaction_hint,
            &external.observed_at,
            DEFAULT_PROVIDER_SESSION_ACTIVITY_TTL,
        )
    }
}

#[derive(Debug, Default)]
pub struct CodexMessageAdapter;

impl CodexMessageAdapter {
    pub fn new() -> Self {
        Self
    }

    fn convert_item(entry: &ThreadItemEntry) -> KernelResult<AgentMessage> {
        let item = &entry.item;
        let item_id = item.id();
        let item_type = item_type(item);
        let parts = message_parts(item)?;

        let mut message = AgentMessage::new(item_id, item_role(item), parts)
            .with_metadata("codex.turn_id", &entry.turn_id)
            .with_metadata("codex.item_type", item_type)
            .with_metadata("codex.item_id", item_id);
        if item_is_untrusted(item) {
            message = message.mark_untrusted();
        }
        Ok(message)
    }
}

impl MessageAdapter for CodexMessageAdapter {
    type ExternalMessage = ThreadItemEntry;

    fn to_agent_message(&self, external: &Self::ExternalMessage) -> KernelResult<AgentMessage> {
        Self::convert_item(external)
    }
}

pub fn map_thread_page(
    response: codex_app_server_protocol::ThreadListResponse,
) -> KernelResult<CodexSessionPage> {
    let data = response
        .data
        .into_iter()
        .map(|provider_thread| {
            let session = provider_session_record(&provider_thread)?;
            Ok(CodexSessionRecord {
                session,
                provider_thread,
            })
        })
        .collect::<KernelResult<Vec<_>>>()?;
    Ok(CodexSessionPage {
        data,
        next_cursor: response.next_cursor,
        backwards_cursor: response.backwards_cursor,
    })
}

pub fn map_thread_record(thread: Thread) -> KernelResult<CodexSessionRecord> {
    let session = provider_session_record(&thread)?;
    Ok(CodexSessionRecord {
        session,
        provider_thread: thread,
    })
}

pub fn map_item_page(
    thread_id: &str,
    response: codex_app_server_protocol::ThreadItemsListResponse,
) -> KernelResult<CodexMessagePage> {
    let data = response
        .data
        .into_iter()
        .map(|provider_item| {
            let message = provider_message_record(thread_id, &provider_item)?;
            Ok(CodexMessageRecord {
                message,
                provider_item,
            })
        })
        .collect::<KernelResult<Vec<_>>>()?;
    Ok(CodexMessagePage {
        data,
        next_cursor: response.next_cursor,
        backwards_cursor: response.backwards_cursor,
    })
}

pub fn map_turn_page(
    thread_id: &str,
    response: ThreadTurnsListResponse,
) -> KernelResult<CodexMessagePage> {
    let data = response
        .data
        .into_iter()
        .flat_map(|turn| {
            let turn_id = turn.id;
            turn.items.into_iter().map(move |item| ThreadItemEntry {
                turn_id: turn_id.clone(),
                item,
            })
        })
        .map(|provider_item| {
            let message = provider_message_record(thread_id, &provider_item)?;
            Ok(CodexMessageRecord {
                message,
                provider_item,
            })
        })
        .collect::<KernelResult<Vec<_>>>()?;
    Ok(CodexMessagePage {
        data,
        next_cursor: response.next_cursor,
        backwards_cursor: response.backwards_cursor,
    })
}

pub fn normalize_page_limit(limit: &mut Option<u32>) -> KernelResult<()> {
    let requested = limit.unwrap_or(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32);
    let maximum = sdkwork_utils_rust::MAX_LIST_PAGE_SIZE as u32;
    if !(1..=maximum).contains(&requested) {
        return Err(KernelError::validation(format!(
            "Codex page limit must be between 1 and {maximum}"
        )));
    }
    *limit = Some(requested);
    Ok(())
}

fn map_session_source(source: &CodexSessionSource) -> SessionSource {
    match source {
        CodexSessionSource::Cli | CodexSessionSource::Exec => SessionSource::Cli,
        CodexSessionSource::VsCode => SessionSource::Ide,
        CodexSessionSource::AppServer => SessionSource::Api,
        CodexSessionSource::SubAgent(_) => SessionSource::Cli,
        CodexSessionSource::Custom(_) | CodexSessionSource::Unknown => SessionSource::Unknown,
    }
}

fn thread_change_summary(thread: &Thread) -> (u32, u32, u32) {
    let mut total_additions = 0_u32;
    let mut total_deletions = 0_u32;
    let mut files_changed = 0_u32;
    for item in thread.turns.iter().flat_map(|turn| &turn.items) {
        if let ThreadItem::FileChange { changes, .. } = item {
            for change in changes {
                let (additions, deletions) =
                    change
                        .diff
                        .lines()
                        .fold((0_u32, 0_u32), |(additions, deletions), line| {
                            if line.starts_with('+') && !line.starts_with("+++") {
                                (additions.saturating_add(1), deletions)
                            } else if line.starts_with('-') && !line.starts_with("---") {
                                (additions, deletions.saturating_add(1))
                            } else {
                                (additions, deletions)
                            }
                        });
                total_additions = total_additions.saturating_add(additions);
                total_deletions = total_deletions.saturating_add(deletions);
                files_changed = files_changed.saturating_add(1);
            }
        }
    }
    (total_additions, total_deletions, files_changed)
}

fn insert_metadata_string(
    metadata: &mut BTreeMap<String, Value>,
    key: impl Into<String>,
    value: Option<String>,
) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        metadata.insert(key.into(), Value::String(value));
    }
}

fn provider_value_string(value: Value) -> String {
    match value {
        Value::String(value) => value,
        value => value.to_string(),
    }
}

fn epoch_seconds_to_rfc3339(epoch_seconds: i64) -> Option<String> {
    let milliseconds = epoch_seconds.checked_mul(1_000)?;
    sdkwork_utils_rust::from_unix_millis(milliseconds)
        .map(|value| sdkwork_utils_rust::format_datetime(value, None))
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn provider_scalar(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(value)) => value,
        Ok(value) => value.to_string(),
        Err(_) => "unknown".to_string(),
    }
}

fn item_type(item: &ThreadItem) -> &'static str {
    match item {
        ThreadItem::UserMessage { .. } => "user_message",
        ThreadItem::HookPrompt { .. } => "hook_prompt",
        ThreadItem::AgentMessage { .. } => "agent_message",
        ThreadItem::Plan { .. } => "plan",
        ThreadItem::Reasoning { .. } => "reasoning",
        ThreadItem::CommandExecution { .. } => "command_execution",
        ThreadItem::FileChange { .. } => "file_change",
        ThreadItem::McpToolCall { .. } => "mcp_tool_call",
        ThreadItem::DynamicToolCall { .. } => "dynamic_tool_call",
        ThreadItem::CollabAgentToolCall { .. } => "collab_agent_tool_call",
        ThreadItem::SubAgentActivity { .. } => "sub_agent_activity",
        ThreadItem::WebSearch(_) => "web_search",
        ThreadItem::ImageView { .. } => "image_view",
        ThreadItem::Sleep(_) => "sleep",
        ThreadItem::ImageGeneration(_) => "image_generation",
        ThreadItem::EnteredReviewMode { .. } => "entered_review_mode",
        ThreadItem::ExitedReviewMode { .. } => "exited_review_mode",
        ThreadItem::ContextCompaction { .. } => "context_compaction",
    }
}

fn item_role(item: &ThreadItem) -> AgentMessageRole {
    match item {
        ThreadItem::UserMessage { .. } => AgentMessageRole::User,
        ThreadItem::HookPrompt { .. } => AgentMessageRole::System,
        ThreadItem::AgentMessage { .. }
        | ThreadItem::Plan { .. }
        | ThreadItem::Reasoning { .. } => AgentMessageRole::Agent,
        ThreadItem::CommandExecution { .. }
        | ThreadItem::FileChange { .. }
        | ThreadItem::McpToolCall { .. }
        | ThreadItem::DynamicToolCall { .. }
        | ThreadItem::CollabAgentToolCall { .. }
        | ThreadItem::WebSearch(_)
        | ThreadItem::ImageView { .. }
        | ThreadItem::ImageGeneration(_) => AgentMessageRole::Tool,
        ThreadItem::SubAgentActivity { .. }
        | ThreadItem::Sleep(_)
        | ThreadItem::EnteredReviewMode { .. }
        | ThreadItem::ExitedReviewMode { .. }
        | ThreadItem::ContextCompaction { .. } => AgentMessageRole::Adapter,
    }
}

fn is_tool_item(item: &ThreadItem) -> bool {
    matches!(
        item,
        ThreadItem::CommandExecution { .. }
            | ThreadItem::FileChange { .. }
            | ThreadItem::McpToolCall { .. }
            | ThreadItem::DynamicToolCall { .. }
            | ThreadItem::CollabAgentToolCall { .. }
            | ThreadItem::WebSearch(_)
            | ThreadItem::ImageView { .. }
            | ThreadItem::Sleep(_)
            | ThreadItem::ImageGeneration(_)
    )
}

fn item_is_untrusted(item: &ThreadItem) -> bool {
    matches!(
        item,
        ThreadItem::CommandExecution {
            aggregated_output: Some(_),
            ..
        } | ThreadItem::FileChange { .. }
            | ThreadItem::McpToolCall {
                result: Some(_),
                ..
            }
            | ThreadItem::McpToolCall { error: Some(_), .. }
            | ThreadItem::DynamicToolCall {
                content_items: Some(_),
                ..
            }
            | ThreadItem::WebSearch(_)
            | ThreadItem::ImageView { .. }
            | ThreadItem::ImageGeneration(_)
    )
}

fn provider_message_record(
    provider_session_id: &str,
    entry: &ThreadItemEntry,
) -> KernelResult<SdkRuntimeMessageRecord> {
    let item = &entry.item;
    let item_id = item.id();
    let mut metadata = BTreeMap::from([
        (
            "codex.turn_id".to_string(),
            Value::String(entry.turn_id.clone()),
        ),
        (
            "codex.item_type".to_string(),
            Value::String(item_type(item).to_string()),
        ),
        (
            "codex.item_id".to_string(),
            Value::String(item_id.to_string()),
        ),
    ]);
    if item_is_untrusted(item) {
        metadata.insert("sdkwork.provider.untrusted".to_string(), Value::Bool(true));
    }
    let parts = message_parts(item)?
        .into_iter()
        .map(provider_message_part)
        .collect::<KernelResult<Vec<_>>>()?;

    SdkRuntimeMessageRecord {
        provider_message_id: item_id.to_string(),
        provider_session_id: provider_session_id.to_string(),
        parent_provider_message_id: None,
        role: item_role(item).as_str().to_string(),
        parts,
        created_at: None,
        metadata,
    }
    .validated(provider_session_id)
}

fn message_parts(item: &ThreadItem) -> KernelResult<Vec<AgentPart>> {
    let mut parts = render_parts(item);
    parts.push(raw_item_part(item)?);
    Ok(parts)
}

fn provider_message_part(part: AgentPart) -> KernelResult<SdkRuntimeMessagePart> {
    let json = part
        .json
        .map(|json| {
            serde_json::from_str(&json).map_err(|error| {
                KernelError::provider_error(
                    "codex_thread_item_json_invalid",
                    format!("failed to project typed Codex message part JSON: {error}"),
                )
                .with_provider(CODEX_PROVIDER_ID)
            })
        })
        .transpose()?;
    let mut metadata = part
        .metadata
        .into_iter()
        .map(|(key, value)| (key, Value::String(value)))
        .collect::<BTreeMap<_, _>>();
    insert_metadata_string(&mut metadata, "sdkwork.provider.schema", part.schema);
    insert_metadata_string(
        &mut metadata,
        "sdkwork.provider.provenance",
        part.provenance,
    );
    if !matches!(part.redaction_classification, KernelEventRedaction::Unknown) {
        metadata.insert(
            "sdkwork.provider.redaction".to_string(),
            Value::String(redaction_name(&part.redaction_classification).to_string()),
        );
    }

    Ok(SdkRuntimeMessagePart {
        part_id: part.part_id,
        kind: part.kind.as_str().to_string(),
        text: part.text,
        json,
        content_ref: part.content_ref,
        artifact_id: part.artifact_id,
        tool_call_id: part.tool_call_id,
        policy_decision_id: part.policy_decision_id,
        error_code: part.error_code,
        mime_type: part.mime_type,
        name: part.name,
        metadata,
    })
}

fn redaction_name(redaction: &KernelEventRedaction) -> &'static str {
    match redaction {
        KernelEventRedaction::Public => "public",
        KernelEventRedaction::Internal => "internal",
        KernelEventRedaction::TenantSensitive => "tenant_sensitive",
        KernelEventRedaction::PersonalData => "personal_data",
        KernelEventRedaction::Secret => "secret",
        KernelEventRedaction::Regulated => "regulated",
        KernelEventRedaction::Unknown => "unknown",
    }
}

fn render_parts(item: &ThreadItem) -> Vec<AgentPart> {
    let item_id = item.id();
    match item {
        ThreadItem::UserMessage { content, .. } => content
            .iter()
            .enumerate()
            .map(|(index, input)| user_input_part(item_id, index, input))
            .collect(),
        ThreadItem::HookPrompt { fragments, .. } => fragments
            .iter()
            .enumerate()
            .map(|(index, fragment)| {
                AgentPart::text(format!("{item_id}.hook.{index}"), &fragment.text)
                    .from_provider(CODEX_PROVIDER_ID)
                    .with_metadata("codex.hook_run_id", &fragment.hook_run_id)
            })
            .collect(),
        ThreadItem::AgentMessage { text, .. } => {
            vec![provider_text(item_id, "agent_message", text)]
        }
        ThreadItem::Plan { text, .. } => vec![provider_text(item_id, "plan", text)],
        ThreadItem::Reasoning {
            summary, content, ..
        } => summary
            .iter()
            .map(|text| ("reasoning_summary", text))
            .chain(content.iter().map(|text| ("reasoning_content", text)))
            .enumerate()
            .map(|(index, (content_type, text))| {
                provider_text(&format!("{item_id}.{index}"), content_type, text)
            })
            .collect(),
        ThreadItem::CommandExecution {
            command,
            status,
            aggregated_output,
            ..
        } => {
            let mut parts = vec![provider_tool_call(
                item_id,
                "shell_command",
                "command_execution",
                provider_scalar(status),
            )];
            if let Some(output) = aggregated_output {
                parts.push(
                    provider_text(&format!("{item_id}.output"), "tool_output", output)
                        .with_redaction(KernelEventRedaction::TenantSensitive),
                );
            }
            parts[0] = parts[0].clone().with_metadata("codex.command", command);
            parts
        }
        ThreadItem::FileChange { status, .. } => vec![provider_tool_call(
            item_id,
            "apply_patch",
            "file_change",
            provider_scalar(status),
        )],
        ThreadItem::McpToolCall {
            server,
            tool,
            status,
            read_only_hint,
            ..
        } => {
            let mut part =
                provider_tool_call(item_id, tool, "mcp_tool_call", provider_scalar(status))
                    .with_metadata("codex.mcp.server", server);
            if let Some(read_only_hint) = read_only_hint {
                part = part.with_metadata("codex.mcp.read_only_hint", read_only_hint.to_string());
            }
            vec![part]
        }
        ThreadItem::DynamicToolCall { tool, status, .. } => vec![provider_tool_call(
            item_id,
            tool,
            "dynamic_tool_call",
            provider_scalar(status),
        )],
        ThreadItem::CollabAgentToolCall { tool, status, .. } => vec![provider_tool_call(
            item_id,
            &provider_scalar(tool),
            "collab_agent_tool_call",
            provider_scalar(status),
        )],
        ThreadItem::SubAgentActivity {
            kind,
            agent_thread_id,
            agent_path,
            ..
        } => vec![provider_text(
            item_id,
            "sub_agent_activity",
            &format!(
                "{}: {} ({})",
                provider_scalar(kind),
                agent_thread_id,
                agent_path
            ),
        )],
        ThreadItem::WebSearch(item) => vec![provider_tool_call(
            &item.id,
            "web_search",
            "web_search",
            "completed".to_string(),
        )
        .with_metadata("codex.web_search.query", &item.query)],
        ThreadItem::ImageView { path, .. } => {
            vec![
                AgentPart::image_ref(format!("{item_id}.image"), path.to_string(), "image/*")
                    .from_provider(CODEX_PROVIDER_ID),
            ]
        }
        ThreadItem::Sleep(item) => {
            vec![
                provider_tool_call(&item.id, "sleep", "sleep", "completed".to_string())
                    .with_metadata("codex.duration_ms", item.duration_ms.to_string()),
            ]
        }
        ThreadItem::ImageGeneration(item) => {
            let mut parts = vec![provider_tool_call(
                &item.id,
                "image_generation",
                "image_generation",
                item.status.clone(),
            )];
            if !item.result.is_empty() {
                parts.push(
                    AgentPart::image_ref(format!("{}.image", item.id), &item.result, "image/*")
                        .from_provider(CODEX_PROVIDER_ID),
                );
            }
            parts
        }
        ThreadItem::EnteredReviewMode { review, .. } => {
            vec![provider_text(item_id, "entered_review_mode", review)]
        }
        ThreadItem::ExitedReviewMode { review, .. } => {
            vec![provider_text(item_id, "exited_review_mode", review)]
        }
        ThreadItem::ContextCompaction { .. } => vec![provider_text(
            item_id,
            "context_compaction",
            "Codex compacted the thread context.",
        )],
    }
}

fn user_input_part(item_id: &str, index: usize, input: &UserInput) -> AgentPart {
    let part_id = format!("{item_id}.input.{index}");
    match input {
        UserInput::Text { text, .. } => provider_text(&part_id, "input_text", text),
        UserInput::Image { url, .. } => {
            AgentPart::image_ref(part_id, url, "image/*").from_provider(CODEX_PROVIDER_ID)
        }
        UserInput::LocalImage { path, .. } => {
            AgentPart::image_ref(part_id, path.display().to_string(), "image/*")
                .from_provider(CODEX_PROVIDER_ID)
        }
        UserInput::Audio { url } => {
            AgentPart::audio_ref(part_id, url, "audio/*").from_provider(CODEX_PROVIDER_ID)
        }
        UserInput::LocalAudio { path } => {
            AgentPart::audio_ref(part_id, path.display().to_string(), "audio/*")
                .from_provider(CODEX_PROVIDER_ID)
        }
        UserInput::Skill { name, path } => AgentPart::file_ref(
            part_id,
            path.display().to_string(),
            "application/vnd.sdkwork.skill",
        )
        .with_name(name)
        .from_provider(CODEX_PROVIDER_ID),
        UserInput::Mention { name, path } => {
            provider_text(&part_id, "mention", &format!("@{name} ({path})"))
        }
    }
}

fn provider_text(part_id: &str, content_type: &str, text: &str) -> AgentPart {
    AgentPart::text(part_id, text)
        .from_provider(CODEX_PROVIDER_ID)
        .with_metadata("codex.content_type", content_type)
}

fn provider_tool_call(item_id: &str, name: &str, content_type: &str, status: String) -> AgentPart {
    AgentPart::tool_call_ref(format!("{item_id}.tool"), item_id)
        .with_name(name)
        .from_provider(CODEX_PROVIDER_ID)
        .with_metadata("codex.content_type", content_type)
        .with_metadata("codex.status", status)
}

fn raw_item_part(item: &ThreadItem) -> KernelResult<AgentPart> {
    let raw = serde_json::to_string(item).map_err(|error| {
        KernelError::provider_error(
            "codex_thread_item_serialization_failed",
            format!("failed to preserve typed Codex thread item: {error}"),
        )
        .with_provider(CODEX_PROVIDER_ID)
    })?;
    Ok(AgentPart::json(format!("{}.raw", item.id()), raw)
        .with_name("codex.thread_item")
        .with_schema(RAW_ITEM_SCHEMA)
        .from_provider(CODEX_PROVIDER_ID)
        .with_redaction(KernelEventRedaction::TenantSensitive)
        .with_metadata("codex.content_type", "raw_provider_item"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::ThreadItemsListResponse;

    fn sample_thread() -> Thread {
        serde_json::from_value(serde_json::json!({
            "id": "0198-thread",
            "extra": null,
            "sessionId": "0198-session",
            "forkedFromId": "0197-thread",
            "parentThreadId": "0196-thread",
            "preview": "Review the provider",
            "ephemeral": false,
            "section": {"id": "section-pinned", "name": "Pinned"},
            "sectionEnteredAt": 1_752_000_100,
            "historyMode": "paginated",
            "modelProvider": "openai",
            "createdAt": 1_752_000_000,
            "updatedAt": 1_752_000_200,
            "recencyAt": 1_752_000_300,
            "status": {"type": "active", "activeFlags": ["waitingOnApproval"]},
            "path": "C:/Users/test/.codex/sessions/thread.jsonl",
            "cwd": "E:/workspace/project",
            "cliVersion": "0.99.0",
            "source": "vscode",
            "canAcceptDirectInput": true,
            "threadSource": "subagent",
            "agentNickname": "reviewer",
            "agentRole": "code-review",
            "gitInfo": {"sha": "abc123", "branch": "main", "originUrl": "https://example.invalid/repo.git"},
            "name": "Provider review",
            "turns": []
        }))
        .expect("thread fixture must match upstream protocol")
    }

    #[test]
    fn maps_every_stable_thread_field_and_preserves_typed_thread() {
        let provider_thread = sample_thread();
        let expected_cwd = normalize_provider_session_path(
            &provider_thread.cwd.display().to_string(),
        );
        let record = map_thread_record(provider_thread.clone()).expect("thread mapping");
        let session = record.session;

        assert_eq!(record.provider_thread, provider_thread);
        assert_eq!(session.provider_session_id, "0198-thread");
        assert_eq!(
            session.parent_provider_session_id.as_deref(),
            Some("0196-thread")
        );
        assert_eq!(session.title.as_deref(), Some("Provider review"));
        assert_eq!(session.preview.as_deref(), Some("Review the provider"));
        assert_eq!(session.model_provider.as_deref(), Some("openai"));
        assert_eq!(session.cwd.as_deref(), Some(expected_cwd.as_str()));
        assert_eq!(
            session
                .metadata
                .get("codex.session_id")
                .and_then(Value::as_str),
            Some("0198-session")
        );
        assert_eq!(
            session
                .metadata
                .get("codex.section.name")
                .and_then(Value::as_str),
            Some("Pinned")
        );
        assert_eq!(
            session
                .metadata
                .get("codex.history_mode")
                .and_then(Value::as_str),
            Some("paginated")
        );
        assert_eq!(
            session
                .metadata
                .get("codex.cli_version")
                .and_then(Value::as_str),
            Some("0.99.0")
        );
        assert_eq!(
            session
                .metadata
                .get("codex.can_accept_direct_input")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            session
                .metadata
                .get("codex.thread_source")
                .and_then(Value::as_str),
            Some("subagent")
        );
        assert_eq!(
            session
                .metadata
                .get("codex.git.sha")
                .and_then(Value::as_str),
            Some("abc123")
        );
        assert!(!session.metadata.contains_key("codex.path"));
        let serialized = serde_json::to_value(&session).expect("serialize provider record");
        assert!(serialized.get("session_id").is_none());
        assert!(serialized.get("parent_session_id").is_none());
    }

    #[test]
    fn maps_tool_item_without_losing_upstream_fields() {
        let response: ThreadItemsListResponse = serde_json::from_value(serde_json::json!({
            "data": [{
                "turnId": "turn-1",
                "item": {
                    "type": "mcpToolCall",
                    "id": "item-1",
                    "server": "docs",
                    "tool": "search",
                    "status": "completed",
                    "arguments": {"query": "Codex"},
                    "appContext": null,
                    "pluginId": null,
                    "readOnlyHint": true,
                    "result": null,
                    "error": null,
                    "durationMs": 42
                }
            }],
            "nextCursor": "next-1",
            "backwardsCursor": "previous-1"
        }))
        .expect("item fixture must match upstream protocol");
        let page = map_item_page("thread-1", response).expect("item page mapping");
        let record = &page.data[0];

        assert_eq!(page.next_cursor.as_deref(), Some("next-1"));
        assert_eq!(record.message.provider_message_id, "item-1");
        assert_eq!(record.message.provider_session_id, "thread-1");
        assert_eq!(record.message.role, "tool");
        assert_eq!(
            record
                .message
                .metadata
                .get("codex.turn_id")
                .and_then(Value::as_str),
            Some("turn-1")
        );
        assert_eq!(
            record.message.parts[0]
                .metadata
                .get("codex.mcp.read_only_hint")
                .and_then(Value::as_str),
            Some("true")
        );
        let raw = record
            .message
            .parts
            .iter()
            .find(|part| {
                part.name.as_deref() == Some("codex.thread_item")
                    && part
                        .metadata
                        .get("sdkwork.provider.schema")
                        .and_then(Value::as_str)
                        == Some(RAW_ITEM_SCHEMA)
            })
            .and_then(|part| part.json.as_ref())
            .expect("raw typed item");
        assert_eq!(raw.get("readOnlyHint").and_then(Value::as_bool), Some(true));
        assert_eq!(
            record
                .message
                .parts
                .iter()
                .find(|part| part.name.as_deref() == Some("codex.thread_item"))
                .and_then(|part| part.metadata.get("sdkwork.provider.redaction"))
                .and_then(Value::as_str),
            Some("tenant_sensitive")
        );
        assert_eq!(record.provider_item.item.id(), "item-1");
    }

    #[test]
    fn maps_every_upstream_thread_item_variant() {
        let fixtures = serde_json::json!([
            {"type":"userMessage","id":"user","clientId":null,"content":[]},
            {"type":"hookPrompt","id":"hook","fragments":[]},
            {"type":"agentMessage","id":"agent","text":"answer"},
            {"type":"plan","id":"plan","text":"plan"},
            {"type":"reasoning","id":"reasoning","summary":["summary"],"content":["detail"]},
            {"type":"commandExecution","id":"command","command":"pwd","cwd":"E:/workspace","processId":null,"status":"completed","commandActions":[],"aggregatedOutput":null,"exitCode":0,"durationMs":1},
            {"type":"fileChange","id":"file","changes":[],"status":"completed"},
            {"type":"mcpToolCall","id":"mcp","server":"docs","tool":"search","status":"completed","arguments":{},"appContext":null,"pluginId":null,"readOnlyHint":true,"result":null,"error":null,"durationMs":1},
            {"type":"dynamicToolCall","id":"dynamic","namespace":null,"tool":"lookup","arguments":{},"status":"completed","contentItems":null,"success":true,"durationMs":1},
            {"type":"collabAgentToolCall","id":"collab","tool":"wait","status":"completed","senderThreadId":"sender","receiverThreadIds":[],"prompt":null,"model":null,"reasoningEffort":null,"agentsStates":{}},
            {"type":"subAgentActivity","id":"subagent","kind":"started","agentThreadId":"child","agentPath":"/root/child"},
            {"type":"webSearch","id":"web","query":"Codex","action":null,"results":null},
            {"type":"imageView","id":"view","path":"E:/workspace/image.png"},
            {"type":"sleep","id":"sleep","durationMs":10},
            {"type":"imageGeneration","id":"image","status":"completed","revisedPrompt":null,"result":"https://example.invalid/image.png"},
            {"type":"enteredReviewMode","id":"review-in","review":"review"},
            {"type":"exitedReviewMode","id":"review-out","review":"review"},
            {"type":"contextCompaction","id":"compaction"}
        ]);
        let expected_types = [
            "user_message",
            "hook_prompt",
            "agent_message",
            "plan",
            "reasoning",
            "command_execution",
            "file_change",
            "mcp_tool_call",
            "dynamic_tool_call",
            "collab_agent_tool_call",
            "sub_agent_activity",
            "web_search",
            "image_view",
            "sleep",
            "image_generation",
            "entered_review_mode",
            "exited_review_mode",
            "context_compaction",
        ];
        let items: Vec<ThreadItem> =
            serde_json::from_value(fixtures).expect("fixtures must match every upstream variant");

        assert_eq!(items.len(), expected_types.len());
        for (item, expected_type) in items.into_iter().zip(expected_types) {
            let item_id = item.id().to_string();
            let message = CodexMessageAdapter::new()
                .to_agent_message(&ThreadItemEntry {
                    turn_id: "turn-all".to_string(),
                    item,
                })
                .expect("every upstream item must map");
            assert_eq!(message.message_id, item_id);
            assert_eq!(
                message.metadata_value("codex.item_type"),
                Some(expected_type)
            );
            assert!(message
                .parts
                .iter()
                .any(|part| part.schema.as_deref() == Some(RAW_ITEM_SCHEMA)));
        }
    }

    #[test]
    fn rejects_out_of_contract_page_limits() {
        let mut missing = None;
        normalize_page_limit(&mut missing).expect("default limit");
        assert_eq!(
            missing,
            Some(sdkwork_utils_rust::DEFAULT_LIST_PAGE_SIZE as u32)
        );

        for invalid in [0, sdkwork_utils_rust::MAX_LIST_PAGE_SIZE as u32 + 1] {
            let mut limit = Some(invalid);
            assert!(normalize_page_limit(&mut limit).is_err());
        }
    }
}
