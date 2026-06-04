use crate::application::{
    ActivateAgentProviderBindingCommand, AgentProviderBindingCommand,
    AgentProviderDeploymentCommand, ChangeAgentStatusCommand, CreateAgentCommand,
    DeleteAgentCommand, GetAgentCommand, ListAgentsCommand, RestoreAgentCommand,
    UpdateAgentCommand,
};
use crate::domain::{
    AgentBusinessRecord, AgentBusinessStatus, AgentDeploymentRecord, AgentImplementationKind,
    AgentProviderBindingRecord, AgentVisibility,
};
use crate::ports::AgentListQuery;
use crate::validation::{
    parse_expected_version, parse_organization_id, parse_owner_user_id, parse_tenant_id,
    validate_requested_at,
};
use sdkwork_agent_kernel::{AgentManifest, KernelError, KernelResult, PolicySubject};
use sdkwork_code_kernel::CodeTaskIntent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAgentsRequestDto {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: Option<String>,
    pub include_deleted: bool,
    pub search_query: Option<String>,
}

impl ListAgentsRequestDto {
    pub fn into_command(self, requested_by: PolicySubject) -> KernelResult<ListAgentsCommand> {
        let mut query = AgentListQuery::for_tenant(parse_tenant_id(&self.tenant_id)?);
        if let Some(organization_id) = self.organization_id {
            query = query.for_organization(parse_organization_id(&organization_id)?);
        }
        if let Some(owner_user_id) = self.owner_user_id {
            query = query.for_owner(parse_owner_user_id(&owner_user_id)?);
        }
        if self.include_deleted {
            query = query.with_deleted();
        }
        if let Some(search_query) = self.search_query {
            query = query.with_search(search_query);
        }
        Ok(ListAgentsCommand {
            query,
            requested_by,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAgentRequestDto {
    pub agent_id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub manifest: AgentManifest,
    pub visibility: String,
    pub tags: Vec<String>,
    pub default_code_task_intent: Option<CodeTaskIntent>,
    pub implementation_provider_id: Option<String>,
    pub implementation_kind: Option<String>,
    pub requested_at: String,
}

impl CreateAgentRequestDto {
    pub fn into_command(self, requested_by: PolicySubject) -> KernelResult<CreateAgentCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(CreateAgentCommand {
            agent_id: self.agent_id,
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            owner_user_id: parse_owner_user_id(&self.owner_user_id)?,
            code: self.code,
            display_name: self.display_name,
            description: self.description,
            manifest: self.manifest,
            visibility: parse_visibility(&self.visibility)?,
            tags: self.tags,
            default_code_task_intent: self.default_code_task_intent,
            implementation_provider_id: self.implementation_provider_id,
            implementation_kind: self
                .implementation_kind
                .as_deref()
                .map(parse_implementation_kind)
                .transpose()?,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub binding_id: String,
    pub provider_id: String,
    pub implementation_kind: String,
    pub configuration_profile_id: String,
    pub capabilities: Vec<String>,
    pub make_default: bool,
    pub requested_at: String,
}

impl AgentProviderBindingRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentProviderBindingCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentProviderBindingCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            binding_id: self.binding_id,
            provider_id: self.provider_id,
            implementation_kind: parse_implementation_kind(&self.implementation_kind)?,
            configuration_profile_id: self.configuration_profile_id,
            capabilities: self.capabilities,
            make_default: self.make_default,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivateAgentProviderBindingRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub binding_id: String,
    pub requested_at: String,
}

impl ActivateAgentProviderBindingRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<ActivateAgentProviderBindingCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(ActivateAgentProviderBindingCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            binding_id: self.binding_id,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderDeploymentRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub deployment_id: String,
    pub binding_id: String,
    pub requested_at: String,
}

impl AgentProviderDeploymentRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentProviderDeploymentCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentProviderDeploymentCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            deployment_id: self.deployment_id,
            binding_id: self.binding_id,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAgentRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub expected_version: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub tags: Option<Vec<String>>,
    pub default_code_task_intent: Option<CodeTaskIntent>,
    pub requested_at: String,
}

impl UpdateAgentRequestDto {
    pub fn into_command(self, requested_by: PolicySubject) -> KernelResult<UpdateAgentCommand> {
        validate_requested_at(&self.requested_at)?;
        let visibility = self
            .visibility
            .as_ref()
            .map(|value| parse_visibility(value))
            .transpose()?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        Ok(UpdateAgentCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            expected_version,
            display_name: self.display_name,
            description: self.description,
            visibility,
            tags: self.tags,
            default_code_task_intent: self.default_code_task_intent,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAgentStatusRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub expected_version: Option<String>,
    pub target_status: String,
    pub requested_at: String,
}

impl UpdateAgentStatusRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<ChangeAgentStatusCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        Ok(ChangeAgentStatusCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            expected_version,
            target_status: parse_status(&self.target_status)?,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteAgentRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub expected_version: Option<String>,
    pub requested_at: String,
}

impl DeleteAgentRequestDto {
    pub fn into_command(self, requested_by: PolicySubject) -> KernelResult<DeleteAgentCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        Ok(DeleteAgentCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            expected_version,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreAgentRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub expected_version: Option<String>,
    pub requested_at: String,
}

impl RestoreAgentRequestDto {
    pub fn into_command(self, requested_by: PolicySubject) -> KernelResult<RestoreAgentCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        Ok(RestoreAgentCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            expected_version,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAgentRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
}

impl GetAgentRequestDto {
    pub fn into_command(self, requested_by: PolicySubject) -> KernelResult<GetAgentCommand> {
        Ok(GetAgentCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            requested_by,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRecordDto {
    pub id: String,
    pub agent_id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub manifest: AgentManifest,
    pub default_code_task_intent: Option<CodeTaskIntent>,
    pub implementation_provider_id: Option<String>,
    pub implementation_kind: Option<String>,
    pub status: String,
    pub visibility: String,
    pub tags: Vec<String>,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentRecordDto {
    pub fn from_record(record: &AgentBusinessRecord) -> Self {
        Self {
            id: record.id.to_string(),
            agent_id: record.agent_id.clone(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            owner_user_id: record.owner_user_id.to_string(),
            code: record.code.clone(),
            display_name: record.display_name.clone(),
            description: record.description.clone(),
            manifest: record.manifest.clone(),
            default_code_task_intent: record.default_code_task_intent.clone(),
            implementation_provider_id: record.implementation_provider_id.clone(),
            implementation_kind: record
                .implementation_kind
                .map(|kind| kind.as_str().to_string()),
            status: record.status.as_str().to_string(),
            visibility: record.visibility.as_str().to_string(),
            tags: record.tags.clone(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingRecordDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub binding_id: String,
    pub provider_id: String,
    pub implementation_kind: String,
    pub configuration_profile_id: String,
    pub capabilities: Vec<String>,
    pub active: bool,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentProviderBindingRecordDto {
    pub fn from_record(record: &AgentProviderBindingRecord) -> Self {
        Self {
            tenant_id: record.tenant_id.to_string(),
            agent_id: record.agent_id.clone(),
            binding_id: record.binding_id.clone(),
            provider_id: record.provider_id.clone(),
            implementation_kind: record.implementation_kind.as_str().to_string(),
            configuration_profile_id: record.configuration_profile_id.clone(),
            capabilities: record.capabilities.clone(),
            active: record.active,
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingResponseDto {
    pub data: AgentProviderBindingRecordDto,
}

impl AgentProviderBindingResponseDto {
    pub fn from_record(record: &AgentProviderBindingRecord) -> Self {
        Self {
            data: AgentProviderBindingRecordDto::from_record(record),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingListDataDto {
    pub items: Vec<AgentProviderBindingRecordDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProviderBindingListResponseDto {
    pub data: AgentProviderBindingListDataDto,
}

impl AgentProviderBindingListResponseDto {
    pub fn from_records(records: &[AgentProviderBindingRecord]) -> Self {
        Self {
            data: AgentProviderBindingListDataDto {
                items: records
                    .iter()
                    .map(AgentProviderBindingRecordDto::from_record)
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeploymentRecordDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub deployment_id: String,
    pub binding_id: String,
    pub provider_id_snapshot: String,
    pub implementation_kind_snapshot: String,
    pub configuration_profile_id_snapshot: String,
    pub capabilities_snapshot: Vec<String>,
    pub status: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentDeploymentRecordDto {
    pub fn from_record(record: &AgentDeploymentRecord) -> Self {
        Self {
            tenant_id: record.tenant_id.to_string(),
            agent_id: record.agent_id.clone(),
            deployment_id: record.deployment_id.clone(),
            binding_id: record.binding_id.clone(),
            provider_id_snapshot: record.provider_id_snapshot.clone(),
            implementation_kind_snapshot: record.implementation_kind_snapshot.as_str().to_string(),
            configuration_profile_id_snapshot: record.configuration_profile_id_snapshot.clone(),
            capabilities_snapshot: record.capabilities_snapshot.clone(),
            status: record.status.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeploymentResponseDto {
    pub data: AgentDeploymentRecordDto,
}

impl AgentDeploymentResponseDto {
    pub fn from_record(record: &AgentDeploymentRecord) -> Self {
        Self {
            data: AgentDeploymentRecordDto::from_record(record),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeploymentListDataDto {
    pub items: Vec<AgentDeploymentRecordDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeploymentListResponseDto {
    pub data: AgentDeploymentListDataDto,
}

impl AgentDeploymentListResponseDto {
    pub fn from_records(records: &[AgentDeploymentRecord]) -> Self {
        Self {
            data: AgentDeploymentListDataDto {
                items: records
                    .iter()
                    .map(AgentDeploymentRecordDto::from_record)
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResponseDto {
    pub data: AgentRecordDto,
}

impl AgentResponseDto {
    pub fn from_record(record: &AgentBusinessRecord) -> Self {
        Self {
            data: AgentRecordDto::from_record(record),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentListDataDto {
    pub items: Vec<AgentRecordDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentListResponseDto {
    pub data: AgentListDataDto,
}

impl AgentListResponseDto {
    pub fn from_records(records: &[AgentBusinessRecord]) -> Self {
        Self {
            data: AgentListDataDto {
                items: records.iter().map(AgentRecordDto::from_record).collect(),
            },
        }
    }
}

fn parse_visibility(value: &str) -> KernelResult<AgentVisibility> {
    AgentVisibility::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "visibility must be one of private, organization, tenant, public: {value}"
        ))
    })
}

fn parse_status(value: &str) -> KernelResult<AgentBusinessStatus> {
    AgentBusinessStatus::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "target_status must be one of draft, active, disabled, archived, deleted: {value}"
        ))
    })
}

fn parse_implementation_kind(value: &str) -> KernelResult<AgentImplementationKind> {
    AgentImplementationKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "implementation_kind must be one of manifest-only, typed-local-provider, process-adapter, protocol-adapter: {value}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentDeploymentRecord, AgentDeploymentStatus, AgentProviderBindingRecord};
    use sdkwork_agent_kernel::PolicySubject;

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

    fn sample_subject() -> PolicySubject {
        PolicySubject::new("u-1", "t-1")
    }

    #[test]
    fn create_request_maps_to_command() {
        let command = CreateAgentRequestDto {
            agent_id: "agent.alpha".to_string(),
            tenant_id: "1".to_string(),
            organization_id: "10".to_string(),
            owner_user_id: "100".to_string(),
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: Some("alpha".to_string()),
            manifest: sample_manifest("agent.alpha"),
            visibility: "organization".to_string(),
            tags: vec!["starter".to_string()],
            default_code_task_intent: Some(CodeTaskIntent::new("Refactor runtime")),
            implementation_provider_id: None,
            implementation_kind: None,
            requested_at: "2026-06-01T00:00:00Z".to_string(),
        }
        .into_command(sample_subject())
        .expect("mapping should succeed");

        assert_eq!(command.tenant_id, 1);
        assert_eq!(command.organization_id, 10);
        assert_eq!(command.owner_user_id, 100);
        assert_eq!(command.visibility, AgentVisibility::Organization);
    }

    #[test]
    fn list_request_maps_search_query() {
        let command = ListAgentsRequestDto {
            tenant_id: "1".to_string(),
            organization_id: None,
            owner_user_id: None,
            include_deleted: false,
            search_query: Some("beta".to_string()),
        }
        .into_command(sample_subject())
        .expect("mapping should succeed");

        assert_eq!(command.query.search_query.as_deref(), Some("beta"));
    }

    #[test]
    fn invalid_status_is_rejected() {
        let result = UpdateAgentStatusRequestDto {
            tenant_id: "1".to_string(),
            agent_id: "agent.alpha".to_string(),
            expected_version: None,
            target_status: "ready".to_string(),
            requested_at: "2026-06-01T00:00:00Z".to_string(),
        }
        .into_command(sample_subject());

        let error = result.expect_err("invalid status should fail");
        match error {
            KernelError::Validation { message } => {
                assert!(message.contains("target_status"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn invalid_requested_at_is_rejected_for_mutation_commands() {
        let create_error = CreateAgentRequestDto {
            agent_id: "agent.alpha".to_string(),
            tenant_id: "1".to_string(),
            organization_id: "10".to_string(),
            owner_user_id: "100".to_string(),
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: Some("alpha".to_string()),
            manifest: sample_manifest("agent.alpha"),
            visibility: "organization".to_string(),
            tags: vec!["starter".to_string()],
            default_code_task_intent: Some(CodeTaskIntent::new("Refactor runtime")),
            implementation_provider_id: None,
            implementation_kind: None,
            requested_at: "2026-06-01".to_string(),
        }
        .into_command(sample_subject())
        .expect_err("invalid requestedAt should fail");
        match create_error {
            KernelError::Validation { message } => {
                assert!(message.contains("requestedAt"));
            }
            _ => panic!("expected validation error"),
        }

        let restore_error = RestoreAgentRequestDto {
            tenant_id: "1".to_string(),
            agent_id: "agent.alpha".to_string(),
            expected_version: None,
            requested_at: "not-a-date".to_string(),
        }
        .into_command(sample_subject())
        .expect_err("invalid requestedAt should fail");
        match restore_error {
            KernelError::Validation { message } => {
                assert!(message.contains("requestedAt"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn invalid_expected_version_is_rejected_for_mutation_commands() {
        let update_error = UpdateAgentRequestDto {
            tenant_id: "1".to_string(),
            agent_id: "agent.alpha".to_string(),
            expected_version: Some("1x".to_string()),
            display_name: None,
            description: None,
            visibility: None,
            tags: None,
            default_code_task_intent: None,
            requested_at: "2026-06-01T00:00:00Z".to_string(),
        }
        .into_command(sample_subject())
        .expect_err("invalid expectedVersion should fail");

        match update_error {
            KernelError::Validation { message } => {
                assert!(message.contains("expectedVersion"));
            }
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn record_maps_to_dto_with_int64_strings() {
        let record = AgentBusinessRecord {
            id: 7,
            agent_id: "agent.alpha".to_string(),
            tenant_id: 1,
            organization_id: 10,
            owner_user_id: 100,
            code: "alpha".to_string(),
            display_name: "Alpha".to_string(),
            description: None,
            manifest: sample_manifest("agent.alpha"),
            default_code_task_intent: None,
            implementation_provider_id: None,
            implementation_kind: None,
            status: AgentBusinessStatus::Draft,
            visibility: AgentVisibility::Private,
            tags: vec!["starter".to_string()],
            version: 2,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
        };
        let dto = AgentRecordDto::from_record(&record);

        assert_eq!(dto.id, "7");
        assert_eq!(dto.tenant_id, "1");
        assert_eq!(dto.organization_id, "10");
        assert_eq!(dto.owner_user_id, "100");
        assert_eq!(dto.version, "2");
        assert_eq!(dto.status, "draft");
        assert_eq!(dto.visibility, "private");
    }

    #[test]
    fn provider_binding_request_maps_to_command_with_implementation_kind() {
        let command = AgentProviderBindingRequestDto {
            tenant_id: "1".to_string(),
            agent_id: "agent.alpha".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: "typed-local-provider".to_string(),
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string()],
            make_default: true,
            requested_at: "2026-06-01T00:00:00Z".to_string(),
        }
        .into_command(sample_subject())
        .expect("binding command should map");

        assert_eq!(command.tenant_id, 1);
        assert_eq!(
            command.implementation_kind,
            crate::domain::AgentImplementationKind::TypedLocalProvider
        );
        assert!(command.make_default);
    }

    #[test]
    fn provider_binding_and_deployment_records_map_to_standard_dtos() {
        let binding = AgentProviderBindingRecord {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id: "provider.model.rig-rust".to_string(),
            implementation_kind: crate::domain::AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id: "profile.rig.local".to_string(),
            capabilities: vec!["model.chat".to_string(), "tool.invoke".to_string()],
            active: true,
            version: 1,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
        };
        let binding_dto = AgentProviderBindingRecordDto::from_record(&binding);

        assert_eq!(binding_dto.tenant_id, "1");
        assert_eq!(binding_dto.binding_id, "binding.rig.default");
        assert_eq!(binding_dto.implementation_kind, "typed-local-provider");
        assert!(binding_dto.active);

        let deployment = AgentDeploymentRecord {
            tenant_id: 1,
            agent_id: "agent.alpha".to_string(),
            deployment_id: "deployment.rig.1".to_string(),
            binding_id: "binding.rig.default".to_string(),
            provider_id_snapshot: "provider.model.rig-rust".to_string(),
            implementation_kind_snapshot:
                crate::domain::AgentImplementationKind::TypedLocalProvider,
            configuration_profile_id_snapshot: "profile.rig.local".to_string(),
            capabilities_snapshot: vec!["model.chat".to_string()],
            status: AgentDeploymentStatus::Created,
            version: 1,
            created_at: "2026-06-01T00:01:00Z".to_string(),
            updated_at: "2026-06-01T00:01:00Z".to_string(),
        };
        let deployment_dto = AgentDeploymentRecordDto::from_record(&deployment);

        assert_eq!(deployment_dto.deployment_id, "deployment.rig.1");
        assert_eq!(
            deployment_dto.provider_id_snapshot,
            "provider.model.rig-rust"
        );
        assert_eq!(
            deployment_dto.implementation_kind_snapshot,
            "typed-local-provider"
        );
        assert_eq!(deployment_dto.status, "created");
    }
}
