use crate::domain::{
    AgentBusinessRecord, AgentBusinessStatus, AgentDeploymentRecord, AgentDeploymentStatus,
    AgentImplementationKind, AgentProviderBindingRecord, AgentVisibility,
};
use crate::ports::{AgentAuditSink, AgentListQuery, AgentRepository};
use crate::validation::{validate_capabilities, validate_standard_id};
#[cfg(feature = "postgres-sync")]
use postgres::{Client, NoTls, Row};
use sdkwork_agent_kernel::{
    AgentManifest, KernelError, KernelEvent, KernelEventSeverity, KernelEventSource, KernelResult,
};
use sdkwork_code_kernel::CodeTaskIntent;
use serde::{Deserialize, Serialize};
#[cfg(feature = "postgres-sync")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

pub const SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID: &str =
    "SELECT id, uuid, tenant_id, organization_id, owner_user_id, agent_id, code, display_name, description, manifest_json, default_code_task_intent_json, implementation_provider_id, implementation_kind, status, visibility, tags_json, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at, version FROM ai_agent_business WHERE tenant_id = $1 AND agent_id = $2 LIMIT 1";
pub const SQL_INSERT_AGENT_BUSINESS: &str =
    "INSERT INTO ai_agent_business (id, uuid, tenant_id, organization_id, owner_user_id, agent_id, code, display_name, description, manifest_json, default_code_task_intent_json, implementation_provider_id, implementation_kind, status, visibility, tags_json, created_at, updated_at, deleted_at, version) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)";
pub const SQL_UPDATE_AGENT_BUSINESS: &str =
    "UPDATE ai_agent_business SET organization_id = $1, owner_user_id = $2, code = $3, display_name = $4, description = $5, manifest_json = $6, default_code_task_intent_json = $7, implementation_provider_id = $8, implementation_kind = $9, status = $10, visibility = $11, tags_json = $12, updated_at = $13, deleted_at = $14, version = $15 WHERE tenant_id = $16 AND agent_id = $17 AND version = $18";
pub const SQL_LIST_AGENT_BUSINESS: &str =
    "SELECT id, uuid, tenant_id, organization_id, owner_user_id, agent_id, code, display_name, description, manifest_json, default_code_task_intent_json, implementation_provider_id, implementation_kind, status, visibility, tags_json, created_at::text AS created_at, updated_at::text AS updated_at, deleted_at::text AS deleted_at, version FROM ai_agent_business WHERE tenant_id = $1 ORDER BY updated_at DESC";
pub const SQL_INSERT_AGENT_PROVIDER_BINDING: &str =
    "INSERT INTO ai_agent_provider_binding (uuid, tenant_id, agent_id, binding_id, provider_id, implementation_kind, configuration_profile_id, capabilities_json, active, version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
pub const SQL_UPDATE_AGENT_PROVIDER_BINDING: &str =
    "UPDATE ai_agent_provider_binding SET provider_id = $1, implementation_kind = $2, configuration_profile_id = $3, capabilities_json = $4, active = $5, version = $6, updated_at = $7 WHERE tenant_id = $8 AND agent_id = $9 AND binding_id = $10 AND version = $11";
pub const SQL_SELECT_AGENT_PROVIDER_BINDING: &str =
    "SELECT id, uuid, tenant_id, agent_id, binding_id, provider_id, implementation_kind, configuration_profile_id, capabilities_json, active, version, created_at::text AS created_at, updated_at::text AS updated_at FROM ai_agent_provider_binding WHERE tenant_id = $1 AND agent_id = $2 AND binding_id = $3 LIMIT 1";
pub const SQL_LIST_AGENT_PROVIDER_BINDINGS: &str =
    "SELECT id, uuid, tenant_id, agent_id, binding_id, provider_id, implementation_kind, configuration_profile_id, capabilities_json, active, version, created_at::text AS created_at, updated_at::text AS updated_at FROM ai_agent_provider_binding WHERE tenant_id = $1 AND agent_id = $2 ORDER BY active DESC, updated_at DESC, binding_id ASC";
pub const SQL_INSERT_AGENT_DEPLOYMENT: &str =
    "INSERT INTO ai_agent_deployment (uuid, tenant_id, agent_id, deployment_id, binding_id, provider_id_snapshot, implementation_kind_snapshot, configuration_profile_id_snapshot, capabilities_snapshot_json, status, version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)";
pub const SQL_LIST_AGENT_DEPLOYMENTS: &str =
    "SELECT id, uuid, tenant_id, agent_id, deployment_id, binding_id, provider_id_snapshot, implementation_kind_snapshot, configuration_profile_id_snapshot, capabilities_snapshot_json, status, version, created_at::text AS created_at, updated_at::text AS updated_at FROM ai_agent_deployment WHERE tenant_id = $1 AND agent_id = $2 ORDER BY created_at DESC, deployment_id ASC";
pub const SQL_INSERT_AUDIT_EVENT: &str =
    "INSERT INTO ai_agent_business_audit_event (uuid, tenant_id, organization_id, agent_business_id, agent_id, action, subject_id, subject_tenant_id, request_id, trace_id, payload_json, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)";
