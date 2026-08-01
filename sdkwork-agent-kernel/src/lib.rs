//! SDKWork Agent Kernel — standardized SPI layer for all agent providers.
//!
//! This crate defines the kernel contracts (SPIs) for memory, tools, MCP,
//! task scheduling, agent classification, message querying, and more.

#![allow(clippy::should_implement_trait)]

mod a2a_protocol;
mod a2a_registry;
pub mod api;
mod chat;
mod classification;
mod collaboration;
mod configuration;
mod conformance;
mod context_memory;
mod definition;
mod error;
mod event;
mod execution;
mod execution_settings;
mod host;
mod host_sandbox;
mod installation;
mod installation_ext;
mod knowledge;
mod lifecycle;
mod manifest;
mod mcp;
mod message;
mod message_query;
pub mod modality;
mod model;
mod package;
mod planning;
mod policy;
mod protocol;
mod session_activity;
pub use a2a_protocol::{
    A2AAdapterHealth, A2AAdapterStatus, A2AAgentCard, A2AAuthentication, A2ACapability,
    A2AEndpoint, A2AError, A2AProtocolAdapter, A2ATaskContext, A2ATaskRequest, A2ATaskResponse,
    A2ATaskStatus,
};
pub use a2a_registry::{A2AAgentRegistry, A2ATaskHandler, RegistryA2AProtocolAdapter};
mod backend_health;
pub use backend_health::{
    AggregateHealthStatus, BackendHealthMonitor, DriverHealthHistory, HealthHistoryEntry,
    HealthMonitorConfig, HealthStatusChange, SdkDriverHealth, SdkDriverStatus,
};
mod cancellation;
pub use cancellation::{
    CancellationError, CancellationHandle, CancellationProvider, CancellationProviderHealth,
    CancellationProviderManifest, CancellationProviderStatus, CancellationRequest,
    CancellationResult, CancellationScope, CancellationSource, CancellationSourceType,
    CancellationStatus, CancellationToken, InMemoryCancellationProvider,
};
mod model_stream;
pub use model_stream::{
    InMemoryStreamProvider, ModelStreamProvider, StreamChunk, StreamChunkType, StreamConfig,
    StreamControl, StreamError, StreamProtocol, StreamProviderHealth, StreamProviderManifest,
    StreamProviderStatus, StreamRequest, StreamResult, StreamState, StreamStatus,
};
mod orchestration;
pub use orchestration::{
    AgentGraph, AgentNode, AggregationStrategy, ExecutionStrategy, OrchestrationPlan,
    OrchestrationResult, OrchestrationStatus, OrchestrationTask, TaskResult,
};
mod rate_limit;
pub use rate_limit::{
    InMemoryRateLimitProvider, QuotaUsage, RateLimitError, RateLimitPolicy, RateLimitProvider,
    RateLimitProviderHealth, RateLimitProviderManifest, RateLimitProviderStatus, RateLimitRequest,
    RateLimitResult, ResourceType, RetryStrategy,
};
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerHealth, CircuitBreakerMetrics,
    CircuitState, ResilienceProfile,
};
pub use resilience_layer::{
    ResilienceHealth, ResilienceLayer, ResilienceLayerConfig, ResilienceRegistry, ResilienceResult,
};
pub use retry::{
    calculate_delay, execute_with_retry, is_retryable_error, RetryBudget, RetryBudgetConfig,
    RetryBudgetStatus, RetryConfig, RetryResult,
};
mod ingress_rate_limit;
pub use ingress_rate_limit::{TokenBucketRateLimitProvider, INGRESS_HTTP_RATE_LIMIT_POLICY_ID};
mod sandbox;
pub mod sandbox_runtime;
pub use sandbox::{
    FileSystemPermission, FileSystemSandboxPolicy, NetworkPermission, NetworkSandboxPolicy,
    NoOpSandboxProvider, PlatformSandboxProvider, SandboxCommand, SandboxError,
    SandboxExecutionResult, SandboxPolicy, SandboxProvider, SandboxType,
};
mod host_secret_env;
mod secret;
mod secret_composite;
mod secret_env;
#[cfg(feature = "secret-vault")]
mod secret_vault;
pub use host_secret_env::{EnvFileSecretFallbackHostProvider, EnvFileSecretHostProvider};
pub use secret::{
    EncryptionAlgorithm, InMemorySecretProvider, SecretAccessPurpose, SecretAccessRequest,
    SecretAccessResult, SecretCreateRequest, SecretError, SecretMetadata, SecretProvider,
    SecretProviderHealth, SecretProviderManifest, SecretProviderStatus, SecretRotateRequest,
    SecretType, SecretValue,
};
pub use secret_composite::ChainedSecretProvider;
pub use secret_env::{
    lookup_env_file_secret, secret_id_to_env_suffix, EnvFileSecretProvider,
    SDKWORK_SECRETS_DIR_ENV, SDKWORK_SECRET_ENV_PREFIX,
};
#[cfg(feature = "secret-vault")]
pub use secret_vault::{
    VaultSecretProvider, SDKWORK_VAULT_ADDR_ENV, SDKWORK_VAULT_MOUNT_ENV,
    SDKWORK_VAULT_NAMESPACE_ENV, SDKWORK_VAULT_TOKEN_ENV,
};
mod plugin;
mod provider;
mod resilience;
mod resilience_layer;
mod retry;
mod runtime;
mod runtime_host;
mod runtime_topology;
mod skill;
mod stream_event;
mod task_scheduling;
mod telemetry;
mod tool;

