mod chat;
mod collaboration;
mod configuration;
mod conformance;
mod context_memory;
mod definition;
mod error;
mod event;
mod execution;
mod host;
mod installation;
mod knowledge;
mod lifecycle;
mod manifest;
mod mcp;
mod message;
mod model;
mod package;
mod planning;
mod policy;
mod protocol;
mod runtime;
mod runtime_host;
mod skill;
mod telemetry;
mod tool;

pub use chat::{
    agent_chat_rpc_adapter_manifest, AgentChatKnowledgeQuery, AgentChatMemoryQuery,
    AgentChatRequest, AgentChatResponse, AgentChatRpcAdapter, AgentChatRpcHandler,
    AgentChatService,
};
pub use collaboration::{
    AgentCard, AgentCollaborationProvider, AgentDelegation, AgentHandoffRequest, AgentHandoffResult,
};
pub use configuration::{
    AgentConfigEntry, AgentConfigField, AgentConfigSection, AgentConfigSectionKind,
    AgentConfigValue, AgentConfigValueKind, AgentConfiguration, AgentConfigurationInvalidField,
    AgentConfigurationProfile, AgentConfigurationProfileStatus, AgentConfigurationProvider,
    AgentConfigurationSpec, AgentConfigurationStore, AgentConfigurationStoreAction,
    AgentConfigurationStoreRecord, AgentConfigurationUpgradePlan, AgentConfigurationUpgradeRequest,
    AgentConfigurationValidation, AgentProfileArchiveRequest, AgentSecretBinding,
    AgentSecretBindingKind, ConfigurationMigrationStep, ConfigurationMigrationStepKind,
};
pub use conformance::{
    AgentRuntimeConformanceProfile, KernelConformanceCase, KernelConformanceCaseStatus,
    KernelConformanceReport,
};
pub use context_memory::{
    ContextFrame, ContextProvider, MemoryProvider, MemoryRecord, MemoryScope,
    RedactionClassification, TrustLevel,
};
pub use definition::{
    AgentDefinition, AgentProviderBinding, AgentProviderBindingMode, AgentProviderFamily,
    MemoryStrategy, ModelSelectionPolicy, ToolCallPolicy,
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
pub use host::{
    EnvironmentRequest, ExecutorRequest, FilesystemOperation, FilesystemRequest, FilesystemResult,
    HostEnvPolicy, HostPathPolicy, HostProvider, NetworkRequest, NetworkResult, ProcessRequest,
    ProcessResult, SecretRef, SecretValue, StorageRequest, TimeRequest,
};
pub use installation::{
    AgentInstallPlan, AgentInstallReport, AgentInstallRequest, AgentInstallStatus,
    AgentInstallStep, AgentInstallStepKind, AgentInstaller, AgentPackageSource,
    AgentUninstallReport, AgentUninstallRequest, AgentUpgradePlan, AgentUpgradeReport,
    AgentUpgradeRequest,
};
pub use knowledge::{
    KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, KnowledgeSearchResult,
};
pub use lifecycle::{
    AgentRun, AgentSession, AgentStep, AgentTask, RunState, SessionState, StepState, TaskState,
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
pub use model::{
    ModelCancellationRequest, ModelCancellationResponse, ModelDescriptor, ModelExecutionRequest,
    ModelExecutionResponse, ModelExecutionService, ModelProvider, ModelRequest, ModelResponse,
    ModelResponseFormat, ModelStatus, ModelStreamChunk, ModelStreamExecutionResponse,
    ModelStructuredOutputValidation, ModelUsage,
};
pub use package::{
    AgentPackageLifecycle, AgentPackageManifest, AgentPackageProviderBinding,
    AgentPackageVersionCompatibility,
};
pub use planning::{Action, ActionKind, ActionStatus, Observation, Plan, PlanningProvider};
pub use policy::{
    PolicyCategory, PolicyDecision, PolicyDecisionConstraint, PolicyDecisionValue, PolicyProvider,
    PolicyRequest, PolicySubject,
};
pub use protocol::{
    ProtocolAdapter, ProtocolAdapterAuthMode, ProtocolAdapterManifest, ProtocolAdapterRequest,
    ProtocolAdapterResponse, ProtocolAdapterStreamingSupport, ProtocolError, ProtocolFamily,
    ProtocolObjectEnvelope, ProtocolObjectKind, ProtocolObjectMapper, ProtocolSseEvent,
    ProtocolStreamUpdate, ProtocolTransport, StandardProtocolObjectMapper,
};
pub use runtime::{
    AgentProviderDiagnostic, AgentRuntime, AgentRuntimeDiagnostics, RuntimeBootstrapReport,
    RuntimeBuilder, RuntimeState,
};
pub use runtime_host::{
    AgentKernelHost, AgentRuntimeExecutionHandle, AgentRuntimeRegistration, AgentRuntimeSlot,
    AgentRuntimeSlotState,
};
pub use skill::{
    AgentSkillDescriptor, AgentSkillInvocationMode, AgentSkillProvider, AgentSkillRequest,
    AgentSkillResult, AgentSkillStatus,
};
pub use telemetry::{
    AuditRecord, TelemetryLogLevel, TelemetryLogRecord, TelemetryMetric, TelemetryMetricKind,
    TelemetryProvider, TelemetrySpan, TelemetrySpanStatus,
};
pub use tool::{
    SideEffectLevel, ToolCall, ToolCallStatus, ToolCancellationRequest, ToolCancellationResponse,
    ToolDescriptor, ToolExecutionRequest, ToolExecutionResponse, ToolExecutionService,
    ToolProvider, ToolResult, ToolSchema, ToolStreamChunk, ToolStreamExecutionResponse,
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
