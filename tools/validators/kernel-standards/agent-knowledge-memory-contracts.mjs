import path from 'node:path';

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
  const agentBusinessDatabaseSpec = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-business', 'specs', 'AGENT_BUSINESS_DATABASE_SPEC.md')
  );
  const agentBusinessApi = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-business', 'src', 'api.rs')
  );
  const agentBusinessLib = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-business', 'src', 'lib.rs')
  );
  const agentBusinessPostgresSql = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-business', 'specs', 'sql', 'agent_business_postgres.sql')
  );

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
    ['agent business API exposes knowledge.list SDK operation', agentBusinessApi, 'knowledgeList.list'],
    ['agent business API exposes knowledge.read SDK operation', agentBusinessApi, 'knowledgeRead.read'],
    ['agent business API exposes knowledge.search SDK operation', agentBusinessApi, 'knowledgeSearch.search'],
    ['agent business API exposes knowledge base retrieve SDK operation', agentBusinessApi, 'knowledgeBases.retrieve'],
    ['agent business API exposes knowledge base update SDK operation', agentBusinessApi, 'knowledgeBases.update'],
    ['agent business API exposes knowledge base delete SDK operation', agentBusinessApi, 'knowledgeBases.delete'],
    ['agent business API exposes knowledge base restore SDK operation', agentBusinessApi, 'knowledgeBases.restore']
  ]) {
    if (!content.includes(requiredText)) {
      errors.push(`${label} must include ${requiredText}`);
    }
  }

  for (const tableName of [
    'a_agent_knowledge_base',
    'a_agent_knowledge_source',
    'a_agent_knowledge_document',
    'a_agent_knowledge_chunk',
    'a_agent_knowledge_index',
    'a_agent_knowledge_binding',
    'a_agent_knowledge_sync_job'
  ]) {
    if (!agentBusinessDatabaseSpec.includes(tableName)) {
      errors.push(`agent business database spec must include ${tableName}`);
    }
    if (!agentBusinessPostgresSql.includes(`CREATE TABLE IF NOT EXISTS ${tableName}`)) {
      errors.push(`agent business postgres SQL must define ${tableName}`);
    }
  }

  for (const retrievalMode of [
    'exact',
    'keyword',
    'full_text',
    'structured',
    'graph',
    'wiki',
    'rule',
    'vector',
    'hybrid',
    'llm_rerank',
    'external'
  ]) {
    if (!agentBusinessDatabaseSpec.includes(retrievalMode)) {
      errors.push(`agent knowledge database spec must document retrieval mode ${retrievalMode}`);
    }
    ensureSqlBlockIncludes(
      'agent knowledge retrieval index kind constraint',
      agentBusinessPostgresSql,
      'CONSTRAINT ck_a_agent_knowledge_index_kind CHECK (',
      'CONSTRAINT ck_a_agent_knowledge_index_provider_standard CHECK',
      `'${retrievalMode}'`,
      errors
    );
  }

  for (const auditAction of [
    'knowledge_base_created',
    'knowledge_base_updated',
    'knowledge_base_deleted',
    'knowledge_base_restored',
    'knowledge_source_created',
    'knowledge_source_updated',
    'knowledge_source_deleted',
    'knowledge_source_restored',
    'knowledge_document_created',
    'knowledge_document_updated',
    'knowledge_document_deleted',
    'knowledge_document_restored',
    'knowledge_chunk_created',
    'knowledge_index_upserted',
    'knowledge_binding_created',
    'knowledge_sync_job_created',
    'knowledge_sync_job_started',
    'knowledge_sync_job_completed',
    'knowledge_sync_job_failed',
    'knowledge_sync_job_cancelled'
  ]) {
    if (!agentBusinessDatabaseSpec.includes(auditAction)) {
      errors.push(`agent business database spec must document knowledge audit action ${auditAction}`);
    }
    if (!agentBusinessPostgresSql.includes(`'${auditAction}'`)) {
      errors.push(`agent business postgres SQL must allow knowledge audit action ${auditAction}`);
    }
  }

  for (const dtoName of [
    'ListAgentKnowledgeBasesRequestDto',
    'AgentKnowledgeBaseCreateRequestDto',
    'AgentKnowledgeBaseUpdateRequestDto',
    'AgentKnowledgeSourceCreateRequestDto',
    'AgentKnowledgeSourceUpdateRequestDto',
    'AgentKnowledgeDocumentCreateRequestDto',
    'AgentKnowledgeDocumentUpdateRequestDto',
    'AgentKnowledgeChunkCreateRequestDto',
    'AgentKnowledgeIndexUpsertRequestDto',
    'AgentKnowledgeSearchRequestDto',
    'AgentKnowledgeBindingCreateRequestDto',
    'AgentKnowledgeSyncJobCreateRequestDto',
    'AgentKnowledgeBaseRecordDto',
    'AgentKnowledgeSourceRecordDto',
    'AgentKnowledgeDocumentRecordDto',
    'AgentKnowledgeChunkRecordDto',
    'AgentKnowledgeIndexRecordDto',
    'AgentKnowledgeSearchResultDto',
    'AgentKnowledgeBindingRecordDto',
    'AgentKnowledgeSyncJobRecordDto'
  ]) {
    if (!agentBusinessLib.includes(dtoName)) {
      errors.push(`agent business crate root must export ${dtoName}`);
    }
  }

  for (const tableName of [
    'a_agent_memory_store',
    'a_agent_memory_profile',
    'a_agent_memory_binding',
    'a_agent_memory_namespace',
    'a_agent_memory_record',
    'a_agent_memory_source',
    'a_agent_memory_relation',
    'a_agent_memory_retrieval_index',
    'a_agent_memory_access_event',
    'a_agent_memory_compaction_job'
  ]) {
    if (!agentBusinessDatabaseSpec.includes(tableName)) {
      errors.push(`agent business database spec must include ${tableName}`);
    }
    if (!agentBusinessPostgresSql.includes(`CREATE TABLE IF NOT EXISTS ${tableName}`)) {
      errors.push(`agent business postgres SQL must define ${tableName}`);
    }
  }

  if (!agentBusinessPostgresSql.includes('ck_a_agent_memory_retrieval_index_kind')) {
    errors.push('agent business postgres SQL must constrain memory retrieval index kinds');
  }

  for (const retrievalMode of ['keyword', 'sparse', 'vector', 'graph', 'wiki', 'rule', 'hybrid']) {
    if (!agentBusinessDatabaseSpec.includes(retrievalMode)) {
      errors.push(`agent memory database spec must document retrieval mode ${retrievalMode}`);
    }
    ensureSqlBlockIncludes(
      'agent memory retrieval index kind constraint',
      agentBusinessPostgresSql,
      'CONSTRAINT ck_a_agent_memory_retrieval_index_kind CHECK (',
      'CONSTRAINT ck_a_agent_memory_retrieval_index_provider_standard CHECK',
      `'${retrievalMode}'`,
      errors
    );
  }

  for (const auditAction of [
    'memory_store_created',
    'memory_store_updated',
    'memory_profile_created',
    'memory_binding_created',
    'memory_namespace_created',
    'memory_record_created',
    'memory_record_deleted',
    'memory_record_restored',
    'memory_source_created',
    'memory_relation_created',
    'memory_retrieval_index_upserted'
  ]) {
    if (!agentBusinessDatabaseSpec.includes(auditAction)) {
      errors.push(`agent business database spec must document memory audit action ${auditAction}`);
    }
    if (!agentBusinessPostgresSql.includes(`'${auditAction}'`)) {
      errors.push(`agent business postgres SQL must allow memory audit action ${auditAction}`);
    }
  }

  for (const dtoName of [
    'AgentMemoryStoreCreateRequestDto',
    'AgentMemoryStoreUpdateRequestDto',
    'AgentMemoryProfileCreateRequestDto',
    'AgentMemoryBindingCreateRequestDto',
    'AgentMemoryNamespaceCreateRequestDto',
    'AgentMemoryRecordCreateRequestDto',
    'AgentMemorySourceCreateRequestDto',
    'AgentMemoryRelationCreateRequestDto',
    'AgentMemoryRetrievalIndexUpsertRequestDto',
    'AgentMemoryStoreRecordDto',
    'AgentMemoryProfileRecordDto',
    'AgentMemoryBindingRecordDto',
    'AgentMemoryNamespaceRecordDto',
    'AgentMemoryRecordDto',
    'AgentMemorySourceRecordDto',
    'AgentMemoryRelationRecordDto',
    'AgentMemoryRetrievalIndexRecordDto'
  ]) {
    if (!agentBusinessLib.includes(dtoName)) {
      errors.push(`agent business crate root must export ${dtoName}`);
    }
  }

  if (!agentBusinessDatabaseSpec.includes('Memory is learned agent/user/session state')) {
    errors.push('agent business database spec must separate knowledge from memory');
  }
  if (!agentBusinessDatabaseSpec.includes('RAG is provider-neutral and is not vector-only')) {
    errors.push('agent business database spec must state RAG is provider-neutral and not vector-only');
  }
  if (!agentBusinessDatabaseSpec.includes('Active read/list/search/index/binding/sync operations treat soft-deleted')) {
    errors.push('agent business database spec must document active knowledge lifecycle filtering');
  }
  if (!agentBusinessDatabaseSpec.includes('retrieval indexes may be base scoped or document scoped')) {
    errors.push('agent business database spec must document knowledge retrieval index scope rules');
  }
  if (!agentBusinessPostgresSql.includes('ck_a_agent_knowledge_index_chunk_requires_document')) {
    errors.push('agent business postgres SQL must require document scope for chunk-scoped knowledge indexes');
  }
  if (!agentBusinessPostgresSql.includes('WHERE knowledge_document_id IS NOT NULL AND status <> 4')) {
    errors.push('agent business postgres SQL must filter deleted document-scoped knowledge indexes');
  }
}

function ensureSqlBlockIncludes(label, content, startMarker, endMarker, requiredText, errors) {
  const start = content.indexOf(startMarker);
  if (start < 0) {
    errors.push(`${label} must include block start ${startMarker}`);
    return;
  }

  const end = content.indexOf(endMarker, start + startMarker.length);
  if (end < 0) {
    errors.push(`${label} must include block end ${endMarker}`);
    return;
  }

  const block = content.slice(start, end);
  if (!block.includes(requiredText)) {
    errors.push(`${label} must include ${requiredText}`);
  }
}