pub use api::{
    AgentConversation, AgentInvokeRequest, AgentInvokeRequestBuilder, ContentBlock,
    ConversationRole, InteractionContractBuilder, MessageBuilder,
};
pub use chat::{
    agent_chat_rpc_adapter_manifest, AgentChatCancelResponse, AgentChatKnowledgeQuery,
    AgentChatMemoryQuery, AgentChatRequest, AgentChatResponse, AgentChatRpcAdapter,
    AgentChatRpcHandler, AgentChatService, AgentChatStreamResponse,
};
pub use classification::{
    AgentCategory, AgentClassification, AgentClassificationProvider, AutonomyLevel,
    CapabilityAssessment, CapabilityLevel, ClassificationQuery,
};
pub use collaboration::{
    AgentCard, AgentCollaborationProvider, AgentDelegation, AgentDelegationRequest,
    AgentDelegationResult, AgentDelegationStatus, AgentHandoffRequest, AgentHandoffResult,
};
pub use configuration::{
    AgentConfigEntry, AgentConfigField, AgentConfigSection, AgentConfigSectionKind,
    AgentConfigValue, AgentConfigValueKind, AgentConfiguration, AgentConfigurationChange,
    AgentConfigurationInvalidField, AgentConfigurationProfile, AgentConfigurationProfileStatus,
    AgentConfigurationProvider, AgentConfigurationSpec, AgentConfigurationStore,
    AgentConfigurationStoreAction, AgentConfigurationStoreRecord, AgentConfigurationSubscriber,
    AgentConfigurationUpgradePlan, AgentConfigurationUpgradeRequest, AgentConfigurationValidation,
    AgentModelConfigurationApplication, AgentModelConfigurationFieldMapping,
    AgentModelConfigurationRequest, AgentModelSelectionRequest, AgentProfileArchiveRequest,
    AgentSecretBinding, AgentSecretBindingKind, AgentSettingSources, AgentSettingsScope,
    ConfigurationMigrationStep, ConfigurationMigrationStepKind, ConfigurationSubscription,
    InMemoryAgentConfigurationStore,
};
pub use conformance::{
    AgentRuntimeConformanceProfile, KernelConformanceCase, KernelConformanceCaseStatus,
    KernelConformanceReport,
};
pub use context_memory::{
    ContextExplanation, ContextFrame, ContextProvider, ContextRanking, MemoryProvider,
    MemoryRecord, MemoryScope, MemoryTier, RedactionClassification, TrustLevel,
};
pub use definition::{
    parse_agent_input_contract_json, parse_agent_input_policy_json, AgentDefinition,
    AgentProviderBinding, AgentProviderBindingMode, AgentProviderFamily, MemoryStrategy,
    ModelSelectionPolicy, ToolCallPolicy,
};
pub use error::{KernelError, KernelErrorInfo, KernelErrorKind, KernelErrorSource, KernelResult};
pub use event::{
    EventRecorder, EventStream, EventStreamBatch, EventStreamCursor, EventStreamFilter,
    EventStreamItem, EventStreamStatus, EventSubscription, KernelEvent, KernelEventRedaction,
    KernelEventSeverity, KernelEventSource, TraceContext,
};
pub use execution::{
    AgentExecutionReport, AgentExecutionRequest, AgentExecutionResumeDecision,
    AgentExecutionResumeRequest, AgentExecutionService, AgentExecutionStatus, AgentObservation,
};
pub use execution_settings::{
    AgentExecutionAccessModeDescriptor, AgentExecutionApprovalBehavior,
    AgentExecutionNetworkAccess, AgentExecutionProviderOption, AgentExecutionProviderOptionValue,
    AgentExecutionRiskLevel, AgentExecutionSettingsRequest, AgentExecutionSettingsResolution,
    AgentExecutionSettingsSpec, AgentExecutionWorkspaceAccess, APPROVE_FOR_ME_ACCESS_MODE_ID,
    ASK_FOR_APPROVAL_ACCESS_MODE_ID, FULL_ACCESS_MODE_ID,
};
pub use host::{
    EnvironmentRequest, EnvironmentResult, ExecutorRequest, ExecutorResult, ExecutorStatus,
    FilesystemOperation, FilesystemRequest, FilesystemResult, HostEnvPolicy, HostPathPolicy,
    HostProvider, NetworkRequest, NetworkResult, ProcessRequest, ProcessResult, SecretRef,
    SecretValue as ProviderSecretValue, StorageRequest, StorageResult, TimeRequest, TimeResult,
};
pub use host_sandbox::SandboxingHostProvider;
pub use installation::{
    AgentInstallPlan, AgentInstallReport, AgentInstallRequest, AgentInstallStatus,
    AgentInstallStep, AgentInstallStepKind, AgentInstallation, AgentInstallationDependency,
    AgentInstallationState, AgentInstaller, AgentPackageSource, AgentUninstallPlan,
    AgentUninstallReport, AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport,
    AgentUpgradeRequest,
};
pub use installation_ext::{
    AgentInstallRecord, AgentInstallRecordStatus, AgentPackageSourceInfo, AgentRollbackReport,
    AgentRollbackRequest, AgentRollbackStatus, AgentVerifyIssue, AgentVerifyIssueCategory,
    AgentVerifyIssueSeverity, AgentVerifyReport, AgentVerifyRequest, AgentVerifyStatus,
};
pub use knowledge::{
    KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, KnowledgeSearchResult,
};
pub use lifecycle::{
    AgentRun, AgentSession, AgentStep, AgentTask, RunState, SessionChangeSummary,
    SessionContinuation, SessionContinuationMode, SessionKind, SessionSource, SessionState,
    SessionTokenUsage, StepState, TaskState,
};
pub use manifest::{
    AgentManifest, Capability, CapabilityManifest, CapabilityRequirement, ProviderHealth,
    ProviderManifest,
};
pub use mcp::{
    McpPromptDescriptor, McpPromptMessage, McpProvider, McpResourceContent, McpResourceDescriptor,
    McpServerDescriptor, McpToolExecutionRequest, McpToolExecutionResponse,
    McpToolExecutionService,
};
pub use message::{
    AgentArtifact, AgentMessage, AgentMessageRole, AgentPart, AgentPartKind, ArtifactKind,
};
pub use message_query::{
    MessageQuery, MessageQueryFilter, MessageQueryProvider, MessageQueryResult, MessageSortField,
    MessageSortOrder, SessionSummary,
};
#[cfg(feature = "sdkwork-models")]
pub use modality::catalog;
pub use modality::{
    agent_messages_from_text_lines, agent_messages_to_text_lines, analyze_message_input,
    apply_delivery_transforms, check_message_against_model_descriptor, enforce_slot_constraints,
    flatten_message_to_text, infer_modality_from_mime_type, parse_chat_rpc_payload,
    parse_input_modalities, part_kind_to_input_modality, resolve_model_input,
    resolve_model_input_with_options, validate_message_against_input_policy,
    validate_structured_model_input, validate_structured_model_input_with_options,
    AgentInputContract, AgentInputModality, AgentInputPolicy, AgentInteractionContract,
    AgentOutputContract, ContentReference, ContentReferenceScheme, InputModalityCompatibility,
    InputModalityPartReport, InputModalityPreprocessor, ModalitySlot, ModelDeliveryStrategy,
    ModelInputResolution, ModelInputResolveOptions, SkillInputModalityPreprocessor,
    UnsupportedInputModalityAction, CARD_INPUT_MODES, INPUT_MODALITIES, INPUT_MODALITY_ARTIFACT,
    INPUT_MODALITY_AUDIO, INPUT_MODALITY_BINARY, INPUT_MODALITY_FILE, INPUT_MODALITY_IMAGE,
    INPUT_MODALITY_JSON, INPUT_MODALITY_MUSIC, INPUT_MODALITY_TEXT, INPUT_MODALITY_VIDEO,
};
pub use model::{
    ModelCancellationRequest, ModelCancellationResponse, ModelChunkKind, ModelDescriptor,
    ModelExecutionRequest, ModelExecutionResponse, ModelExecutionService, ModelProvider,
    ModelRequest, ModelResponse, ModelResponseFormat, ModelStatus, ModelStreamChunk,
    ModelStreamExecutionResponse, ModelStreamSink, ModelStructuredOutputValidation, ModelUsage,
};
pub use package::{
    AgentPackageLifecycle, AgentPackageManifest, AgentPackageProviderBinding,
    AgentPackageVersionCompatibility,
};
pub use planning::{Action, ActionKind, ActionStatus, Observation, Plan, PlanningProvider};
pub use plugin::{Plugin, PluginContext, PluginMetadata, PluginRegistry, PluginState};
pub use policy::{
    PolicyCategory, PolicyDecision, PolicyDecisionConstraint, PolicyDecisionValue,
    PolicyExplanation, PolicyProvider, PolicyRequest, PolicySubject,
};
pub use protocol::{
    ProtocolAdapter, ProtocolAdapterAuthMode, ProtocolAdapterManifest, ProtocolAdapterRequest,
    ProtocolAdapterResponse, ProtocolAdapterStreamingSupport, ProtocolError, ProtocolFamily,
    ProtocolObjectEnvelope, ProtocolObjectKind, ProtocolObjectMapper, ProtocolSseEvent,
    ProtocolStreamUpdate, ProtocolTransport, StandardProtocolObjectMapper,
};
pub use provider::{
    AgentProvider, BatchOperations, Cancellable, Lifecycle, Listable, PolicyGated, ProviderError,
    ProviderRegistration, ProviderSource, Streaming,
};
pub use runtime::{
    AgentProviderDiagnostic, AgentRuntime, AgentRuntimeDiagnostics, RuntimeBootstrapReport,
    RuntimeBuilder, RuntimeState,
};
pub use runtime_host::{
    AgentKernelHost, AgentRuntimeExecutionHandle, AgentRuntimeRegistration, AgentRuntimeSlot,
    AgentRuntimeSlotState,
};
pub use runtime_topology::{
    is_production_kernel_profile, is_production_kernel_profile_from_env,
    kernel_profile_id_from_env, mock_provider_invocation_allowed,
    mock_provider_invocation_allowed_from_env, mock_provider_override_disabled_from_env,
    mock_provider_override_enabled_from_env, normalize_kernel_profile_id, ALLOW_MOCK_PROVIDERS_ENV,
    KERNEL_ENVIRONMENT_ENV, KERNEL_PROFILE_ID_ENV,
};
pub use session_activity::{
    ProviderSessionActivityProvider, ProviderSessionActivitySink, SessionActivityEvidenceKind,
    SessionActivityFreshness, SessionActivityInteractionHint, SessionActivitySnapshot,
    SessionActivityState,
};
pub use skill::{
    parse_skill_markdown_frontmatter, AgentSkillDescriptor, AgentSkillInvocationMode,
    AgentSkillProvider, AgentSkillRequest, AgentSkillResult, AgentSkillStatus, SkillContentFile,
    SkillContentLayer, SkillContentLayout, SkillMarkdownFrontmatter, SkillVisibility,
};
pub use stream_event::{
    stream_event_with_trace, AgentStreamEvent, AgentStreamSink, CancelledEvent,
    CompactBoundaryEvent, CostEvent, EndedEvent, ErrorEvent, InMemoryAgentStreamSink,
    KernelEventStreamSink, MessageDeltaEvent, MessageDeltaKind, MessageStartEvent,
    MessageStopEvent, ProgressEvent, RateLimitEvent, RateLimitStatus, ResultEvent,
    SessionInitEvent, StatusEvent, StreamStatusLevel, ToolCallDeltaEvent, ToolCallStartEvent,
    ToolCallStopEvent, ToolResultEvent, UsageEvent, AGENT_STREAM_EVENT_FAMILY,
};
pub use task_scheduling::{
    ScheduleQuery, ScheduleResult, ScheduleState, ScheduledTask, TaskPriority,
    TaskSchedulingProvider, TaskTrigger, TriggerKind,
};
pub use telemetry::{
    AuditRecord, TelemetryLogLevel, TelemetryLogRecord, TelemetryMetric, TelemetryMetricKind,
    TelemetryProvider, TelemetrySpan, TelemetrySpanStatus,
};
pub use tool::{
    ApprovedToolExecution, SideEffectLevel, ToolCall, ToolCallStatus, ToolCancellationRequest,
    ToolCancellationResponse, ToolDescriptor, ToolExecutionRequest, ToolExecutionResponse,
    ToolExecutionService, ToolProvider, ToolResult, ToolSchema, ToolStreamChunk,
    ToolStreamExecutionResponse,
};

