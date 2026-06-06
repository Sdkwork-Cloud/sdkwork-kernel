use crate::application::{
    ActivateAgentProviderBindingCommand, AgentKnowledgeBaseCreateCommand,
    AgentKnowledgeBaseUpdateCommand, AgentKnowledgeBindingCreateCommand,
    AgentKnowledgeChunkCreateCommand, AgentKnowledgeDocumentCreateCommand,
    AgentKnowledgeDocumentUpdateCommand, AgentKnowledgeIndexUpsertCommand,
    AgentKnowledgeSearchCommand, AgentKnowledgeSourceCreateCommand,
    AgentKnowledgeSourceUpdateCommand, AgentKnowledgeSyncJobCreateCommand,
    AgentMemoryBindingCreateCommand, AgentMemoryNamespaceCreateCommand,
    AgentMemoryProfileCreateCommand, AgentMemoryRecordCreateCommand,
    AgentMemoryRelationCreateCommand, AgentMemoryRetrievalIndexUpsertCommand,
    AgentMemorySourceCreateCommand, AgentMemoryStoreCreateCommand, AgentMemoryStoreUpdateCommand,
    AgentPreviewResponseCommand, AgentPromptOptimizationCommand, AgentProviderBindingCommand,
    AgentProviderDeploymentCommand, ChangeAgentStatusCommand, CreateAgentCommand,
    DeleteAgentCommand, GetAgentCommand, ListAgentsCommand, RestoreAgentCommand,
    UpdateAgentCommand,
};
use crate::domain::{
    AgentBusinessRecord, AgentBusinessStatus, AgentDeploymentRecord, AgentImplementationKind,
    AgentKnowledgeBaseKind, AgentKnowledgeBaseRecord, AgentKnowledgeBindingRecord,
    AgentKnowledgeBindingScopeKind, AgentKnowledgeChunkRecord, AgentKnowledgeDocumentKind,
    AgentKnowledgeDocumentRecord, AgentKnowledgeIndexKind, AgentKnowledgeIndexRecord,
    AgentKnowledgeSearchResult, AgentKnowledgeSourceKind, AgentKnowledgeSourceRecord,
    AgentKnowledgeSyncJobKind, AgentKnowledgeSyncJobRecord, AgentMemoryBindingRecord,
    AgentMemoryBindingScopeKind, AgentMemoryIndexKind, AgentMemoryNamespaceKind,
    AgentMemoryNamespaceRecord, AgentMemoryProfileRecord, AgentMemoryRecord, AgentMemoryRecordKind,
    AgentMemoryRelationKind, AgentMemoryRelationRecord, AgentMemoryRetrievalIndexRecord,
    AgentMemorySourceKind, AgentMemorySourceRecord, AgentMemoryStoreKind, AgentMemoryStoreRecord,
    AgentProviderBindingRecord, AgentRuntimeExecutionRecord, AgentVisibility,
};
use crate::ports::{AgentListQuery, AgentMarketplaceListQuery};
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

#[derive(Debug, Clone, PartialEq)]
pub struct AgentPreviewResponseRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub execution_id: String,
    pub content: String,
    pub debug_mode: bool,
    pub memory_enabled: bool,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub input_payload_json: String,
    pub requested_at: String,
}

impl AgentPreviewResponseRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentPreviewResponseCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentPreviewResponseCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            execution_id: self.execution_id,
            content: self.content,
            debug_mode: self.debug_mode,
            memory_enabled: self.memory_enabled,
            model: self.model,
            temperature: self.temperature,
            input_payload_json: self.input_payload_json,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPromptOptimizationRequestDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub execution_id: String,
    pub prompt: String,
    pub input_payload_json: String,
    pub requested_at: String,
}

impl AgentPromptOptimizationRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentPromptOptimizationCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentPromptOptimizationCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            agent_id: self.agent_id,
            execution_id: self.execution_id,
            prompt: self.prompt,
            input_payload_json: self.input_payload_json,
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
    pub manifest: Option<AgentManifest>,
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
            manifest: self.manifest,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeExecutionRecordDto {
    pub tenant_id: String,
    pub agent_id: String,
    pub execution_id: String,
    pub operation: String,
    pub status: String,
    pub input_payload_json: String,
    pub output_payload_json: String,
    pub requested_at: String,
    pub completed_at: String,
}

