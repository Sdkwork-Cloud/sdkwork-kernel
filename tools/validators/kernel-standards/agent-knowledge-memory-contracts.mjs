import path from 'node:path';

const AGENTS_MANAGED_STORE_ROOT = path.join(
  '..',
  'sdkwork-agents',
  'crates',
  'sdkwork-intelligence-agents-service'
);

export function validateAgentKnowledgeMemoryContracts({ kernelRoot, errors, readFileIfExists }) {
  const agentKernelLib = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'lib.rs'));
  const agentDefinitionRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'definition.rs'));
  const agentModelRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'model.rs'));
  const agentKnowledgeRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'knowledge.rs'));
  const agentPolicyRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'policy.rs'));
  const agentRuntimeRust = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-kernel', 'src', 'runtime.rs'));
  const modelProviderSpec = readFileIfExists(path.join(kernelRoot, 'specs', 'AGENT_MODEL_PROVIDER_SPI_SPEC.md'));
  const knowledgeProviderSpec = readFileIfExists(path.join(kernelRoot, 'specs', 'AGENT_KNOWLEDGE_PROVIDER_SPI_SPEC.md'));
  const agentKernelSpec = readFileIfExists(path.join(kernelRoot, 'specs', 'AGENT_KERNEL_SPEC.md'));
  const agentManifestSpec = readFileIfExists(path.join(kernelRoot, 'specs', 'AGENT_MANIFEST_SPEC.md'));
  const agentSecurityPolicySpec = readFileIfExists(
    path.join(kernelRoot, 'specs', 'AGENT_SECURITY_POLICY_SPEC.md')
  );
  const agentsManagedStoreRoot = path.resolve(kernelRoot, AGENTS_MANAGED_STORE_ROOT);
  const agentsCompositionDatabaseSpec = readFileIfExists(
    path.join(agentsManagedStoreRoot, 'specs', 'AGENTS_AI_COMPOSITION_DATABASE_SPEC.md')
  );
  const agentsManagedStoreDatabaseSpec = readFileIfExists(
    path.join(agentsManagedStoreRoot, 'specs', 'AGENTS_MANAGED_STORE_DATABASE_SPEC.md')
  );
  const agentsManagedStoreApi = readFileIfExists(path.join(agentsManagedStoreRoot, 'src', 'api.rs'));
  const agentsManagedStoreLib = readFileIfExists(path.join(agentsManagedStoreRoot, 'src', 'lib.rs'));
  const agentsManagedStorePostgresSql = readFileIfExists(
    path.join(agentsManagedStoreRoot, 'specs', 'sql', 'agents_managed_store_postgres.sql')
  );

  if (!agentsCompositionDatabaseSpec) {
    errors.push(
      'sdkwork-agents composition database spec must exist at ../sdkwork-agents/crates/sdkwork-intelligence-agents-service/specs/AGENTS_AI_COMPOSITION_DATABASE_SPEC.md'
    );
  }

  if (!agentsManagedStoreDatabaseSpec?.includes('AGENTS_AI_COMPOSITION_DATABASE_SPEC.md')) {
    errors.push(
      'sdkwork-agents managed store database spec must redirect to AGENTS_AI_COMPOSITION_DATABASE_SPEC.md'
    );
  }

  for (const [label, content, requiredText] of [
    ['agent lib exports ModelDescriptor', agentKernelLib, 'ModelDescriptor'],
    ['agent lib exports AgentDefinition', agentKernelLib, 'AgentDefinition'],
    ['agent lib exports KnowledgeProvider', agentKernelLib, 'KnowledgeProvider'],
    ['agent definition defines provider binding', agentDefinitionRust, 'pub struct AgentProviderBinding'],
    ['agent definition defines model selection policy', agentDefinitionRust, 'pub struct ModelSelectionPolicy'],
    ['agent definition defines tool call policy', agentDefinitionRust, 'pub struct ToolCallPolicy'],
    ['agent definition defines memory strategy', agentDefinitionRust, 'pub struct MemoryStrategy'],
    ['model SPI defines ModelDescriptor', agentModelRust, 'pub struct ModelDescriptor'],
    ['model request supports model_id', agentModelRust, 'pub model_id: Option<String>'],
    ['model provider exposes list_models', agentModelRust, 'fn list_models(&self) -> Vec<ModelDescriptor>'],
    ['knowledge SPI defines KnowledgeProvider', agentKnowledgeRust, 'pub trait KnowledgeProvider'],
    ['knowledge SPI supports non-vector retrieval methods', agentKnowledgeRust, 'Graph'],
    ['knowledge SPI declares knowledge.search capability', agentKnowledgeRust, '"knowledge.search"'],
    ['knowledge SPI declares knowledge.read capability', agentKnowledgeRust, '"knowledge.read"'],
    ['knowledge SPI declares knowledge.list capability', agentKnowledgeRust, '"knowledge.list"'],
    ['policy category defines KnowledgeList', agentPolicyRust, 'KnowledgeList'],
    ['policy category maps knowledge.list', agentPolicyRust, 'Self::KnowledgeList => "knowledge.list"'],
    ['runtime registers knowledge providers', agentRuntimeRust, 'register_knowledge_provider'],
    ['runtime negotiates model.catalog metadata', agentRuntimeRust, '"model.catalog"'],
    ['runtime negotiates knowledge.search metadata', agentRuntimeRust, '"knowledge.search"'],
    ['runtime negotiates knowledge.read metadata', agentRuntimeRust, '"knowledge.read"'],
    ['runtime negotiates knowledge.list metadata', agentRuntimeRust, '"knowledge.list"'],
    ['runtime maps knowledge.list to KnowledgeList policy category', agentRuntimeRust, 'PolicyCategory::KnowledgeList'],
    ['model provider spec documents model catalog', modelProviderSpec, 'ModelDescriptor'],
    ['model provider spec documents request model_id', modelProviderSpec, 'model_id'],
    ['knowledge provider spec documents provider family', knowledgeProviderSpec, 'provider_family: knowledge'],
    ['knowledge provider spec documents knowledge.search', knowledgeProviderSpec, '`knowledge.search`'],
    ['knowledge provider spec documents knowledge.read', knowledgeProviderSpec, '`knowledge.read`'],
    ['knowledge provider spec documents knowledge.list', knowledgeProviderSpec, '`knowledge.list`'],
    ['knowledge provider spec documents same-named policy categories', knowledgeProviderSpec, 'same-named policy category'],
    ['knowledge provider spec documents RAG boundary', knowledgeProviderSpec, 'KnowledgeProvider -> context selection/assembly -> ModelProvider'],
    ['knowledge provider spec documents soft-delete lifecycle', knowledgeProviderSpec, 'soft-deleted or otherwise unavailable'],
    ['knowledge provider spec documents retrieval scope integrity', knowledgeProviderSpec, 'Retrieval indexes must preserve scope integrity'],
    ['agent kernel spec documents knowledge.list policy category', agentKernelSpec, '`knowledge.list`'],
    ['agent manifest spec documents knowledge.list policy category', agentManifestSpec, '`knowledge.list` policy'],
    ['agent security policy spec documents knowledge.list policy category', agentSecurityPolicySpec, '`knowledge.list`'],
    ['agents composition API exposes compositionSlots.list', agentsManagedStoreApi, 'agents.compositionSlots.list'],
    ['agents composition API exposes compositionSlots.create', agentsManagedStoreApi, 'agents.compositionSlots.create'],
    ['agents composition API exposes compositionSlots.retrieve', agentsManagedStoreApi, 'agents.compositionSlots.retrieve'],
    ['agents composition API exposes compositionSlots.update', agentsManagedStoreApi, 'agents.compositionSlots.update'],
    ['agents composition API exposes compositionSlots.delete', agentsManagedStoreApi, 'agents.compositionSlots.delete']
  ]) {
    if (!content?.includes(requiredText)) {
      errors.push(`${label} must include ${requiredText}`);
    }
  }

  for (const tableName of [
    'ai_app_registry',
    'ai_agent',
    'ai_agent_runtime_binding',
    'ai_agent_deployment',
    'ai_agent_composition_slot',
    'ai_agent_audit_event',
    'ai_agent_outbox_event'
  ]) {
    if (!agentsCompositionDatabaseSpec?.includes(tableName)) {
      errors.push(`agents composition database spec must include ${tableName}`);
    }
    if (!agentsManagedStorePostgresSql?.includes(`CREATE TABLE IF NOT EXISTS ${tableName}`)) {
      errors.push(`agents managed store postgres SQL must define ${tableName}`);
    }
  }

  for (const legacyTable of [
    'a_agent_knowledge_base',
    'a_agent_memory_store'
  ]) {
    if (agentsManagedStorePostgresSql?.includes(`CREATE TABLE IF NOT EXISTS ${legacyTable}`)) {
      errors.push(`agents managed store postgres SQL must not define inline domain table ${legacyTable}`);
    }
  }

  for (const auditAction of [
    'composition_slot_created',
    'composition_slot_updated',
    'composition_slot_deleted'
  ]) {
    if (!agentsCompositionDatabaseSpec?.includes(auditAction)) {
      errors.push(`agents composition database spec must document audit action ${auditAction}`);
    }
  }

  for (const dtoName of [
    'AgentCompositionSlotCreateRequestDto',
    'AgentCompositionSlotUpdateRequestDto',
    'AgentCompositionSlotDeleteRequestDto',
    'AgentCompositionSlotRecordDto',
    'AgentCompositionSlotResponseDto',
    'AgentCompositionSlotListResponseDto'
  ]) {
    if (!agentsManagedStoreLib?.includes(dtoName)) {
      errors.push(`agents managed store crate root must export ${dtoName}`);
    }
  }

  if (!agentsCompositionDatabaseSpec?.includes('sdkwork-knowledgebase')) {
    errors.push('agents composition database spec must document sdkwork-knowledgebase ownership');
  }
  if (!agentsCompositionDatabaseSpec?.includes('sdkwork-memory')) {
    errors.push('agents composition database spec must document sdkwork-memory ownership');
  }
  if (!agentsCompositionDatabaseSpec?.includes('ai_agent_composition_slot')) {
    errors.push('agents composition database spec must document ai_agent_composition_slot');
  }
}