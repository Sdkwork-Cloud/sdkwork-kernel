use crate::domain::{AgentBusinessRecord, AgentBusinessStatus, AgentVisibility};
use crate::ports::{AgentAuditSink, AgentListQuery, AgentRepository};
use sdkwork_agent_kernel::{
    AgentManifest, KernelError, KernelEvent, KernelEventSeverity, KernelEventSource, KernelResult,
};
use sdkwork_code_kernel::CodeTaskIntent;
use serde::{Deserialize, Serialize};
#[cfg(feature = "postgres-sync")]
use postgres::{Client, NoTls, Row};
#[cfg(feature = "postgres-sync")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

pub const SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID: &str =
    "SELECT id, uuid, tenant_id, organization_id, owner_user_id, agent_id, code, display_name, description, manifest_json, default_code_task_intent_json, status, visibility, tags_json, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at, version FROM ai_agent_business WHERE tenant_id = $1 AND agent_id = $2 LIMIT 1";
pub const SQL_INSERT_AGENT_BUSINESS: &str =
    "INSERT INTO ai_agent_business (id, uuid, tenant_id, organization_id, owner_user_id, agent_id, code, display_name, description, manifest_json, default_code_task_intent_json, status, visibility, tags_json, created_at, updated_at, deleted_at, version) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)";
pub const SQL_UPDATE_AGENT_BUSINESS: &str =
    "UPDATE ai_agent_business SET organization_id = $1, owner_user_id = $2, code = $3, display_name = $4, description = $5, manifest_json = $6, default_code_task_intent_json = $7, status = $8, visibility = $9, tags_json = $10, updated_at = $11, deleted_at = $12, version = $13 WHERE tenant_id = $14 AND agent_id = $15";
pub const SQL_LIST_AGENT_BUSINESS: &str =
    "SELECT id, uuid, tenant_id, organization_id, owner_user_id, agent_id, code, display_name, description, manifest_json, default_code_task_intent_json, status, visibility, tags_json, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at, version FROM ai_agent_business WHERE tenant_id = $1 ORDER BY updated_at DESC";
pub const SQL_INSERT_AUDIT_EVENT: &str =
    "INSERT INTO ai_agent_business_audit_event (uuid, tenant_id, organization_id, agent_business_id, agent_id, action, subject_id, subject_tenant_id, request_id, trace_id, payload_json, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
pub const SQL_NEXT_AGENT_BUSINESS_ID: &str =
    "SELECT nextval(pg_get_serial_sequence('ai_agent_business', 'id')) AS next_id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBusinessRow {
    pub id: u64,
    pub uuid: String,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub owner_user_id: u64,
    pub agent_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub manifest_json: String,
    pub default_code_task_intent_json: Option<String>,
    pub status: i16,
    pub visibility: i16,
    pub tags_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub version: u64,
}

impl AgentBusinessRow {
    pub fn from_record(record: &AgentBusinessRecord) -> KernelResult<Self> {
        Ok(Self {
            id: record.id,
            uuid: build_agent_business_uuid(record.tenant_id, &record.agent_id),
            tenant_id: record.tenant_id,
            organization_id: record.organization_id,
            owner_user_id: record.owner_user_id,
            agent_id: record.agent_id.clone(),
            code: record.code.clone(),
            display_name: record.display_name.clone(),
            description: record.description.clone(),
            manifest_json: manifest_to_json(&record.manifest)?,
            default_code_task_intent_json: intent_to_json(record.default_code_task_intent.as_ref())?,
            status: record.status.as_db_code(),
            visibility: record.visibility.as_db_code(),
            tags_json: tags_to_json(&record.tags)?,
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
            version: record.version,
        })
    }