impl AgentRuntimeExecutionRecordDto {
    pub fn from_record(record: &AgentRuntimeExecutionRecord) -> Self {
        Self {
            tenant_id: record.tenant_id.to_string(),
            agent_id: record.agent_id.clone(),
            execution_id: record.execution_id.clone(),
            operation: record.operation.as_str().to_string(),
            status: record.status.as_str().to_string(),
            input_payload_json: record.input_payload_json.clone(),
            output_payload_json: record.output_payload_json.clone(),
            requested_at: record.requested_at.clone(),
            completed_at: record.completed_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeExecutionResponseDto {
    pub data: AgentRuntimeExecutionRecordDto,
}

impl AgentRuntimeExecutionResponseDto {
    pub fn from_record(record: &AgentRuntimeExecutionRecord) -> Self {
        Self {
            data: AgentRuntimeExecutionRecordDto::from_record(record),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAgentKnowledgeBasesRequestDto {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: Option<String>,
    pub include_deleted: bool,
    pub search_query: Option<String>,
    pub status: Option<String>,
    pub visibility: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
}

impl ListAgentKnowledgeBasesRequestDto {
    pub fn into_query(self) -> KernelResult<AgentMarketplaceListQuery> {
        let mut query = AgentMarketplaceListQuery::for_tenant(parse_tenant_id(&self.tenant_id)?);
        if let Some(organization_id) = self.organization_id {
            query = query.for_organization(parse_organization_id(&organization_id)?);
        }
        if let Some(owner_user_id) = self.owner_user_id {
            query = query.for_owner(parse_owner_user_id(&owner_user_id)?);
        }
        if let Some(status) = self.status {
            query = query.with_status(parse_status(&status)?);
        }
        if let Some(visibility) = self.visibility {
            query = query.with_visibility(parse_visibility(&visibility)?);
        }
        if self.include_deleted {
            query = query.with_deleted();
        }
        if let Some(search_query) = self.search_query {
            query = query.with_search(search_query);
        }
        if let Some(category) = self.category {
            query = query.with_category(category);
        }
        if let Some(tag) = self.tag {
            query = query.with_tag(tag);
        }
        Ok(query)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBaseCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub knowledge_base_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub provider_id: String,
    pub base_kind: String,
    pub retrieval_modes: Vec<String>,
    pub capability_ids: Vec<String>,
    pub configuration_profile_id: String,
    pub visibility: String,
    pub requested_at: String,
}

impl AgentKnowledgeBaseCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeBaseCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        let retrieval_modes = self
            .retrieval_modes
            .iter()
            .map(|value| parse_knowledge_index_kind(value))
            .collect::<KernelResult<Vec<_>>>()?;
        Ok(AgentKnowledgeBaseCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            owner_user_id: parse_owner_user_id(&self.owner_user_id)?,
            knowledge_base_id: self.knowledge_base_id,
            code: self.code,
            display_name: self.display_name,
            description: self.description,
            provider_id: self.provider_id,
            base_kind: parse_knowledge_base_kind(&self.base_kind)?,
            retrieval_modes,
            capability_ids: self.capability_ids,
            configuration_profile_id: self.configuration_profile_id,
            visibility: parse_visibility(&self.visibility)?,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBaseUpdateRequestDto {
    pub tenant_id: String,
    pub knowledge_base_id: String,
    pub expected_version: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub provider_id: Option<String>,
    pub base_kind: Option<String>,
    pub retrieval_modes: Option<Vec<String>>,
    pub capability_ids: Option<Vec<String>>,
    pub configuration_profile_id: Option<String>,
    pub visibility: Option<String>,
    pub requested_at: String,
}

impl AgentKnowledgeBaseUpdateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeBaseUpdateCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        let base_kind = self
            .base_kind
            .as_deref()
            .map(parse_knowledge_base_kind)
            .transpose()?;
        let retrieval_modes = self
            .retrieval_modes
            .as_ref()
            .map(|values| {
                values
                    .iter()
                    .map(|value| parse_knowledge_index_kind(value))
                    .collect::<KernelResult<Vec<_>>>()
            })
            .transpose()?;
        let visibility = self
            .visibility
            .as_deref()
            .map(parse_visibility)
            .transpose()?;
        Ok(AgentKnowledgeBaseUpdateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            knowledge_base_id: self.knowledge_base_id,
            expected_version,
            display_name: self.display_name,
            description: self.description,
            provider_id: self.provider_id,
            base_kind,
            retrieval_modes,
            capability_ids: self.capability_ids,
            configuration_profile_id: self.configuration_profile_id,
            visibility,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSourceCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_source_id: String,
    pub knowledge_base_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub sync_policy_json: String,
    pub metadata_json: String,
    pub requested_at: String,
}

impl AgentKnowledgeSourceCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeSourceCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentKnowledgeSourceCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            knowledge_source_id: self.knowledge_source_id,
            knowledge_base_id: self.knowledge_base_id,
            source_kind: parse_knowledge_source_kind(&self.source_kind)?,
            source_ref: self.source_ref,
            source_hash: self.source_hash,
            sync_policy_json: self.sync_policy_json,
            metadata_json: self.metadata_json,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSourceUpdateRequestDto {
    pub tenant_id: String,
    pub knowledge_source_id: String,
    pub expected_version: Option<String>,
    pub source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub source_hash: Option<String>,
    pub sync_policy_json: Option<String>,
    pub metadata_json: Option<String>,
    pub requested_at: String,
}

impl AgentKnowledgeSourceUpdateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeSourceUpdateCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        let source_kind = self
            .source_kind
            .as_deref()
            .map(parse_knowledge_source_kind)
            .transpose()?;
        Ok(AgentKnowledgeSourceUpdateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            knowledge_source_id: self.knowledge_source_id,
            expected_version,
            source_kind,
            source_ref: self.source_ref,
            source_hash: self.source_hash,
            sync_policy_json: self.sync_policy_json,
            metadata_json: self.metadata_json,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeDocumentCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_document_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub document_kind: String,
    pub title: String,
    pub content_ref: String,
    pub content_hash: String,
    pub summary: Option<String>,
    pub metadata_json: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub trust_level: i16,
    pub redaction_classification: String,
    pub requested_at: String,
}

impl AgentKnowledgeDocumentCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeDocumentCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentKnowledgeDocumentCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            knowledge_document_id: self.knowledge_document_id,
            knowledge_base_id: self.knowledge_base_id,
            knowledge_source_id: self.knowledge_source_id,
            document_kind: parse_knowledge_document_kind(&self.document_kind)?,
            title: self.title,
            content_ref: self.content_ref,
            content_hash: self.content_hash,
            summary: self.summary,
            metadata_json: self.metadata_json,
            tags: self.tags,
            categories: self.categories,
            trust_level: self.trust_level,
            redaction_classification: self.redaction_classification,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeDocumentUpdateRequestDto {
    pub tenant_id: String,
    pub knowledge_document_id: String,
    pub expected_version: Option<String>,
    pub knowledge_source_id: Option<String>,
    pub document_kind: Option<String>,
    pub title: Option<String>,
    pub content_ref: Option<String>,
    pub content_hash: Option<String>,
    pub summary: Option<String>,
    pub metadata_json: Option<String>,
    pub tags: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub trust_level: Option<i16>,
    pub redaction_classification: Option<String>,
    pub requested_at: String,
}

impl AgentKnowledgeDocumentUpdateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeDocumentUpdateCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        let document_kind = self
            .document_kind
            .as_deref()
            .map(parse_knowledge_document_kind)
            .transpose()?;
        Ok(AgentKnowledgeDocumentUpdateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            knowledge_document_id: self.knowledge_document_id,
            expected_version,
            knowledge_source_id: self.knowledge_source_id,
            document_kind,
            title: self.title,
            content_ref: self.content_ref,
            content_hash: self.content_hash,
            summary: self.summary,
            metadata_json: self.metadata_json,
            tags: self.tags,
            categories: self.categories,
            trust_level: self.trust_level,
            redaction_classification: self.redaction_classification,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeChunkCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_chunk_id: String,
    pub knowledge_document_id: String,
    pub parent_chunk_id: Option<String>,
    pub chunk_ordinal: u32,
    pub heading: Option<String>,
    pub content_ref: String,
    pub content_hash: String,
    pub token_estimate: u32,
    pub summary: Option<String>,
    pub metadata_json: String,
    pub requested_at: String,
}

impl AgentKnowledgeChunkCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeChunkCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentKnowledgeChunkCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            knowledge_chunk_id: self.knowledge_chunk_id,
            knowledge_document_id: self.knowledge_document_id,
            parent_chunk_id: self.parent_chunk_id,
            chunk_ordinal: self.chunk_ordinal,
            heading: self.heading,
            content_ref: self.content_ref,
            content_hash: self.content_hash,
            token_estimate: self.token_estimate,
            summary: self.summary,
            metadata_json: self.metadata_json,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeIndexUpsertRequestDto {
    pub tenant_id: String,
    pub knowledge_index_id: String,
    pub knowledge_base_id: String,
    pub knowledge_document_id: Option<String>,
    pub knowledge_chunk_id: Option<String>,
    pub index_kind: String,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub requested_at: String,
}

impl AgentKnowledgeIndexUpsertRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeIndexUpsertCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentKnowledgeIndexUpsertCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            knowledge_index_id: self.knowledge_index_id,
            knowledge_base_id: self.knowledge_base_id,
            knowledge_document_id: self.knowledge_document_id,
            knowledge_chunk_id: self.knowledge_chunk_id,
            index_kind: parse_knowledge_index_kind(&self.index_kind)?,
            index_provider_id: self.index_provider_id,
            external_ref: self.external_ref,
            embedding_model_id: self.embedding_model_id,
            vector_dimension: self.vector_dimension,
            content_hash: self.content_hash,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSearchRequestDto {
    pub tenant_id: String,
    pub knowledge_base_id: String,
    pub query: String,
    pub top_k: usize,
    pub retrieval_modes: Vec<String>,
    pub include_external: bool,
}

impl AgentKnowledgeSearchRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeSearchCommand> {
        let retrieval_modes = self
            .retrieval_modes
            .iter()
            .map(|value| parse_knowledge_index_kind(value))
            .collect::<KernelResult<Vec<_>>>()?;
        Ok(AgentKnowledgeSearchCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            knowledge_base_id: self.knowledge_base_id,
            query: self.query,
            top_k: self.top_k,
            retrieval_modes,
            include_external: self.include_external,
            requested_by,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBindingCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_binding_id: String,
    pub knowledge_base_id: String,
    pub agent_id: Option<String>,
    pub deployment_id: Option<String>,
    pub scope_kind: String,
    pub scope_ref: String,
    pub active: bool,
    pub default_binding: bool,
    pub requested_at: String,
}

impl AgentKnowledgeBindingCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeBindingCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentKnowledgeBindingCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            knowledge_binding_id: self.knowledge_binding_id,
            knowledge_base_id: self.knowledge_base_id,
            agent_id: self.agent_id,
            deployment_id: self.deployment_id,
            scope_kind: parse_knowledge_binding_scope_kind(&self.scope_kind)?,
            scope_ref: self.scope_ref,
            active: self.active,
            default_binding: self.default_binding,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub sync_job_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub job_kind: String,
    pub input_ref: String,
    pub input_json: String,
    pub requested_at: String,
}

impl AgentKnowledgeSyncJobCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentKnowledgeSyncJobCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentKnowledgeSyncJobCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            sync_job_id: self.sync_job_id,
            knowledge_base_id: self.knowledge_base_id,
            knowledge_source_id: self.knowledge_source_id,
            job_kind: parse_knowledge_sync_job_kind(&self.job_kind)?,
            input_ref: self.input_ref,
            input_json: self.input_json,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryStoreCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub memory_store_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub provider_id: String,
    pub store_kind: String,
    pub retrieval_modes: Vec<String>,
    pub capability_ids: Vec<String>,
    pub configuration_profile_id: String,
    pub visibility: String,
    pub requested_at: String,
}

impl AgentMemoryStoreCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryStoreCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        let retrieval_modes = self
            .retrieval_modes
            .iter()
            .map(|value| parse_memory_index_kind(value))
            .collect::<KernelResult<Vec<_>>>()?;
        Ok(AgentMemoryStoreCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            owner_user_id: parse_owner_user_id(&self.owner_user_id)?,
            memory_store_id: self.memory_store_id,
            code: self.code,
            display_name: self.display_name,
            description: self.description,
            provider_id: self.provider_id,
            store_kind: parse_memory_store_kind(&self.store_kind)?,
            retrieval_modes,
            capability_ids: self.capability_ids,
            configuration_profile_id: self.configuration_profile_id,
            visibility: parse_visibility(&self.visibility)?,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryStoreUpdateRequestDto {
    pub tenant_id: String,
    pub memory_store_id: String,
    pub expected_version: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub provider_id: Option<String>,
    pub store_kind: Option<String>,
    pub retrieval_modes: Option<Vec<String>>,
    pub capability_ids: Option<Vec<String>>,
    pub configuration_profile_id: Option<String>,
    pub visibility: Option<String>,
    pub requested_at: String,
}

impl AgentMemoryStoreUpdateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryStoreUpdateCommand> {
        validate_requested_at(&self.requested_at)?;
        let expected_version = self
            .expected_version
            .as_deref()
            .map(parse_expected_version)
            .transpose()?;
        let store_kind = self
            .store_kind
            .as_deref()
            .map(parse_memory_store_kind)
            .transpose()?;
        let retrieval_modes = self
            .retrieval_modes
            .as_ref()
            .map(|values| {
                values
                    .iter()
                    .map(|value| parse_memory_index_kind(value))
                    .collect::<KernelResult<Vec<_>>>()
            })
            .transpose()?;
        let visibility = self
            .visibility
            .as_deref()
            .map(parse_visibility)
            .transpose()?;
        Ok(AgentMemoryStoreUpdateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            memory_store_id: self.memory_store_id,
            expected_version,
            display_name: self.display_name,
            description: self.description,
            provider_id: self.provider_id,
            store_kind,
            retrieval_modes,
            capability_ids: self.capability_ids,
            configuration_profile_id: self.configuration_profile_id,
            visibility,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryProfileCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub memory_profile_id: String,
    pub memory_store_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub write_policy_json: String,
    pub retrieval_policy_json: String,
    pub compaction_policy_json: String,
    pub retention_policy_json: String,
    pub privacy_policy_json: String,
    pub visibility: String,
    pub requested_at: String,
}

impl AgentMemoryProfileCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryProfileCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemoryProfileCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            owner_user_id: parse_owner_user_id(&self.owner_user_id)?,
            memory_profile_id: self.memory_profile_id,
            memory_store_id: self.memory_store_id,
            code: self.code,
            display_name: self.display_name,
            description: self.description,
            write_policy_json: self.write_policy_json,
            retrieval_policy_json: self.retrieval_policy_json,
            compaction_policy_json: self.compaction_policy_json,
            retention_policy_json: self.retention_policy_json,
            privacy_policy_json: self.privacy_policy_json,
            visibility: parse_visibility(&self.visibility)?,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryBindingCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub memory_binding_id: String,
    pub memory_profile_id: String,
    pub agent_id: Option<String>,
    pub deployment_id: Option<String>,
    pub scope_kind: String,
    pub scope_ref: String,
    pub active: bool,
    pub default_binding: bool,
    pub requested_at: String,
}

impl AgentMemoryBindingCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryBindingCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemoryBindingCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            memory_binding_id: self.memory_binding_id,
            memory_profile_id: self.memory_profile_id,
            agent_id: self.agent_id,
            deployment_id: self.deployment_id,
            scope_kind: parse_memory_binding_scope_kind(&self.scope_kind)?,
            scope_ref: self.scope_ref,
            active: self.active,
            default_binding: self.default_binding,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryNamespaceCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub user_ref: Option<String>,
    pub session_ref: Option<String>,
    pub thread_ref: Option<String>,
    pub namespace_kind: String,
    pub visibility: String,
    pub requested_at: String,
}

impl AgentMemoryNamespaceCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryNamespaceCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemoryNamespaceCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            memory_namespace_id: self.memory_namespace_id,
            agent_id: self.agent_id,
            user_ref: self.user_ref,
            session_ref: self.session_ref,
            thread_ref: self.thread_ref,
            namespace_kind: parse_memory_namespace_kind(&self.namespace_kind)?,
            visibility: parse_visibility(&self.visibility)?,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRecordCreateRequestDto {
    pub tenant_id: String,
    pub organization_id: String,
    pub memory_id: String,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub memory_kind: String,
    pub content_format: String,
    pub content_json: String,
    pub summary: Option<String>,
    pub salience_score: f32,
    pub confidence_score: f32,
    pub freshness_score: f32,
    pub sensitivity_level: i16,
    pub effective_at: Option<String>,
    pub expires_at: Option<String>,
    pub requested_at: String,
}

impl AgentMemoryRecordCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryRecordCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemoryRecordCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            organization_id: parse_organization_id(&self.organization_id)?,
            memory_id: self.memory_id,
            memory_namespace_id: self.memory_namespace_id,
            agent_id: self.agent_id,
            memory_kind: parse_memory_record_kind(&self.memory_kind)?,
            content_format: self.content_format,
            content_json: self.content_json,
            summary: self.summary,
            salience_score: self.salience_score,
            confidence_score: self.confidence_score,
            freshness_score: self.freshness_score,
            sensitivity_level: self.sensitivity_level,
            effective_at: self.effective_at,
            expires_at: self.expires_at,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemorySourceCreateRequestDto {
    pub tenant_id: String,
    pub memory_source_id: String,
    pub memory_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub evidence_json: String,
    pub captured_at: String,
    pub requested_at: String,
}

impl AgentMemorySourceCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemorySourceCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemorySourceCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            memory_source_id: self.memory_source_id,
            memory_id: self.memory_id,
            source_kind: parse_memory_source_kind(&self.source_kind)?,
            source_ref: self.source_ref,
            source_hash: self.source_hash,
            evidence_json: self.evidence_json,
            captured_at: self.captured_at,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRelationCreateRequestDto {
    pub tenant_id: String,
    pub memory_relation_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation_kind: String,
    pub weight: f32,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub requested_at: String,
}

impl AgentMemoryRelationCreateRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryRelationCreateCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemoryRelationCreateCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            memory_relation_id: self.memory_relation_id,
            from_memory_id: self.from_memory_id,
            to_memory_id: self.to_memory_id,
            relation_kind: parse_memory_relation_kind(&self.relation_kind)?,
            weight: self.weight,
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryRetrievalIndexUpsertRequestDto {
    pub tenant_id: String,
    pub memory_index_id: String,
    pub memory_id: String,
    pub index_kind: String,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub requested_at: String,
}

impl AgentMemoryRetrievalIndexUpsertRequestDto {
    pub fn into_command(
        self,
        requested_by: PolicySubject,
    ) -> KernelResult<AgentMemoryRetrievalIndexUpsertCommand> {
        validate_requested_at(&self.requested_at)?;
        Ok(AgentMemoryRetrievalIndexUpsertCommand {
            tenant_id: parse_tenant_id(&self.tenant_id)?,
            memory_index_id: self.memory_index_id,
            memory_id: self.memory_id,
            index_kind: parse_memory_index_kind(&self.index_kind)?,
            index_provider_id: self.index_provider_id,
            external_ref: self.external_ref,
            embedding_model_id: self.embedding_model_id,
            vector_dimension: self.vector_dimension,
            content_hash: self.content_hash,
            requested_by,
            requested_at: self.requested_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBaseRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub knowledge_base_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub provider_id: String,
    pub base_kind: String,
    pub retrieval_modes: Vec<String>,
    pub capability_ids: Vec<String>,
    pub configuration_profile_id: String,
    pub status: String,
    pub visibility: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentKnowledgeBaseRecordDto {
    pub fn from_record(record: &AgentKnowledgeBaseRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            owner_user_id: record.owner_user_id.to_string(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            code: record.code.clone(),
            display_name: record.display_name.clone(),
            description: record.description.clone(),
            provider_id: record.provider_id.clone(),
            base_kind: record.base_kind.as_str().to_string(),
            retrieval_modes: record
                .retrieval_modes
                .iter()
                .map(|kind| kind.as_str().to_string())
                .collect(),
            capability_ids: record.capability_ids.clone(),
            configuration_profile_id: record.configuration_profile_id.clone(),
            status: record.status.as_str().to_string(),
            visibility: record.visibility.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSourceRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_source_id: String,
    pub knowledge_base_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub sync_policy_json: String,
    pub metadata_json: String,
    pub status: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentKnowledgeSourceRecordDto {
    pub fn from_record(record: &AgentKnowledgeSourceRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            knowledge_source_id: record.knowledge_source_id.clone(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            source_kind: record.source_kind.as_str().to_string(),
            source_ref: record.source_ref.clone(),
            source_hash: record.source_hash.clone(),
            sync_policy_json: record.sync_policy_json.clone(),
            metadata_json: record.metadata_json.clone(),
            status: record.status.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeDocumentRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_document_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub document_kind: String,
    pub title: String,
    pub content_ref: String,
    pub content_hash: String,
    pub summary: Option<String>,
    pub metadata_json: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub trust_level: i16,
    pub redaction_classification: String,
    pub chunk_count: u32,
    pub status: String,
    pub visibility: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentKnowledgeDocumentRecordDto {
    pub fn from_record(record: &AgentKnowledgeDocumentRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            knowledge_document_id: record.knowledge_document_id.clone(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            knowledge_source_id: record.knowledge_source_id.clone(),
            document_kind: record.document_kind.as_str().to_string(),
            title: record.title.clone(),
            content_ref: record.content_ref.clone(),
            content_hash: record.content_hash.clone(),
            summary: record.summary.clone(),
            metadata_json: record.metadata_json.clone(),
            tags: record.tags.clone(),
            categories: record.categories.clone(),
            trust_level: record.trust_level,
            redaction_classification: record.redaction_classification.clone(),
            chunk_count: record.chunk_count,
            status: record.status.as_str().to_string(),
            visibility: record.visibility.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeChunkRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_chunk_id: String,
    pub knowledge_document_id: String,
    pub parent_chunk_id: Option<String>,
    pub chunk_ordinal: u32,
    pub heading: Option<String>,
    pub content_ref: String,
    pub content_hash: String,
    pub token_estimate: u32,
    pub summary: Option<String>,
    pub metadata_json: String,
    pub status: String,
    pub created_at: String,
}

impl AgentKnowledgeChunkRecordDto {
    pub fn from_record(record: &AgentKnowledgeChunkRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            knowledge_chunk_id: record.knowledge_chunk_id.clone(),
            knowledge_document_id: record.knowledge_document_id.clone(),
            parent_chunk_id: record.parent_chunk_id.clone(),
            chunk_ordinal: record.chunk_ordinal,
            heading: record.heading.clone(),
            content_ref: record.content_ref.clone(),
            content_hash: record.content_hash.clone(),
            token_estimate: record.token_estimate,
            summary: record.summary.clone(),
            metadata_json: record.metadata_json.clone(),
            status: record.status.as_str().to_string(),
            created_at: record.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeIndexRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub knowledge_index_id: String,
    pub knowledge_base_id: String,
    pub knowledge_document_id: Option<String>,
    pub knowledge_chunk_id: Option<String>,
    pub index_kind: String,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub indexed_at: String,
    pub status: String,
}

impl AgentKnowledgeIndexRecordDto {
    pub fn from_record(record: &AgentKnowledgeIndexRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            knowledge_index_id: record.knowledge_index_id.clone(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            knowledge_document_id: record.knowledge_document_id.clone(),
            knowledge_chunk_id: record.knowledge_chunk_id.clone(),
            index_kind: record.index_kind.as_str().to_string(),
            index_provider_id: record.index_provider_id.clone(),
            external_ref: record.external_ref.clone(),
            embedding_model_id: record.embedding_model_id.clone(),
            vector_dimension: record.vector_dimension,
            content_hash: record.content_hash.clone(),
            indexed_at: record.indexed_at.clone(),
            status: record.status.as_str().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentKnowledgeSearchResultDto {
    pub tenant_id: String,
    pub knowledge_base_id: String,
    pub provider_id: String,
    pub knowledge_index_id: String,
    pub index_provider_id: String,
    pub retrieval_method: String,
    pub knowledge_document_id: Option<String>,
    pub document_kind: Option<String>,
    pub knowledge_chunk_id: Option<String>,
    pub title: String,
    pub snippet: Option<String>,
    pub score: Option<f32>,
    pub source_ref: Option<String>,
    pub content_ref: Option<String>,
    pub external_ref: Option<String>,
    pub trust_level: i16,
    pub redaction_classification: String,
    pub metadata_json: String,
}

impl AgentKnowledgeSearchResultDto {
    pub fn from_record(record: &AgentKnowledgeSearchResult) -> Self {
        Self {
            tenant_id: record.tenant_id.to_string(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            provider_id: record.provider_id.clone(),
            knowledge_index_id: record.knowledge_index_id.clone(),
            index_provider_id: record.index_provider_id.clone(),
            retrieval_method: record.retrieval_method.as_str().to_string(),
            knowledge_document_id: record.knowledge_document_id.clone(),
            document_kind: record.document_kind.map(|kind| kind.as_str().to_string()),
            knowledge_chunk_id: record.knowledge_chunk_id.clone(),
            title: record.title.clone(),
            snippet: record.snippet.clone(),
            score: record.score,
            source_ref: record.source_ref.clone(),
            content_ref: record.content_ref.clone(),
            external_ref: record.external_ref.clone(),
            trust_level: record.trust_level,
            redaction_classification: record.redaction_classification.clone(),
            metadata_json: record.metadata_json.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeBindingRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub knowledge_binding_id: String,
    pub knowledge_base_id: String,
    pub agent_id: Option<String>,
    pub deployment_id: Option<String>,
    pub scope_kind: String,
    pub scope_ref: String,
    pub active: bool,
    pub default_binding: bool,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentKnowledgeBindingRecordDto {
    pub fn from_record(record: &AgentKnowledgeBindingRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            knowledge_binding_id: record.knowledge_binding_id.clone(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            agent_id: record.agent_id.clone(),
            deployment_id: record.deployment_id.clone(),
            scope_kind: record.scope_kind.as_str().to_string(),
            scope_ref: record.scope_ref.clone(),
            active: record.active,
            default_binding: record.default_binding,
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKnowledgeSyncJobRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub sync_job_id: String,
    pub knowledge_base_id: String,
    pub knowledge_source_id: Option<String>,
    pub job_kind: String,
    pub status: String,
    pub input_ref: String,
    pub input_json: String,
    pub output_json: Option<String>,
    pub error_json: Option<String>,
    pub requested_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentKnowledgeSyncJobRecordDto {
    pub fn from_record(record: &AgentKnowledgeSyncJobRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            sync_job_id: record.sync_job_id.clone(),
            knowledge_base_id: record.knowledge_base_id.clone(),
            knowledge_source_id: record.knowledge_source_id.clone(),
            job_kind: record.job_kind.as_str().to_string(),
            status: record.status.as_str().to_string(),
            input_ref: record.input_ref.clone(),
            input_json: record.input_json.clone(),
            output_json: record.output_json.clone(),
            error_json: record.error_json.clone(),
            requested_at: record.requested_at.clone(),
            started_at: record.started_at.clone(),
            completed_at: record.completed_at.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryStoreRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub memory_store_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub provider_id: String,
    pub store_kind: String,
    pub retrieval_modes: Vec<String>,
    pub capability_ids: Vec<String>,
    pub configuration_profile_id: String,
    pub status: String,
    pub visibility: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentMemoryStoreRecordDto {
    pub fn from_record(record: &AgentMemoryStoreRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            owner_user_id: record.owner_user_id.to_string(),
            memory_store_id: record.memory_store_id.clone(),
            code: record.code.clone(),
            display_name: record.display_name.clone(),
            description: record.description.clone(),
            provider_id: record.provider_id.clone(),
            store_kind: record.store_kind.as_str().to_string(),
            retrieval_modes: record
                .retrieval_modes
                .iter()
                .map(|kind| kind.as_str().to_string())
                .collect(),
            capability_ids: record.capability_ids.clone(),
            configuration_profile_id: record.configuration_profile_id.clone(),
            status: record.status.as_str().to_string(),
            visibility: record.visibility.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryProfileRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub owner_user_id: String,
    pub memory_profile_id: String,
    pub memory_store_id: String,
    pub code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub write_policy_json: String,
    pub retrieval_policy_json: String,
    pub compaction_policy_json: String,
    pub retention_policy_json: String,
    pub privacy_policy_json: String,
    pub status: String,
    pub visibility: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentMemoryProfileRecordDto {
    pub fn from_record(record: &AgentMemoryProfileRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            owner_user_id: record.owner_user_id.to_string(),
            memory_profile_id: record.memory_profile_id.clone(),
            memory_store_id: record.memory_store_id.clone(),
            code: record.code.clone(),
            display_name: record.display_name.clone(),
            description: record.description.clone(),
            write_policy_json: record.write_policy_json.clone(),
            retrieval_policy_json: record.retrieval_policy_json.clone(),
            compaction_policy_json: record.compaction_policy_json.clone(),
            retention_policy_json: record.retention_policy_json.clone(),
            privacy_policy_json: record.privacy_policy_json.clone(),
            status: record.status.as_str().to_string(),
            visibility: record.visibility.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryBindingRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub memory_binding_id: String,
    pub memory_profile_id: String,
    pub agent_id: Option<String>,
    pub deployment_id: Option<String>,
    pub scope_kind: String,
    pub scope_ref: String,
    pub active: bool,
    pub default_binding: bool,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
}

impl AgentMemoryBindingRecordDto {
    pub fn from_record(record: &AgentMemoryBindingRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            memory_binding_id: record.memory_binding_id.clone(),
            memory_profile_id: record.memory_profile_id.clone(),
            agent_id: record.agent_id.clone(),
            deployment_id: record.deployment_id.clone(),
            scope_kind: record.scope_kind.as_str().to_string(),
            scope_ref: record.scope_ref.clone(),
            active: record.active,
            default_binding: record.default_binding,
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryNamespaceRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub user_ref: Option<String>,
    pub session_ref: Option<String>,
    pub thread_ref: Option<String>,
    pub namespace_kind: String,
    pub status: String,
    pub visibility: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AgentMemoryNamespaceRecordDto {
    pub fn from_record(record: &AgentMemoryNamespaceRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            memory_namespace_id: record.memory_namespace_id.clone(),
            agent_id: record.agent_id.clone(),
            user_ref: record.user_ref.clone(),
            session_ref: record.session_ref.clone(),
            thread_ref: record.thread_ref.clone(),
            namespace_kind: record.namespace_kind.as_str().to_string(),
            status: record.status.as_str().to_string(),
            visibility: record.visibility.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub memory_id: String,
    pub memory_namespace_id: String,
    pub agent_id: Option<String>,
    pub memory_kind: String,
    pub content_format: String,
    pub content_json: String,
    pub summary: Option<String>,
    pub salience_score: f32,
    pub confidence_score: f32,
    pub freshness_score: f32,
    pub sensitivity_level: i16,
    pub source_count: u32,
    pub effective_at: Option<String>,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub use_count: String,
    pub status: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub redacted_at: Option<String>,
}

impl AgentMemoryRecordDto {
    pub fn from_record(record: &AgentMemoryRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            organization_id: record.organization_id.to_string(),
            memory_id: record.memory_id.clone(),
            memory_namespace_id: record.memory_namespace_id.clone(),
            agent_id: record.agent_id.clone(),
            memory_kind: record.memory_kind.as_str().to_string(),
            content_format: record.content_format.clone(),
            content_json: record.content_json.clone(),
            summary: record.summary.clone(),
            salience_score: record.salience_score,
            confidence_score: record.confidence_score,
            freshness_score: record.freshness_score,
            sensitivity_level: record.sensitivity_level,
            source_count: record.source_count,
            effective_at: record.effective_at.clone(),
            expires_at: record.expires_at.clone(),
            last_used_at: record.last_used_at.clone(),
            use_count: record.use_count.to_string(),
            status: record.status.as_str().to_string(),
            version: record.version.to_string(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            deleted_at: record.deleted_at.clone(),
            redacted_at: record.redacted_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemorySourceRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub memory_source_id: String,
    pub memory_id: String,
    pub source_kind: String,
    pub source_ref: String,
    pub source_hash: String,
    pub evidence_json: String,
    pub captured_at: String,
    pub created_at: String,
}

impl AgentMemorySourceRecordDto {
    pub fn from_record(record: &AgentMemorySourceRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            memory_source_id: record.memory_source_id.clone(),
            memory_id: record.memory_id.clone(),
            source_kind: record.source_kind.as_str().to_string(),
            source_ref: record.source_ref.clone(),
            source_hash: record.source_hash.clone(),
            evidence_json: record.evidence_json.clone(),
            captured_at: record.captured_at.clone(),
            created_at: record.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMemoryRelationRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub memory_relation_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation_kind: String,
    pub weight: f32,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub created_at: String,
}

impl AgentMemoryRelationRecordDto {
    pub fn from_record(record: &AgentMemoryRelationRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            memory_relation_id: record.memory_relation_id.clone(),
            from_memory_id: record.from_memory_id.clone(),
            to_memory_id: record.to_memory_id.clone(),
            relation_kind: record.relation_kind.as_str().to_string(),
            weight: record.weight,
            valid_from: record.valid_from.clone(),
            valid_until: record.valid_until.clone(),
            created_at: record.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryRetrievalIndexRecordDto {
    pub id: String,
    pub tenant_id: String,
    pub memory_index_id: String,
    pub memory_id: String,
    pub index_kind: String,
    pub index_provider_id: String,
    pub external_ref: String,
    pub embedding_model_id: Option<String>,
    pub vector_dimension: Option<u32>,
    pub content_hash: String,
    pub indexed_at: String,
    pub status: String,
}

impl AgentMemoryRetrievalIndexRecordDto {
    pub fn from_record(record: &AgentMemoryRetrievalIndexRecord) -> Self {
        Self {
            id: record.id.to_string(),
            tenant_id: record.tenant_id.to_string(),
            memory_index_id: record.memory_index_id.clone(),
            memory_id: record.memory_id.clone(),
            index_kind: record.index_kind.as_str().to_string(),
            index_provider_id: record.index_provider_id.clone(),
            external_ref: record.external_ref.clone(),
            embedding_model_id: record.embedding_model_id.clone(),
            vector_dimension: record.vector_dimension,
            content_hash: record.content_hash.clone(),
            indexed_at: record.indexed_at.clone(),
            status: record.status.as_str().to_string(),
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

fn parse_knowledge_base_kind(value: &str) -> KernelResult<AgentKnowledgeBaseKind> {
    AgentKnowledgeBaseKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "baseKind must be one of wiki, document-repository, database, api-reference, graph, hybrid, external-provider, file-store: {value}"
        ))
    })
}

fn parse_knowledge_index_kind(value: &str) -> KernelResult<AgentKnowledgeIndexKind> {
    AgentKnowledgeIndexKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "indexKind must be one of exact, keyword, full_text, structured, graph, wiki, rule, vector, hybrid, llm_rerank, external: {value}"
        ))
    })
}

fn parse_knowledge_source_kind(value: &str) -> KernelResult<AgentKnowledgeSourceKind> {
    AgentKnowledgeSourceKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "sourceKind must be one of upload, wiki, web, database, api, filesystem, manual, external-provider: {value}"
        ))
    })
}

fn parse_knowledge_document_kind(value: &str) -> KernelResult<AgentKnowledgeDocumentKind> {
    AgentKnowledgeDocumentKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "documentKind must be one of wiki-page, wiki-section, article, faq, api-reference, spec, runbook, policy, external-reference, other: {value}"
        ))
    })
}

fn parse_knowledge_binding_scope_kind(value: &str) -> KernelResult<AgentKnowledgeBindingScopeKind> {
    AgentKnowledgeBindingScopeKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "scopeKind must be one of agent, deployment, user, session, organization, tenant: {value}"
        ))
    })
}

fn parse_knowledge_sync_job_kind(value: &str) -> KernelResult<AgentKnowledgeSyncJobKind> {
    AgentKnowledgeSyncJobKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "jobKind must be one of import, refresh, reindex, delete: {value}"
        ))
    })
}

fn parse_memory_store_kind(value: &str) -> KernelResult<AgentMemoryStoreKind> {
    AgentMemoryStoreKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "storeKind must be one of local-postgres, external-provider, vector-store, graph-store, hybrid-store, file-store: {value}"
        ))
    })
}

fn parse_memory_index_kind(value: &str) -> KernelResult<AgentMemoryIndexKind> {
    AgentMemoryIndexKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "indexKind must be one of keyword, sparse, vector, graph, wiki, rule, hybrid: {value}"
        ))
    })
}

fn parse_memory_binding_scope_kind(value: &str) -> KernelResult<AgentMemoryBindingScopeKind> {
    AgentMemoryBindingScopeKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "scopeKind must be one of agent, deployment, user, session, organization, tenant: {value}"
        ))
    })
}

fn parse_memory_namespace_kind(value: &str) -> KernelResult<AgentMemoryNamespaceKind> {
    AgentMemoryNamespaceKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "namespaceKind must be one of tenant, organization, agent, user, session, thread, task: {value}"
        ))
    })
}

fn parse_memory_record_kind(value: &str) -> KernelResult<AgentMemoryRecordKind> {
    AgentMemoryRecordKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "memoryKind must be one of working, episodic, semantic, procedural, preference, summary, task, correction, system: {value}"
        ))
    })
}

fn parse_memory_source_kind(value: &str) -> KernelResult<AgentMemorySourceKind> {
    AgentMemorySourceKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "sourceKind must be one of conversation-message, tool-result, document, knowledge-ref, human-feedback, system-rule, business-event: {value}"
        ))
    })
}

fn parse_memory_relation_kind(value: &str) -> KernelResult<AgentMemoryRelationKind> {
    AgentMemoryRelationKind::from_str(value).ok_or_else(|| {
        KernelError::validation(format!(
            "relationKind must be one of supports, contradicts, supersedes, duplicates, depends-on, part-of, about-entity: {value}"
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
            manifest: None,
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
            id: 10,
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
            id: 11,
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