pub const AGENT_KERNEL_SPEC_VERSION: &str = "0.1.0";

pub const AGENT_MANIFEST_SCHEMA: &str =
    include_str!("../../specs/schemas/agent-manifest.schema.json");
pub const AGENT_DEFINITION_SCHEMA: &str =
    include_str!("../../specs/schemas/agent-definition.schema.json");
pub const AGENT_CARD_SCHEMA: &str = include_str!("../../specs/schemas/agent-card.schema.json");
pub const PROVIDER_MANIFEST_SCHEMA: &str =
    include_str!("../../specs/schemas/provider-manifest.schema.json");
pub const CAPABILITY_MANIFEST_SCHEMA: &str =
    include_str!("../../specs/schemas/capability-manifest.schema.json");
pub const AGENT_CONFIGURATION_SPEC_SCHEMA: &str =
    include_str!("../../specs/schemas/agent-configuration-spec.schema.json");
pub const AGENT_CONFIGURATION_PROFILE_SCHEMA: &str =
    include_str!("../../specs/schemas/agent-configuration-profile.schema.json");
pub const AGENT_CONFIGURATION_MIGRATION_SCHEMA: &str =
    include_str!("../../specs/schemas/agent-configuration-migration.schema.json");
pub const KERNEL_CONFORMANCE_REPORT_SCHEMA: &str =
    include_str!("../../specs/schemas/kernel-conformance-report.schema.json");
pub const AGENT_RUNTIME_DIAGNOSTICS_SCHEMA: &str =
    include_str!("../../specs/schemas/agent-runtime-diagnostics.schema.json");