#[cfg(feature = "postgres-sync")]
pub const SQL_LIST_AUDIT_EVENTS_BY_TENANT_AND_AGENT_ID: &str =
    "SELECT id, uuid, tenant_id, organization_id, agent_business_id, agent_id, action, subject_id, subject_tenant_id, request_id, trace_id, payload_json, created_at::text AS created_at FROM ai_agent_business_audit_event WHERE tenant_id = $1 AND agent_id = $2 ORDER BY created_at DESC, id DESC";
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
    pub implementation_provider_id: Option<String>,
    pub implementation_kind: Option<String>,
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
        validate_agent_business_storage_contract(record)?;
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
            default_code_task_intent_json: intent_to_json(
                record.default_code_task_intent.as_ref(),
            )?,
            implementation_provider_id: record.implementation_provider_id.clone(),
            implementation_kind: record
                .implementation_kind
                .map(|kind| kind.as_str().to_string()),
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
        let record = AgentBusinessRecord {
            id: self.id,
            agent_id: self.agent_id,
            tenant_id: self.tenant_id,
            organization_id: self.organization_id,
            owner_user_id: self.owner_user_id,
            code: self.code,
            display_name: self.display_name,
            description: self.description,
            manifest: manifest_from_json(&self.manifest_json)?,
            default_code_task_intent: intent_from_json(
                self.default_code_task_intent_json.as_deref(),
            )?,
            implementation_provider_id: self.implementation_provider_id,
            implementation_kind: self
                .implementation_kind
                .as_deref()
                .map(parse_implementation_kind)
                .transpose()?,
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
        };
        validate_agent_business_storage_contract(&record)?;
        Ok(record)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingRow {
    pub id: u64,
    pub uuid: String,
    pub tenant_id: u64,
    pub agent_id: String,
    pub binding_id: String,
    pub provider_id: String,
    pub implementation_kind: String,
    pub configuration_profile_id: String,
    pub capabilities_json: String,
    pub active: bool,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentProviderBindingRow {
    pub fn from_record(record: &AgentProviderBindingRecord) -> KernelResult<Self> {
        validate_provider_binding_storage_contract(record)?;
        Ok(Self {
            id: 0,
            uuid: build_agent_provider_binding_uuid(
                record.tenant_id,
                &record.agent_id,
                &record.binding_id,
            ),
            tenant_id: record.tenant_id,
            agent_id: record.agent_id.clone(),
            binding_id: record.binding_id.clone(),
            provider_id: record.provider_id.clone(),
            implementation_kind: record.implementation_kind.as_str().to_string(),
            configuration_profile_id: record.configuration_profile_id.clone(),
            capabilities_json: string_list_to_json(&record.capabilities, "capabilities")?,
            active: record.active,
            version: record.version,
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        })
    }

    pub fn into_record(self) -> KernelResult<AgentProviderBindingRecord> {
        let capabilities = string_list_from_json(&self.capabilities_json, "capabilities")?;
        let record = AgentProviderBindingRecord {
            tenant_id: self.tenant_id,
            agent_id: self.agent_id,
            binding_id: self.binding_id,
            provider_id: self.provider_id,
            implementation_kind: parse_implementation_kind(&self.implementation_kind)?,
            configuration_profile_id: self.configuration_profile_id,
            capabilities,
            active: self.active,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        validate_provider_binding_storage_contract(&record)?;
        Ok(record)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeploymentRow {
    pub id: u64,
    pub uuid: String,
    pub tenant_id: u64,
    pub agent_id: String,
    pub deployment_id: String,
    pub binding_id: String,
    pub provider_id_snapshot: String,
    pub implementation_kind_snapshot: String,
    pub configuration_profile_id_snapshot: String,
    pub capabilities_snapshot_json: String,
    pub status: i16,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentDeploymentRow {
    pub fn from_record(record: &AgentDeploymentRecord) -> KernelResult<Self> {
        validate_deployment_storage_contract(record)?;
        Ok(Self {
            id: 0,
            uuid: build_agent_deployment_uuid(
                record.tenant_id,
                &record.agent_id,
                &record.deployment_id,
            ),
            tenant_id: record.tenant_id,
            agent_id: record.agent_id.clone(),
            deployment_id: record.deployment_id.clone(),
            binding_id: record.binding_id.clone(),
            provider_id_snapshot: record.provider_id_snapshot.clone(),
            implementation_kind_snapshot: record.implementation_kind_snapshot.as_str().to_string(),
            configuration_profile_id_snapshot: record.configuration_profile_id_snapshot.clone(),
            capabilities_snapshot_json: string_list_to_json(
                &record.capabilities_snapshot,
                "capabilities_snapshot",
            )?,
            status: record.status.as_db_code(),
            version: record.version,
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        })
    }

    pub fn into_record(self) -> KernelResult<AgentDeploymentRecord> {
        let capabilities_snapshot =
            string_list_from_json(&self.capabilities_snapshot_json, "capabilities_snapshot")?;
        let record = AgentDeploymentRecord {
            tenant_id: self.tenant_id,
            agent_id: self.agent_id,
            deployment_id: self.deployment_id,
            binding_id: self.binding_id,
            provider_id_snapshot: self.provider_id_snapshot,
            implementation_kind_snapshot: parse_implementation_kind(
                &self.implementation_kind_snapshot,
            )?,
            configuration_profile_id_snapshot: self.configuration_profile_id_snapshot,
            capabilities_snapshot,
            status: AgentDeploymentStatus::from_db_code(self.status).ok_or_else(|| {
                KernelError::validation(format!("invalid deployment status code: {}", self.status))
            })?,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        validate_deployment_storage_contract(&record)?;
        Ok(record)
    }
}

fn parse_implementation_kind(input: &str) -> KernelResult<AgentImplementationKind> {
    AgentImplementationKind::from_str(input)
        .ok_or_else(|| KernelError::validation(format!("invalid implementation kind: {input}")))
}

fn validate_agent_business_storage_contract(record: &AgentBusinessRecord) -> KernelResult<()> {
    if let Some(provider_id) = record.implementation_provider_id.as_deref() {
        validate_standard_id(provider_id, "implementationProviderId", Some("provider."))?;
    }
    Ok(())
}

fn validate_provider_binding_storage_contract(
    record: &AgentProviderBindingRecord,
) -> KernelResult<()> {
    validate_standard_id(record.binding_id.as_str(), "bindingId", Some("binding."))?;
    validate_standard_id(record.provider_id.as_str(), "providerId", Some("provider."))?;
    validate_standard_id(
        record.configuration_profile_id.as_str(),
        "configurationProfileId",
        Some("profile."),
    )?;
    validate_capabilities(record.capabilities.as_slice(), "capabilities")
}

fn validate_deployment_storage_contract(record: &AgentDeploymentRecord) -> KernelResult<()> {
    validate_standard_id(
        record.deployment_id.as_str(),
        "deploymentId",
        Some("deployment."),
    )?;
    validate_standard_id(record.binding_id.as_str(), "bindingId", Some("binding."))?;
    validate_standard_id(
        record.provider_id_snapshot.as_str(),
        "providerId",
        Some("provider."),
    )?;
    validate_standard_id(
        record.configuration_profile_id_snapshot.as_str(),
        "configurationProfileId",
        Some("profile."),
    )?;
    validate_capabilities(
        record.capabilities_snapshot.as_slice(),
        "capabilitiesSnapshot",
    )
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
            trace_id: event
                .trace_context
                .as_ref()
                .map(|trace| trace.trace_id.clone()),
            payload_json: serde_json::to_string(&AuditPayloadSnapshot {
                event_id: event.event_id.clone(),
                event_type: event.event_type.clone(),
                severity: severity_as_str(event.severity).to_string(),
                source: source_as_str(event.source).to_string(),
                payload: event.payload.clone(),
            })
            .map_err(|error| {
                KernelError::validation(format!("invalid audit payload json: {error}"))
            })?,
            created_at: occurred_at,
        })
    }

    pub fn into_kernel_event(self) -> KernelResult<KernelEvent> {
        let payload: AuditPayloadSnapshot = serde_json::from_str(self.payload_json.as_str())
            .map_err(|error| {
                KernelError::validation(format!("invalid audit payload json: {error}"))
            })?;
        Ok(KernelEvent::new(
            payload.event_id,
            payload.event_type,
            severity_from_str(payload.severity.as_str())?,
            payload.payload,
        )
        .from_source(source_from_str(payload.source.as_str())?)
        .occurred_at(self.created_at))
    }

    #[cfg(feature = "postgres-sync")]
    fn from_pg_row(row: &Row) -> KernelResult<Self> {
        Ok(Self {
            id: int64_to_u64(
                row.try_get::<_, i64>("id").map_err(map_postgres_error)?,
                "id",
            )?,
            uuid: row.try_get("uuid").map_err(map_postgres_error)?,
            tenant_id: int64_to_u64(
                row.try_get::<_, i64>("tenant_id")
                    .map_err(map_postgres_error)?,
                "tenant_id",
            )?,
            organization_id: int64_to_u64(
                row.try_get::<_, i64>("organization_id")
                    .map_err(map_postgres_error)?,
                "organization_id",
            )?,
            agent_business_id: int64_to_u64(
                row.try_get::<_, i64>("agent_business_id")
                    .map_err(map_postgres_error)?,
                "agent_business_id",
            )?,
            agent_id: row.try_get("agent_id").map_err(map_postgres_error)?,
            action: row.try_get("action").map_err(map_postgres_error)?,
            subject_id: row.try_get("subject_id").map_err(map_postgres_error)?,
            subject_tenant_id: row
                .try_get("subject_tenant_id")
                .map_err(map_postgres_error)?,
            request_id: row.try_get("request_id").map_err(map_postgres_error)?,
            trace_id: row.try_get("trace_id").map_err(map_postgres_error)?,
            payload_json: row.try_get("payload_json").map_err(map_postgres_error)?,
            created_at: row.try_get("created_at").map_err(map_postgres_error)?,
        })
    }
}

pub trait PostgresAgentRepositoryAdapter {
    fn next_id(&mut self) -> u64;
    fn insert_row(&mut self, row: AgentBusinessRow) -> KernelResult<()>;
    fn update_row(&mut self, row: AgentBusinessRow) -> KernelResult<()>;
    fn get_row(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRow>;
    fn list_rows(&self, query: &AgentListQuery) -> Vec<AgentBusinessRow>;
    fn insert_provider_binding_row(&mut self, row: AgentProviderBindingRow) -> KernelResult<()>;
    fn update_provider_binding_row(&mut self, row: AgentProviderBindingRow) -> KernelResult<()>;
    fn get_provider_binding_row(
        &self,
        tenant_id: u64,
        agent_id: &str,
        binding_id: &str,
    ) -> Option<AgentProviderBindingRow>;
    fn list_provider_binding_rows(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> Vec<AgentProviderBindingRow>;
    fn insert_deployment_row(&mut self, row: AgentDeploymentRow) -> KernelResult<()>;
    fn list_deployment_rows(&self, tenant_id: u64, agent_id: &str) -> Vec<AgentDeploymentRow>;
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
        self.adapter
            .insert_row(AgentBusinessRow::from_record(&record)?)
    }

    fn update(&mut self, record: AgentBusinessRecord) -> KernelResult<()> {
        self.adapter
            .update_row(AgentBusinessRow::from_record(&record)?)
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

    fn insert_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        self.adapter
            .insert_provider_binding_row(AgentProviderBindingRow::from_record(&record)?)
    }

    fn update_provider_binding(&mut self, record: AgentProviderBindingRecord) -> KernelResult<()> {
        self.adapter
            .update_provider_binding_row(AgentProviderBindingRow::from_record(&record)?)
    }

    fn get_provider_binding(
        &self,
        tenant_id: u64,
        agent_id: &str,
        binding_id: &str,
    ) -> Option<AgentProviderBindingRecord> {
        self.adapter
            .get_provider_binding_row(tenant_id, agent_id, binding_id)
            .and_then(|row| row.into_record().ok())
    }

    fn list_provider_bindings(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> Vec<AgentProviderBindingRecord> {
        self.adapter
            .list_provider_binding_rows(tenant_id, agent_id)
            .into_iter()
            .filter_map(|row| row.into_record().ok())
            .collect()
    }

    fn insert_deployment(&mut self, record: AgentDeploymentRecord) -> KernelResult<()> {
        self.adapter
            .insert_deployment_row(AgentDeploymentRow::from_record(&record)?)
    }

    fn list_deployments(&self, tenant_id: u64, agent_id: &str) -> Vec<AgentDeploymentRecord> {
        self.adapter
            .list_deployment_rows(tenant_id, agent_id)
            .into_iter()
            .filter_map(|row| row.into_record().ok())
            .collect()
    }
}

pub trait PostgresAuditAdapter {
    fn insert_audit_row(&mut self, row: AgentAuditEventRow) -> KernelResult<()>;
    fn list_audit_rows(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> KernelResult<Vec<AgentAuditEventRow>> {
        let _ = (tenant_id, agent_id);
        Ok(Vec::new())
    }
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
        let mut client = self.client.lock().map_err(|_| {
            KernelError::provider_error("postgres_lock_error", "postgres mutex poisoned")
        })?;
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
                        &row.implementation_provider_id,
                        &row.implementation_kind,
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
        let previous_version =
            u64_to_i64(expected_previous_version(row.version)?, "previous_version")?;
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
                        &row.implementation_provider_id,
                        &row.implementation_kind,
                        &row.status,
                        &row.visibility,
                        &row.tags_json,
                        &row.updated_at,
                        &row.deleted_at,
                        &version,
                        &tenant_id,
                        &row.agent_id,
                        &previous_version,
                    ],
                )
                .map_err(map_postgres_error)?;

            if updated_rows == 0 {
                let exists = client
                    .query_opt(
                        SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID,
                        &[&tenant_id, &row.agent_id],
                    )
                    .map_err(map_postgres_error)?
                    .is_some();
                if exists {
                    return Err(KernelError::conflict("agent version mismatch"));
                }
                return Err(KernelError::validation("agent not found"));
            }
            Ok(())
        })
    }

    fn get_row(&self, tenant_id: u64, agent_id: &str) -> Option<AgentBusinessRow> {
        let tenant_id = u64_to_i64(tenant_id, "tenant_id").ok()?;
        self.with_locked_client(|client| {
            let row = client
                .query_opt(
                    SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID,
                    &[&tenant_id, &agent_id],
                )
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
                .filter(|row| {
                    let Some(search_query) = query.search_query.as_ref() else {
                        return true;
                    };
                    let normalized_query = search_query.trim().to_lowercase();
                    if normalized_query.is_empty() {
                        return true;
                    }

                    let description = row.description.as_deref().unwrap_or("");
                    row.agent_id
                        .to_lowercase()
                        .contains(normalized_query.as_str())
                        || row.code.to_lowercase().contains(normalized_query.as_str())
                        || row
                            .display_name
                            .to_lowercase()
                            .contains(normalized_query.as_str())
                        || description
                            .to_lowercase()
                            .contains(normalized_query.as_str())
                })
                .collect()
        })
        .unwrap_or_default()
    }

    fn insert_provider_binding_row(&mut self, row: AgentProviderBindingRow) -> KernelResult<()> {
        let tenant_id = u64_to_i64(row.tenant_id, "tenant_id")?;
        let version = u64_to_i64(row.version, "version")?;

        self.with_locked_client(|client| {
            client
                .execute(
                    SQL_INSERT_AGENT_PROVIDER_BINDING,
                    &[
                        &row.uuid,
                        &tenant_id,
                        &row.agent_id,
                        &row.binding_id,
                        &row.provider_id,
                        &row.implementation_kind,
                        &row.configuration_profile_id,
                        &row.capabilities_json,
                        &row.active,
                        &version,
                        &row.created_at,
                        &row.updated_at,
                    ],
                )
                .map_err(map_postgres_error)?;
            Ok(())
        })
    }

    fn update_provider_binding_row(&mut self, row: AgentProviderBindingRow) -> KernelResult<()> {
        let version = u64_to_i64(row.version, "version")?;
        let previous_version =
            u64_to_i64(expected_previous_version(row.version)?, "previous_version")?;
        let tenant_id = u64_to_i64(row.tenant_id, "tenant_id")?;

        self.with_locked_client(|client| {
            let updated_rows = client
                .execute(
                    SQL_UPDATE_AGENT_PROVIDER_BINDING,
                    &[
                        &row.provider_id,
                        &row.implementation_kind,
                        &row.configuration_profile_id,
                        &row.capabilities_json,
                        &row.active,
                        &version,
                        &row.updated_at,
                        &tenant_id,
                        &row.agent_id,
                        &row.binding_id,
                        &previous_version,
                    ],
                )
                .map_err(map_postgres_error)?;

            if updated_rows == 0 {
                let exists = client
                    .query_opt(
                        SQL_SELECT_AGENT_PROVIDER_BINDING,
                        &[&tenant_id, &row.agent_id, &row.binding_id],
                    )
                    .map_err(map_postgres_error)?
                    .is_some();
                if exists {
                    return Err(KernelError::conflict(
                        "agent provider binding version mismatch",
                    ));
                }
                return Err(KernelError::validation("agent provider binding not found"));
            }
            Ok(())
        })
    }

    fn get_provider_binding_row(
        &self,
        tenant_id: u64,
        agent_id: &str,
        binding_id: &str,
    ) -> Option<AgentProviderBindingRow> {
        let tenant_id = u64_to_i64(tenant_id, "tenant_id").ok()?;
        self.with_locked_client(|client| {
            let row = client
                .query_opt(
                    SQL_SELECT_AGENT_PROVIDER_BINDING,
                    &[&tenant_id, &agent_id, &binding_id],
                )
                .map_err(map_postgres_error)?;
            row.map(pg_row_to_agent_provider_binding_row).transpose()
        })
        .ok()
        .flatten()
    }

    fn list_provider_binding_rows(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> Vec<AgentProviderBindingRow> {
        let tenant_id = match u64_to_i64(tenant_id, "tenant_id") {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };

        self.with_locked_client(|client| {
            let rows = client
                .query(SQL_LIST_AGENT_PROVIDER_BINDINGS, &[&tenant_id, &agent_id])
                .map_err(map_postgres_error)?;

            let mut mapped_rows = Vec::with_capacity(rows.len());
            for row in rows {
                mapped_rows.push(pg_row_to_agent_provider_binding_row(row)?);
            }
            Ok(mapped_rows)
        })
        .unwrap_or_default()
    }

    fn insert_deployment_row(&mut self, row: AgentDeploymentRow) -> KernelResult<()> {
        let tenant_id = u64_to_i64(row.tenant_id, "tenant_id")?;
        let version = u64_to_i64(row.version, "version")?;

        self.with_locked_client(|client| {
            client
                .execute(
                    SQL_INSERT_AGENT_DEPLOYMENT,
                    &[
                        &row.uuid,
                        &tenant_id,
                        &row.agent_id,
                        &row.deployment_id,
                        &row.binding_id,
                        &row.provider_id_snapshot,
                        &row.implementation_kind_snapshot,
                        &row.configuration_profile_id_snapshot,
                        &row.capabilities_snapshot_json,
                        &row.status,
                        &version,
                        &row.created_at,
                        &row.updated_at,
                    ],
                )
                .map_err(map_postgres_error)?;
            Ok(())
        })
    }

    fn list_deployment_rows(&self, tenant_id: u64, agent_id: &str) -> Vec<AgentDeploymentRow> {
        let tenant_id = match u64_to_i64(tenant_id, "tenant_id") {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };

        self.with_locked_client(|client| {
            let rows = client
                .query(SQL_LIST_AGENT_DEPLOYMENTS, &[&tenant_id, &agent_id])
                .map_err(map_postgres_error)?;

            let mut mapped_rows = Vec::with_capacity(rows.len());
            for row in rows {
                mapped_rows.push(pg_row_to_agent_deployment_row(row)?);
            }
            Ok(mapped_rows)
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

    fn list_audit_rows(
        &self,
        tenant_id: u64,
        agent_id: &str,
    ) -> KernelResult<Vec<AgentAuditEventRow>> {
        let tenant_id = u64_to_i64(tenant_id, "tenant_id")?;
        self.with_locked_client(|client| {
            let rows = client
                .query(
                    SQL_LIST_AUDIT_EVENTS_BY_TENANT_AND_AGENT_ID,
                    &[&tenant_id, &agent_id],
                )
                .map_err(map_postgres_error)?;
            rows.iter().map(AgentAuditEventRow::from_pg_row).collect()
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

    fn list_events(&self, tenant_id: u64, agent_id: &str) -> KernelResult<Vec<KernelEvent>> {
        self.adapter
            .list_audit_rows(tenant_id, agent_id)?
            .into_iter()
            .map(AgentAuditEventRow::into_kernel_event)
            .collect()
    }
}

fn build_agent_business_uuid(tenant_id: u64, agent_id: &str) -> String {
    format!("agent_business_{}_{}", tenant_id, agent_id)
}

fn build_agent_provider_binding_uuid(tenant_id: u64, agent_id: &str, binding_id: &str) -> String {
    format!(
        "agent_provider_binding_{}_{}_{}",
        tenant_id, agent_id, binding_id
    )
}

fn build_agent_deployment_uuid(tenant_id: u64, agent_id: &str, deployment_id: &str) -> String {
    format!(
        "agent_deployment_{}_{}_{}",
        tenant_id, agent_id, deployment_id
    )
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
    implementation_provider_id: Option<String>,
    implementation_kind: Option<String>,
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
            implementation_provider_id: None,
            implementation_kind: None,
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

fn string_list_to_json(values: &[String], field_name: &str) -> KernelResult<String> {
    serde_json::to_string(values)
        .map_err(|error| KernelError::validation(format!("invalid {field_name} json: {error}")))
}

fn string_list_from_json(input: &str, field_name: &str) -> KernelResult<Vec<String>> {
    serde_json::from_str(input)
        .map_err(|error| KernelError::validation(format!("invalid {field_name} json: {error}")))
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

#[cfg(any(feature = "postgres-sync", test))]
fn expected_previous_version(next_version: u64) -> KernelResult<u64> {
    next_version
        .checked_sub(1)
        .ok_or_else(|| KernelError::validation("agent version must be >= 1 for update"))
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
        implementation_provider_id: row
            .try_get("implementation_provider_id")
            .map_err(map_postgres_error)?,
        implementation_kind: row
            .try_get("implementation_kind")
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

#[cfg(feature = "postgres-sync")]
fn pg_row_to_agent_provider_binding_row(row: Row) -> KernelResult<AgentProviderBindingRow> {
    Ok(AgentProviderBindingRow {
        id: int64_to_u64(row.try_get("id").map_err(map_postgres_error)?, "id")?,
        uuid: row.try_get("uuid").map_err(map_postgres_error)?,
        tenant_id: int64_to_u64(
            row.try_get("tenant_id").map_err(map_postgres_error)?,
            "tenant_id",
        )?,
        agent_id: row.try_get("agent_id").map_err(map_postgres_error)?,
        binding_id: row.try_get("binding_id").map_err(map_postgres_error)?,
        provider_id: row.try_get("provider_id").map_err(map_postgres_error)?,
        implementation_kind: row
            .try_get("implementation_kind")
            .map_err(map_postgres_error)?,
        configuration_profile_id: row
            .try_get("configuration_profile_id")
            .map_err(map_postgres_error)?,
        capabilities_json: row
            .try_get("capabilities_json")
            .map_err(map_postgres_error)?,
        active: row.try_get("active").map_err(map_postgres_error)?,
        version: int64_to_u64(
            row.try_get("version").map_err(map_postgres_error)?,
            "version",
        )?,
        created_at: row.try_get("created_at").map_err(map_postgres_error)?,
        updated_at: row.try_get("updated_at").map_err(map_postgres_error)?,
    })
}

#[cfg(feature = "postgres-sync")]
fn pg_row_to_agent_deployment_row(row: Row) -> KernelResult<AgentDeploymentRow> {
    Ok(AgentDeploymentRow {
        id: int64_to_u64(row.try_get("id").map_err(map_postgres_error)?, "id")?,
        uuid: row.try_get("uuid").map_err(map_postgres_error)?,
        tenant_id: int64_to_u64(
            row.try_get("tenant_id").map_err(map_postgres_error)?,
            "tenant_id",
        )?,
        agent_id: row.try_get("agent_id").map_err(map_postgres_error)?,
        deployment_id: row.try_get("deployment_id").map_err(map_postgres_error)?,
        binding_id: row.try_get("binding_id").map_err(map_postgres_error)?,
        provider_id_snapshot: row
            .try_get("provider_id_snapshot")
            .map_err(map_postgres_error)?,
        implementation_kind_snapshot: row
            .try_get("implementation_kind_snapshot")
            .map_err(map_postgres_error)?,
        configuration_profile_id_snapshot: row
            .try_get("configuration_profile_id_snapshot")
            .map_err(map_postgres_error)?,
        capabilities_snapshot_json: row
            .try_get("capabilities_snapshot_json")
            .map_err(map_postgres_error)?,
        status: row.try_get("status").map_err(map_postgres_error)?,
        version: int64_to_u64(
            row.try_get("version").map_err(map_postgres_error)?,
            "version",
        )?,
        created_at: row.try_get("created_at").map_err(map_postgres_error)?,
        updated_at: row.try_get("updated_at").map_err(map_postgres_error)?,
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

    fn sample_provider_binding_row() -> AgentProviderBindingRow {
        AgentProviderBindingRow {
            id: 1,
            uuid: "agent_provider_binding_7_agent.alpha_binding.rig.default".to_string(),
            tenant_id: 7,
            agent_id: "agent.alpha".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: "typed-local-provider".to_string(),
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities_json: r#"["model.chat","tool.invoke"]"#.to_string(),
            active: true,
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        }
    }

    fn sample_deployment_row() -> AgentDeploymentRow {
        AgentDeploymentRow {
            id: 1,
            uuid: "agent_deployment_7_agent.alpha_deployment.rig.local.001".to_string(),
            tenant_id: 7,
            agent_id: "agent.alpha".to_string(),
            deployment_id: "deployment.rig.local.001".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id_snapshot: "provider.model.rig-rust".to_string(),
            implementation_kind_snapshot: "typed-local-provider".to_string(),
            configuration_profile_id_snapshot: "profile.rig.local".to_string(),
            capabilities_snapshot_json: r#"["model.chat","planning.create"]"#.to_string(),
            status: 0,
            version: 1,
            created_at: "2026-06-01T03:00:00Z".to_string(),
            updated_at: "2026-06-01T03:00:00Z".to_string(),
        }
    }

    fn sample_agent_business_row() -> AgentBusinessRow {
        AgentBusinessRow {
            id: 1,
            uuid: "agent_business_7_agent.alpha".to_string(),
            tenant_id: 7,
            organization_id: 70,
            owner_user_id: 700,
            agent_id: "agent.alpha".to_string(),
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: Some("desc".to_string()),
            manifest_json: manifest_to_json(&sample_manifest("agent.alpha"))
                .expect("manifest json should be valid"),
            default_code_task_intent_json: None,
            implementation_provider_id: Some("provider.model.rig-rust".to_string()),
            implementation_kind: Some("typed-local-provider".to_string()),
            status: 1,
            visibility: 1,
            tags_json: "[]".to_string(),
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
            version: 1,
        }
    }

    fn assert_validation_contains(error: KernelError, expected: &str) {
        match error {
            KernelError::Validation { message } => assert!(
                message.contains(expected),
                "expected validation message to contain {expected:?}, got {message:?}"
            ),
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn sql_contracts_use_expected_placeholders_and_filters() {
        let postgres_schema = include_str!("../specs/sql/agent_business_postgres.sql");

        assert!(SQL_NEXT_AGENT_BUSINESS_ID.contains("pg_get_serial_sequence"));
        assert!(SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID.contains("tenant_id = $1"));
        assert!(SQL_SELECT_AGENT_BY_TENANT_AND_AGENT_ID.contains("agent_id = $2"));
        assert!(SQL_INSERT_AGENT_BUSINESS.contains("VALUES ($1"));
        assert!(SQL_INSERT_AGENT_BUSINESS.contains("$20"));
        assert!(SQL_INSERT_AGENT_BUSINESS.contains("implementation_provider_id"));
        assert!(SQL_UPDATE_AGENT_BUSINESS
            .contains("WHERE tenant_id = $16 AND agent_id = $17 AND version = $18"));
        assert!(SQL_LIST_AGENT_BUSINESS.contains("ORDER BY updated_at DESC"));
        assert!(SQL_INSERT_AUDIT_EVENT.contains("$12"));
        #[cfg(feature = "postgres-sync")]
        assert!(SQL_LIST_AUDIT_EVENTS_BY_TENANT_AND_AGENT_ID
            .contains("ORDER BY created_at DESC, id DESC"));
        assert!(SQL_INSERT_AGENT_PROVIDER_BINDING.contains("INSERT INTO ai_agent_provider_binding"));
        assert!(SQL_INSERT_AGENT_PROVIDER_BINDING.contains("$12"));
        assert!(SQL_UPDATE_AGENT_PROVIDER_BINDING
            .contains("WHERE tenant_id = $8 AND agent_id = $9 AND binding_id = $10"));
        assert!(SQL_UPDATE_AGENT_PROVIDER_BINDING.contains("AND version = $11"));
        assert!(SQL_SELECT_AGENT_PROVIDER_BINDING.contains("binding_id = $3"));
        assert!(SQL_LIST_AGENT_PROVIDER_BINDINGS
            .contains("ORDER BY active DESC, updated_at DESC, binding_id ASC"));
        assert!(SQL_INSERT_AGENT_DEPLOYMENT.contains("INSERT INTO ai_agent_deployment"));
        assert!(SQL_INSERT_AGENT_DEPLOYMENT.contains("$12"));
        assert!(SQL_LIST_AGENT_DEPLOYMENTS.contains("ORDER BY created_at DESC, deployment_id ASC"));

        for required in [
            "ck_ai_agent_business_implementation_provider_id_standard",
            "implementation_provider_id ~ '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_provider_binding_binding_id_standard",
            "binding_id ~ '^binding\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_provider_binding_provider_id_standard",
            "provider_id ~ '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_provider_binding_configuration_profile_id_standard",
            "configuration_profile_id ~ '^profile\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_provider_binding_capabilities_standard",
            "sdkwork_agent_business_capabilities_json_is_standard(capabilities_json)",
            "ck_ai_agent_deployment_deployment_id_standard",
            "deployment_id ~ '^deployment\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_deployment_binding_id_standard",
            "binding_id ~ '^binding\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_deployment_provider_id_snapshot_standard",
            "provider_id_snapshot ~ '^provider\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_deployment_configuration_profile_id_snapshot_standard",
            "configuration_profile_id_snapshot ~ '^profile\\.[a-z0-9_-]+(\\.[a-z0-9_-]+)*$'",
            "ck_ai_agent_deployment_capabilities_snapshot_standard",
            "sdkwork_agent_business_capabilities_json_is_standard(capabilities_snapshot_json)",
            "jsonb_typeof(capability_values.value) = 'string'",
            "capability_values.value #>> '{}'",
            "char_length(capability_values.value #>> '{}') <= 128",
            "~ '^[a-z0-9_-]+(\\.[a-z0-9_-]+)+$'",
            "COUNT(DISTINCT capability_values.value #>> '{}')",
        ] {
            assert!(
                postgres_schema.contains(required),
                "postgres schema must contain {required}"
            );
        }
    }

    #[test]
    fn expected_previous_version_maps_incremented_version() {
        let previous = expected_previous_version(3).expect("version should map");
        assert_eq!(previous, 2);
    }

    #[test]
    fn expected_previous_version_rejects_zero() {
        let error = expected_previous_version(0)
            .expect_err("version=0 cannot be used for update precondition");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains(">= 1"));
            }
            _ => panic!("expected validation error"),
        }
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
            implementation_provider_id: Some("provider.model.rig-rust".to_string()),
            implementation_kind: Some(AgentImplementationKind::TypedLocalProvider),
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
    fn agent_business_row_rejects_non_standard_implementation_provider_id_from_storage() {
        let mut row = sample_agent_business_row();
        row.implementation_provider_id = Some("model.rig-rust".to_string());

        let error = row
            .into_record()
            .expect_err("implementation provider id without provider prefix should fail");

        assert_validation_contains(error, "implementationProviderId");
    }

    #[test]
    fn provider_binding_row_roundtrip_preserves_standard_snapshots() {
        let record = AgentProviderBindingRecord {
            tenant_id: 7,
            agent_id: "agent.alpha".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string(), "tool.invoke".to_string()],
            active: true,
            version: 2,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T02:00:00Z".to_string(),
        };

        let row = AgentProviderBindingRow::from_record(&record)
            .expect("provider binding row mapping should succeed");

        assert_eq!(
            row.uuid,
            "agent_provider_binding_7_agent.alpha_binding.rig.default"
        );
        assert_eq!(row.implementation_kind, "typed-local-provider");
        assert!(row.active);
        assert!(row.capabilities_json.contains("model.chat"));

        let rebuilt = row.into_record().expect("record mapping should succeed");

        assert_eq!(rebuilt, record);
    }

    #[test]
    fn deployment_row_roundtrip_preserves_provider_binding_snapshot() {
        let record = AgentDeploymentRecord {
            tenant_id: 7,
            agent_id: "agent.alpha".to_string(),
            deployment_id: "deployment.rig.local.001".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id_snapshot: "provider.model.rig-rust".to_string(),
            implementation_kind_snapshot: AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id_snapshot: "profile.rig.local".to_string(),
            capabilities_snapshot: vec!["model.chat".to_string(), "planning.create".to_string()],
            status: AgentDeploymentStatus::Created,
            version: 1,
            created_at: "2026-06-01T03:00:00Z".to_string(),
            updated_at: "2026-06-01T03:00:00Z".to_string(),
        };

        let row = AgentDeploymentRow::from_record(&record)
            .expect("deployment row mapping should succeed");

        assert_eq!(
            row.uuid,
            "agent_deployment_7_agent.alpha_deployment.rig.local.001"
        );
        assert_eq!(row.status, 0);
        assert_eq!(row.implementation_kind_snapshot, "typed-local-provider");
        assert!(row.capabilities_snapshot_json.contains("planning.create"));

        let rebuilt = row.into_record().expect("record mapping should succeed");

        assert_eq!(rebuilt, record);
    }

    #[test]
    fn provider_binding_row_rejects_non_standard_ids_from_storage() {
        let mut row = sample_provider_binding_row();
        row.binding_id = "rig.default".to_string();
        let error = row
            .into_record()
            .expect_err("binding id without binding prefix should fail");
        assert_validation_contains(error, "bindingId");

        let mut row = sample_provider_binding_row();
        row.provider_id = "model.rig-rust".to_string();
        let error = row
            .into_record()
            .expect_err("provider id without provider prefix should fail");
        assert_validation_contains(error, "providerId");

        let mut row = sample_provider_binding_row();
        row.configuration_profile_id = "config.rig.local".to_string();
        let error = row
            .into_record()
            .expect_err("configuration profile id without profile prefix should fail");
        assert_validation_contains(error, "configurationProfileId");
    }

    #[test]
    fn provider_binding_row_rejects_non_standard_capabilities_from_storage() {
        let mut row = sample_provider_binding_row();
        row.capabilities_json = r#"["model.chat","model.chat"]"#.to_string();
        let error = row
            .into_record()
            .expect_err("duplicate capability ids should fail");
        assert_validation_contains(error, "capabilities");

        let mut row = sample_provider_binding_row();
        row.capabilities_json = r#"["Model.Chat"]"#.to_string();
        let error = row
            .into_record()
            .expect_err("uppercase capability id should fail");
        assert_validation_contains(error, "capabilities");

        let mut row = sample_provider_binding_row();
        row.capabilities_json = r#"["chat"]"#.to_string();
        let error = row
            .into_record()
            .expect_err("unnamespaced capability id should fail");
        assert_validation_contains(error, "capabilities");
    }

    #[test]
    fn deployment_row_rejects_non_standard_snapshots_from_storage() {
        let mut row = sample_deployment_row();
        row.deployment_id = "rig.local.001".to_string();
        let error = row
            .into_record()
            .expect_err("deployment id without deployment prefix should fail");
        assert_validation_contains(error, "deploymentId");

        let mut row = sample_deployment_row();
        row.binding_id = "rig.default".to_string();
        let error = row
            .into_record()
            .expect_err("binding id without binding prefix should fail");
        assert_validation_contains(error, "bindingId");

        let mut row = sample_deployment_row();
        row.provider_id_snapshot = "model.rig-rust".to_string();
        let error = row
            .into_record()
            .expect_err("provider snapshot without provider prefix should fail");
        assert_validation_contains(error, "providerId");

        let mut row = sample_deployment_row();
        row.configuration_profile_id_snapshot = "config.rig.local".to_string();
        let error = row
            .into_record()
            .expect_err("profile snapshot without profile prefix should fail");
        assert_validation_contains(error, "configurationProfileId");

        let mut row = sample_deployment_row();
        row.capabilities_snapshot_json = r#"["planning.create","planning.create"]"#.to_string();
        let error = row
            .into_record()
            .expect_err("duplicate capability snapshot ids should fail");
        assert_validation_contains(error, "capabilitiesSnapshot");
    }

    #[test]
    fn invalid_deployment_status_code_is_rejected() {
        let row = AgentDeploymentRow {
            id: 1,
            uuid: "deployment.invalid".to_string(),
            tenant_id: 7,
            agent_id: "agent.alpha".to_string(),
            deployment_id: "deployment.invalid".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id_snapshot: "provider.model.rig-rust".to_string(),
            implementation_kind_snapshot: "typed-local-provider".to_string(),
            configuration_profile_id_snapshot: "profile.rig.local".to_string(),
            capabilities_snapshot_json: "[]".to_string(),
            status: 99,
            version: 1,
            created_at: "2026-06-01T03:00:00Z".to_string(),
            updated_at: "2026-06-01T03:00:00Z".to_string(),
        };

        let error = row
            .into_record()
            .expect_err("invalid deployment status should fail");

        match error {
            KernelError::Validation { message } => assert!(message.contains("deployment status")),
            _ => panic!("expected validation error"),
        }
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
            implementation_provider_id: None,
            implementation_kind: None,
            status: 9,
            visibility: 0,
            tags_json: "[]".to_string(),
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
            version: 0,
        };

        let error = row
            .into_record()
            .expect_err("invalid db status should fail");
        match error {
            KernelError::Validation { message } => assert!(message.contains("status")),
            _ => panic!("expected validation error"),
        }
    }
}