    pub fn into_record(self) -> KernelResult<AgentBusinessRecord> {
        Ok(AgentBusinessRecord {
            id: self.id,
            agent_id: self.agent_id,
            tenant_id: self.tenant_id,
            organization_id: self.organization_id,
            owner_user_id: self.owner_user_id,
            code: self.code,
            display_name: self.display_name,
            description: self.description,
            manifest: manifest_from_json(&self.manifest_json)?,
            default_code_task_intent: intent_from_json(self.default_code_task_intent_json.as_deref())?,
            status: AgentBusinessStatus::from_db_code(self.status).ok_or_else(|| {
                KernelError::validation(format!("invalid db status code: {}", self.status))
            })?,
            visibility: AgentVisibility::from_db_code(self.visibility).ok_or_else(|| {
                KernelError::validation(format!("invalid db visibility code: {}", self.visibility))
            })?,
            tags: tags_from_json(&self.tags_json)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
            version: self.version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAuditEventRow {
    pub id: u64,
    pub uuid: String,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub agent_business_id: u64,
    pub agent_id: String,
    pub action: String,
    pub subject_id: String,
    pub subject_tenant_id: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub payload_json: String,
    pub created_at: String,
}

impl AgentAuditEventRow {
    pub fn from_kernel_event(
        event: &KernelEvent,
        tenant_id: u64,
        organization_id: u64,
        agent_business_id: u64,
        agent_id: &str,
    ) -> KernelResult<Self> {
        let occurred_at = event
            .occurred_at
            .clone()
            .ok_or_else(|| KernelError::validation("audit event occurred_at is required"))?;

        Ok(Self {
            id: 0,
            uuid: format!("audit_{}_{}", tenant_id, event.event_id),
            tenant_id,
            organization_id,
            agent_business_id,
            agent_id: agent_id.to_string(),
            action: event
                .event_type
                .rsplit('.')
                .next()
                .unwrap_or("unknown")
                .to_string(),
            subject_id: event
                .correlation_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            subject_tenant_id: "unknown".to_string(),
            request_id: None,
            trace_id: event.trace_context.as_ref().map(|trace| trace.trace_id.clone()),
            payload_json: serde_json::to_string(&AuditPayloadSnapshot {
                event_id: event.event_id.clone(),
                event_type: event.event_type.clone(),
                severity: severity_as_str(event.severity).to_string(),
                source: source_as_str(event.source).to_string(),
                payload: event.payload.clone(),
            })
            .map_err(|error| KernelError::validation(format!("invalid audit payload json: {error}")))?,
            created_at: occurred_at,
        })
    }

    pub fn into_kernel_event(self) -> KernelResult<KernelEvent> {
        let payload: AuditPayloadSnapshot = serde_json::from_str(self.payload_json.as_str())
            .map_err(|error| KernelError::validation(format!("invalid audit payload json: {error}")))?;
        Ok(
            KernelEvent::new(
                payload.event_id,
                payload.event_type,
                severity_from_str(payload.severity.as_str())?,
                payload.payload,
            )
            .from_source(source_from_str(payload.source.as_str())?)
            .occurred_at(self.created_at),
        )
    }
}

pub trait PostgresAgentRepositoryAdapter {
    fn next_id(&mut self) -> u64;
    fn insert_row(&mut self, row: AgentBusinessRow) -> KernelResult<()>;
    fn update_row(&mut self, row: AgentBusinessRow) -> KernelResult<()>;
    fn get_row(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRow>;
    fn list_rows(&self, query: &AgentListQuery) -> Vec<AgentBusinessRow>;
}

pub struct PostgresAgentRepository<A>
where
    A: PostgresAgentRepositoryAdapter,
{
    adapter: A,
}

impl<A> PostgresAgentRepository<A>
where
    A: PostgresAgentRepositoryAdapter,
{
    pub fn new(adapter: A) -> Self {
        Self { adapter }
    }
}

impl<A> AgentRepository for PostgresAgentRepository<A>
where
    A: PostgresAgentRepositoryAdapter,
{
    fn next_id(&mut self) -> u64 {
        self.adapter.next_id()
    }

    fn insert(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        self.adapter.insert_row(AgentBusinessRow::from_record(&record)?)
    }

    fn update(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        self.adapter.update_row(AgentBusinessRow::from_record(&record)?)
    }

    fn get(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRecord> {
        self.adapter
            .get_row(tenant_id, agent_id)
            .and_then(|row| row.into_record().ok())
    }

    fn list(&self, query: &AgentListQuery) -> Vec<AgentBusinessRecord> {
        self.adapter
            .list_rows(query)
            .into_iter()
            .filter_map(|row| row.into_record().ok())
            .collect()
    }
}

pub trait PostgresAuditAdapter {
    fn insert_audit_row(&mut self, row: AgentAuditEventRow) -> KernelResult<()>;
}

#[cfg(feature = "postgres-sync")]
pub struct SyncPostgresAdapter {
    client: Mutex<Client>,
    fallback_next_id: AtomicU64,
}

#[cfg(feature = "postgres-sync")]
impl SyncPostgresAdapter {
    pub fn connect(connection_uri: &str) -> KernelResult<Self> {
        let client = Client::connect(connection_uri, NoTls).map_err(map_postgres_error)?;
        Ok(Self {
            client: Mutex::new(client),
            fallback_next_id: AtomicU64::new(1),
        })
    }

    pub fn with_client(client: Client) -> Self {
        Self {
            client: Mutex::new(client),
            fallback_next_id: AtomicU64::new(1),
        }
    }

    fn with_locked_client<T>(
        &self,
        action: impl FnOnce(&mut Client) -> KernelResult<T>,
    ) -> KernelResult<T> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| KernelError::provider_error("postgres_lock_error", "postgres mutex poisoned"))?;
        action(&mut client)
    }
}

#[cfg(feature = "postgres-sync")]
impl PostgresAgentRepositoryAdapter for SyncPostgresAdapter {
    fn next_id(&mut self) -> u64 {
        let next_id = self.with_locked_client(|client| {
            let row = client
                .query_one(SQL_NEXT_AGENT_BUSINESS_ID, &[])
                .map_err(map_postgres_error)?;
            let value: i64 = row.try_get("next_id").map_err(map_postgres_error)?;
            int64_to_u64(value, "next_id")
        });

        match next_id {
            Ok(value) if value > 0 => value,
            _ => self.fallback_next_id.fetch_add(1, Ordering::SeqCst),
        }
    }

    fn insert_row(&mut self, row: AgentBusinessRow) -> KernelResult<()> {
        let id = u64_to_i64(row.id, "id")?;
        let tenant_id = u64_to_i64(row.tenant_id, "tenant_id")?;
        let organization_id = u64_to_i64(row.organization_id, "organization_id")?;
        let owner_user_id = u64_to_i64(row.owner_user_id, "owner_user_id")?;
        let version = u64_to_i64(row.version, "version")?;

        self.with_locked_client(|client| {
            client
                .execute(
                    SQL_INSERT_AGENT_BUSINESS,
                    &[
                        &id,
                        &row.uuid,
                        &tenant_id,
                        &organization_id,
                        &owner_user_id,
                        &row.agent_id,
                        &row.code,
                        &row.display_name,
                        &row.description,
                        &row.manifest_json,
                        &row.default_code_task_intent_json,
                        &row.status,
                        &row.visibility,
                        &row.tags_json,
                        &row.created_at,
                        &row.updated_at,
                        &row.deleted_at,
                        &version,
                    ],
                )
                .map_err(map_postgres_error)?;
            Ok(())
        })
    }

    fn update_row(&mut self, row: AgentBusinessRow) -> KernelResult<()> {
        let organization_id = u64_to_i64(row.organization_id, "organization_id")?;
        let owner_user_id = u64_to_i64(row.owner_user_id, "owner_user_id")?;
        let version = u64_to_i64(row.version, "version")?;
        let tenant_id = u64_to_i64(row.tenant_id, "tenant_id")?;

        self.with_locked_client(|client| {
            let updated_rows = client
                .execute(
                    SQL_UPDATE_AGENT_BUSINESS,
                    &[
                        &organization_id,
                        &owner_user_id,
                        &row.code,
                        &row.display_name,
                        &row.description,
                        &row.manifest_json,
                        &row.default_code_task_intent_json,
                        &row.status,
                        &row.visibility,
                        &row.tags_json,
                        &row.updated_at,
                        &row.deleted_at,
                        &version,
                        &tenant_id,
                        &row.agent_id,
                    ],
                )
                .map_err(map_postgres_error)?;

            if updated_rows == 0 {
                return Err(KernelError::validation("agent not found"));
            }
            Ok(())
        })
    }

    fn get_row(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRow> {
        let tenant_id = u64_to_i64(tenant_id, "tenant_id").ok()?;
        self.with_locked_client(|client| {
            let row = client
                .query_opt(SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID, &[&tenant_id, &agent_id])
                .map_err(map_postgres_error)?;
            row.map(pg_row_to_agent_business_row).transpose()
        })
        .ok()
        .flatten()
    }

    fn list_rows(&self, query: &AgentListQuery) -> Vec<AgentBusinessRow> {
        let tenant_id = match u64_to_i64(query.tenant_id, "tenant_id") {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };

        self.with_locked_client(|client| {
            let rows = client
                .query(SQL_LIST_AGENT_BUSINESS, &[&tenant_id])
                .map_err(map_postgres_error)?;

            let mut mapped_rows = Vec::with_capacity(rows.len());
            for row in rows {
                mapped_rows.push(pg_row_to_agent_business_row(row)?);
            }
            Ok(mapped_rows)
        })
        .map(|rows| {
            rows.into_iter()
                .filter(|row| {
                    if let Some(organization_id) = query.organization_id {
                        row.organization_id == organization_id
                    } else {
                        true
                    }
                })
                .filter(|row| {
                    if let Some(owner_user_id) = query.owner_user_id {
                        row.owner_user_id == owner_user_id
                    } else {
                        true
                    }
                })
                .filter(|row| {
                    if query.include_deleted {
                        true
                    } else {
                        row.status != AgentBusinessStatus::Deleted.as_db_code()
                            && row.deleted_at.is_none()
                    }
                })
                .collect()
        })
        .unwrap_or_default()
    }
}

#[cfg(feature = "postgres-sync")]
impl PostgresAuditAdapter for SyncPostgresAdapter {
    fn insert_audit_row(&mut self, row: AgentAuditEventRow) -> KernelResult<()> {
        let tenant_id = u64_to_i64(row.tenant_id, "tenant_id")?;
        let organization_id = u64_to_i64(row.organization_id, "organization_id")?;
        let agent_business_id = u64_to_i64(row.agent_business_id, "agent_business_id")?;

        self.with_locked_client(|client| {
            client
                .execute(
                    SQL_INSERT_AUDIT_EVENT,
                    &[
                        &row.uuid,
                        &tenant_id,
                        &organization_id,
                        &agent_business_id,
                        &row.agent_id,
                        &row.action,
                        &row.subject_id,
                        &row.subject_tenant_id,
                        &row.request_id,
                        &row.trace_id,
                        &row.payload_json,
                        &row.created_at,
                    ],
                )
                .map_err(map_postgres_error)?;
            Ok(())
        })
    }
}

pub struct PostgresAgentAuditSink<A>
where
    A: PostgresAuditAdapter,
{
    adapter: A,
    tenant_id: u64,
    organization_id: u64,
    agent_business_id: u64,
    agent_id: String,
}

impl<A> PostgresAgentAuditSink<A>
where
    A: PostgresAuditAdapter,
{
    pub fn new(
        adapter: A,
        tenant_id: u64,
        organization_id: u64,
        agent_business_id: u64,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            adapter,
            tenant_id,
            organization_id,
            agent_business_id,
            agent_id: agent_id.into(),
        }
    }
}

impl<A> AgentAuditSink for PostgresAgentAuditSink<A>
where
    A: PostgresAuditAdapter,
{
    fn record(&mut self, event: KernelEvent) -> KernelResult<()> {
        let row = AgentAuditEventRow::from_kernel_event(
            &event,
            self.tenant_id,
            self.organization_id,
            self.agent_business_id,
            self.agent_id.as_str(),
        )?;
        self.adapter.insert_audit_row(row)
    }
}

fn build_agent_business_uuid(tenant_id: u64, agent_id: &str) -> String {
    format!("agent_business_{}_{}", tenant_id, agent_id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AgentManifestSnapshot {
    schema_version: String,
    manifest_type: String,
    agent_id: String,
    name: String,
    display_name: String,
    description: String,
    version: String,
    domain: String,
    required_capabilities: Vec<String>,
    optional_capabilities: Vec<String>,
    event_families: Vec<String>,
    owner_name: String,
    status: String,
}

impl From<&AgentManifest> for AgentManifestSnapshot {
    fn from(value: &AgentManifest) -> Self {
        Self {
            schema_version: value.schema_version.clone(),
            manifest_type: value.manifest_type.clone(),
            agent_id: value.agent_id.clone(),
            name: value.name.clone(),
            display_name: value.display_name.clone(),
            description: value.description.clone(),
            version: value.version.clone(),
            domain: value.domain.clone(),
            required_capabilities: value.required_capabilities.clone(),
            optional_capabilities: value.optional_capabilities.clone(),
            event_families: value.event_families.clone(),
            owner_name: value.owner_name.clone(),
            status: value.status.clone(),
        }
    }
}

impl From<AgentManifestSnapshot> for AgentManifest {
    fn from(value: AgentManifestSnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            manifest_type: value.manifest_type,
            agent_id: value.agent_id,
            name: value.name,
            display_name: value.display_name,
            description: value.description,
            version: value.version,
            domain: value.domain,
            required_capabilities: value.required_capabilities,
            optional_capabilities: value.optional_capabilities,
            required_capability_requirements: Vec::new(),
            optional_capability_requirements: Vec::new(),
            event_families: value.event_families,
            owner_name: value.owner_name,
            status: value.status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CodeTaskIntentSnapshot {
    prompt: String,
    context_paths: Vec<String>,
    constraints: Vec<String>,
}

impl From<&CodeTaskIntent> for CodeTaskIntentSnapshot {
    fn from(value: &CodeTaskIntent) -> Self {
        Self {
            prompt: value.prompt.clone(),
            context_paths: value.context_paths.clone(),
            constraints: value.constraints.clone(),
        }
    }
}

impl From<CodeTaskIntentSnapshot> for CodeTaskIntent {
    fn from(value: CodeTaskIntentSnapshot) -> Self {
        Self {
            prompt: value.prompt,
            context_paths: value.context_paths,
            constraints: value.constraints,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuditPayloadSnapshot {
    event_id: String,
    event_type: String,
    severity: String,
    source: String,
    payload: String,
}

fn manifest_to_json(manifest: &AgentManifest) -> KernelResult<String> {
    serde_json::to_string(&AgentManifestSnapshot::from(manifest))
        .map_err(|error| KernelError::validation(format!("invalid manifest json: {error}")))
}

fn manifest_from_json(input: &str) -> KernelResult<AgentManifest> {
    let snapshot: AgentManifestSnapshot = serde_json::from_str(input)
        .map_err(|error| KernelError::validation(format!("invalid manifest json: {error}")))?;
    Ok(snapshot.into())
}

fn intent_to_json(intent: Option<&CodeTaskIntent>) -> KernelResult<Option<String>> {
    intent
        .map(|value| {
            serde_json::to_string(&CodeTaskIntentSnapshot::from(value)).map_err(|error| {
                KernelError::validation(format!("invalid default_code_task_intent json: {error}"))
            })
        })
        .transpose()
}

fn intent_from_json(input: Option<&str>) -> KernelResult<Option<CodeTaskIntent>> {
    input
        .map(|value| {
            serde_json::from_str::<CodeTaskIntentSnapshot>(value)
                .map(Into::into)
                .map_err(|error| {
                    KernelError::validation(format!(
                        "invalid default_code_task_intent json: {error}"
                    ))
                })
        })
        .transpose()
}

fn tags_to_json(tags: &[String]) -> KernelResult<String> {
    serde_json::to_string(tags)
        .map_err(|error| KernelError::validation(format!("invalid tags json: {error}")))
}

fn tags_from_json(input: &str) -> KernelResult<Vec<String>> {
    serde_json::from_str(input)
        .map_err(|error| KernelError::validation(format!("invalid tags json: {error}")))
}

fn severity_as_str(value: KernelEventSeverity) -> &'static str {
    match value {
        KernelEventSeverity::Debug => "debug",
        KernelEventSeverity::Info => "info",
        KernelEventSeverity::Warn => "warn",
        KernelEventSeverity::Error => "error",
    }
}

fn severity_from_str(value: &str) -> KernelResult<KernelEventSeverity> {
    match value {
        "debug" => Ok(KernelEventSeverity::Debug),
        "info" => Ok(KernelEventSeverity::Info),
        "warn" => Ok(KernelEventSeverity::Warn),
        "error" => Ok(KernelEventSeverity::Error),
        _ => Err(KernelError::validation(format!(
            "invalid audit severity: {value}"
        ))),
    }
}

fn source_as_str(value: KernelEventSource) -> &'static str {
    match value {
        KernelEventSource::Runtime => "runtime",
        KernelEventSource::Manifest => "manifest",
        KernelEventSource::Provider => "provider",
        KernelEventSource::Model => "model",
        KernelEventSource::Tool => "tool",
        KernelEventSource::Context => "context",
        KernelEventSource::Memory => "memory",
        KernelEventSource::Policy => "policy",
        KernelEventSource::Host => "host",
        KernelEventSource::ProtocolAdapter => "protocol_adapter",
        KernelEventSource::KernelUi => "kernel_ui",
        KernelEventSource::CodeKernel => "code_kernel",
        KernelEventSource::Telemetry => "telemetry",
        KernelEventSource::Unknown => "unknown",
    }
}

fn source_from_str(value: &str) -> KernelResult<KernelEventSource> {
    match value {
        "runtime" => Ok(KernelEventSource::Runtime),
        "manifest" => Ok(KernelEventSource::Manifest),
        "provider" => Ok(KernelEventSource::Provider),
        "model" => Ok(KernelEventSource::Model),
        "tool" => Ok(KernelEventSource::Tool),
        "context" => Ok(KernelEventSource::Context),
        "memory" => Ok(KernelEventSource::Memory),
        "policy" => Ok(KernelEventSource::Policy),
        "host" => Ok(KernelEventSource::Host),
        "protocol_adapter" => Ok(KernelEventSource::ProtocolAdapter),
        "kernel_ui" => Ok(KernelEventSource::KernelUi),
        "code_kernel" => Ok(KernelEventSource::CodeKernel),
        "telemetry" => Ok(KernelEventSource::Telemetry),
        "unknown" => Ok(KernelEventSource::Unknown),
        _ => Err(KernelError::validation(format!(
            "invalid audit source: {value}"
        ))),
    }
}

#[cfg(feature = "postgres-sync")]
fn map_postgres_error(error: postgres::Error) -> KernelError {
    KernelError::provider_error("postgres_error", error.to_string())
}

#[cfg(feature = "postgres-sync")]
fn u64_to_i64(value: u64, field: &str) -> KernelResult<i64> {
    i64::try_from(value)
        .map_err(|_| KernelError::validation(format!("{field} exceeds postgres int64 range")))
}

#[cfg(feature = "postgres-sync")]
fn int64_to_u64(value: i64, field: &str) -> KernelResult<u64> {
    u64::try_from(value).map_err(|_| {
        KernelError::validation(format!("{field} must be a positive postgres int64 value"))
    })
}

#[cfg(feature = "postgres-sync")]
fn pg_row_to_agent_business_row(row: Row) -> KernelResult<AgentBusinessRow> {
    Ok(AgentBusinessRow {
        id: int64_to_u64(row.try_get("id").map_err(map_postgres_error)?, "id")?,
        uuid: row.try_get("uuid").map_err(map_postgres_error)?,
        tenant_id: int64_to_u64(
            row.try_get("tenant_id").map_err(map_postgres_error)?,
            "tenant_id",
        )?,
        organization_id: int64_to_u64(
            row.try_get("organization_id").map_err(map_postgres_error)?,
            "organization_id",
        )?,
        owner_user_id: int64_to_u64(
            row.try_get("owner_user_id").map_err(map_postgres_error)?,
            "owner_user_id",
        )?,
        agent_id: row.try_get("agent_id").map_err(map_postgres_error)?,
        code: row.try_get("code").map_err(map_postgres_error)?,
        display_name: row.try_get("display_name").map_err(map_postgres_error)?,
        description: row.try_get("description").map_err(map_postgres_error)?,
        manifest_json: row.try_get("manifest_json").map_err(map_postgres_error)?,
        default_code_task_intent_json: row
            .try_get("default_code_task_intent_json")
            .map_err(map_postgres_error)?,
        status: row.try_get("status").map_err(map_postgres_error)?,
        visibility: row.try_get("visibility").map_err(map_postgres_error)?,
        tags_json: row.try_get("tags_json").map_err(map_postgres_error)?,
        created_at: row.try_get("created_at").map_err(map_postgres_error)?,
        updated_at: row.try_get("updated_at").map_err(map_postgres_error)?,
        deleted_at: row.try_get("deleted_at").map_err(map_postgres_error)?,
        version: int64_to_u64(
            row.try_get("version").map_err(map_postgres_error)?,
            "version",
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(agent_id: &str) -> AgentManifest {
        AgentManifest {
            schema_version: "1.0.0".to_string(),
            manifest_type: "agent".to_string(),
            agent_id: agent_id.to_string(),
            name: "sample-agent".to_string(),
            display_name: "Sample Agent".to_string(),
            description: "sample".to_string(),
            version: "0.1.0".to_string(),
            domain: "intelligence".to_string(),
            required_capabilities: vec!["model.chat".to_string()],
            optional_capabilities: vec!["tool.invoke".to_string()],
            required_capability_requirements: vec![],
            optional_capability_requirements: vec![],
            event_families: vec!["agent.lifecycle".to_string()],
            owner_name: "sdkwork".to_string(),
            status: "active".to_string(),
        }
    }

    #[test]
    fn sql_contracts_use_expected_placeholders_and_filters() {
        assert!(SQL_NEXT_AGENT_BUSINESS_ID.contains("pg_get_serial_sequence"));
        assert!(SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID.contains("tenant_id = $1"));
        assert!(SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID.contains("agent_id = $2"));
        assert!(SQL_INSERT_AGENT_BUSINESS.contains("VALUES ($1"));
        assert!(SQL_INSERT_AGENT_BUSINESS.contains("$18"));
        assert!(SQL_UPDATE_AGENT_BUSINESS.contains("WHERE tenant_id = $14 AND agent_id = $15"));
        assert!(SQL_LIST_AGENT_BUSINESS.contains("ORDER BY updated_at DESC"));
        assert!(SQL_INSERT_AUDIT_EVENT.contains("$12"));
    }

    #[test]
    fn row_roundtrip_preserves_record_contract() {
        let record = AgentBusinessRecord {
            id: 1,
            agent_id: "agent.alpha".to_string(),
            tenant_id: 7,
            organization_id: 70,
            owner_user_id: 700,
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: Some("desc".to_string()),
            manifest: sample_manifest("agent.alpha"),
            default_code_task_intent: Some(CodeTaskIntent::new("Refactor runtime")),
            status: AgentBusinessStatus::Active,
            visibility: AgentVisibility::Tenant,
            tags: vec!["starter".to_string()],
            version: 3,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T01:00:00Z".to_string(),
            deleted_at: None,
        };

        let row = AgentBusinessRow::from_record(&record).expect("row mapping should succeed");
        let rebuilt = row.into_record().expect("record mapping should succeed");

        assert_eq!(rebuilt, record);
    }

    #[test]
    fn invalid_status_code_is_rejected() {
        let row = AgentBusinessRow {
            id: 1,
            uuid: "uuid".to_string(),
            tenant_id: 1,
            organization_id: 1,
            owner_user_id: 1,
            agent_id: "agent.alpha".to_string(),
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: None,
            manifest_json: manifest_to_json(&sample_manifest("agent.alpha"))
                .expect("manifest json should be valid"),
            default_code_task_intent_json: None,
            status: 9,
            visibility: 0,
            tags_json: "[]".to_string(),
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
            version: 0,
        };

        let error = row.into_record().expect_err("invalid db status should fail");
        match error {
            KernelError::Validation { message } => assert!(message.contains("status")),
            _ => panic!("expected validation error"),
        }
    }
}
