import {
  AGENT_SDK_OWNER,
  forbiddenAgentApiPrefixesFor
} from '../../../sdks/_shared/agent-sdk-families.mjs';

export function validateOpenApi({ label, content, family, errors }) {
  if (!content) {
    return;
  }
  for (const required of [
    'openapi: 3.1.2',
    `title: SDKWork Agent ${titleKind(family.key)} API`,
    `x-sdkwork-owner: ${AGENT_SDK_OWNER}`,
    `x-sdkwork-api-authority: ${family.authority}`,
    family.apiPrefix,
    'operationId: agents.list',
    'operationId: agents.create',
    'operationId: agents.providerBindings.create',
    'operationId: agents.deployments.create',
    'operationId: knowledgeBases.list',
    'operationId: knowledgeBases.create',
    'operationId: knowledgeBases.retrieve',
    'operationId: knowledgeBases.update',
    'operationId: knowledgeBases.delete',
    'operationId: knowledgeBases.restore',
    'operationId: knowledgeSources.list',
    'operationId: knowledgeSources.create',
    'operationId: knowledgeSources.retrieve',
    'operationId: knowledgeSources.update',
    'operationId: knowledgeSources.delete',
    'operationId: knowledgeSources.restore',
    'operationId: knowledgeList.list',
    'operationId: knowledgeDocuments.create',
    'operationId: knowledgeDocuments.update',
    'operationId: knowledgeDocuments.delete',
    'operationId: knowledgeDocuments.restore',
    'operationId: knowledgeRead.read',
    'operationId: knowledgeSearch.search',
    'operationId: knowledgeChunks.list',
    'operationId: knowledgeChunks.create',
    'operationId: knowledgeChunks.retrieve',
    'operationId: knowledgeIndexes.list',
    'operationId: knowledgeIndexes.upsert',
    'operationId: knowledgeIndexes.retrieve',
    'operationId: knowledgeBindings.list',
    'operationId: knowledgeBindings.create',
    'operationId: knowledgeBindings.retrieve',
    'operationId: knowledgeSyncJobs.list',
    'operationId: knowledgeSyncJobs.create',
    'operationId: knowledgeSyncJobs.retrieve',
    'operationId: knowledgeSyncJobs.start',
    'operationId: knowledgeSyncJobs.complete',
    'operationId: knowledgeSyncJobs.fail',
    'operationId: knowledgeSyncJobs.cancel',
    'operationId: memoryStores.create',
    'operationId: memoryStores.retrieve',
    'operationId: memoryStores.update',
    'operationId: memoryProfiles.create',
    'operationId: memoryProfiles.retrieve',
    'operationId: memoryBindings.create',
    'operationId: memoryBindings.retrieve',
    'operationId: memoryNamespaces.create',
    'operationId: memoryNamespaces.retrieve',
    'operationId: memoryRecords.list',
    'operationId: memoryRecords.create',
    'operationId: memoryRecords.retrieve',
    'operationId: memoryRecords.delete',
    'operationId: memoryRecords.restore',
    'operationId: memorySources.list',
    'operationId: memorySources.create',
    'operationId: memoryRelations.list',
    'operationId: memoryRelations.create',
    'operationId: memoryRetrievalIndexes.list',
    'operationId: memoryRetrievalIndexes.upsert',
    'UpdateKnowledgeBaseRequest:',
    'UpdateKnowledgeSourceRequest:',
    'UpdateKnowledgeDocumentRequest:',
    'StartKnowledgeSyncJobRequest:',
    'CompleteKnowledgeSyncJobRequest:',
    'FailKnowledgeSyncJobRequest:',
    'CancelKnowledgeSyncJobRequest:',
    'CreateMemoryStoreRequest:',
    'UpdateMemoryStoreRequest:',
    'CreateMemoryProfileRequest:',
    'CreateMemoryBindingRequest:',
    'CreateMemoryNamespaceRequest:',
    'CreateMemoryRecordRequest:',
    'CreateMemorySourceRequest:',
    'CreateMemoryRelationRequest:',
    'UpsertMemoryRetrievalIndexRequest:',
    'MemoryStoreKind:',
    'MemoryIndexKind:',
    'MemoryBindingScopeKind:',
    'MemoryNamespaceKind:',
    'MemoryRecordKind:',
    'MemorySourceKind:',
    'MemoryRelationKind:',
    'MemoryStoreRecord:',
    'MemoryProfileRecord:',
    'MemoryBindingRecord:',
    'MemoryNamespaceRecord:',
    'MemoryRecord:',
    'MemorySourceRecord:',
    'MemoryRelationRecord:',
    'MemoryRetrievalIndexRecord:',
    'KnowledgeSourceIdPath:',
    'KnowledgeChunkIdPath:',
    'KnowledgeIndexIdPath:',
    'KnowledgeBindingIdPath:',
    'KnowledgeSyncJobIdPath:',
    'MemoryStoreIdPath:',
    'MemoryProfileIdPath:',
    'MemoryBindingIdPath:',
    'MemoryNamespaceIdPath:',
    'MemoryIdPath:',
    'x-sdkwork-resource: knowledgeBases',
    'x-sdkwork-resource: knowledgeSources',
    'x-sdkwork-resource: knowledgeDocuments',
    'x-sdkwork-resource: knowledgeList',
    'x-sdkwork-resource: knowledgeRead',
    'x-sdkwork-resource: knowledgeSearch',
    'x-sdkwork-resource: knowledgeChunks',
    'x-sdkwork-resource: knowledgeIndexes',
    'x-sdkwork-resource: knowledgeBindings',
    'x-sdkwork-resource: knowledgeSyncJobs',
    'x-sdkwork-resource: memoryStores',
    'x-sdkwork-resource: memoryProfiles',
    'x-sdkwork-resource: memoryBindings',
    'x-sdkwork-resource: memoryNamespaces',
    'x-sdkwork-resource: memoryRecords',
    'x-sdkwork-resource: memorySources',
    'x-sdkwork-resource: memoryRelations',
    'x-sdkwork-resource: memoryRetrievalIndexes',
    'x-sdkwork-permission: agent.business.knowledge.base.list',
    'x-sdkwork-permission: agent.business.knowledge.base.create',
    'x-sdkwork-permission: agent.business.knowledge.base.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.base.update',
    'x-sdkwork-permission: agent.business.knowledge.base.delete',
    'x-sdkwork-permission: agent.business.knowledge.base.restore',
    'x-sdkwork-permission: agent.business.knowledge.source.list',
    'x-sdkwork-permission: agent.business.knowledge.source.create',
    'x-sdkwork-permission: agent.business.knowledge.source.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.source.update',
    'x-sdkwork-permission: agent.business.knowledge.source.delete',
    'x-sdkwork-permission: agent.business.knowledge.source.restore',
    'x-sdkwork-permission: agent.business.knowledge.list',
    'x-sdkwork-permission: agent.business.knowledge.document.create',
    'x-sdkwork-permission: agent.business.knowledge.document.update',
    'x-sdkwork-permission: agent.business.knowledge.document.delete',
    'x-sdkwork-permission: agent.business.knowledge.document.restore',
    'x-sdkwork-permission: agent.business.knowledge.read',
    'x-sdkwork-permission: agent.business.knowledge.search',
    'x-sdkwork-permission: agent.business.knowledge.chunk.list',
    'x-sdkwork-permission: agent.business.knowledge.chunk.create',
    'x-sdkwork-permission: agent.business.knowledge.chunk.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.index.list',
    'x-sdkwork-permission: agent.business.knowledge.index.upsert',
    'x-sdkwork-permission: agent.business.knowledge.index.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.binding.list',
    'x-sdkwork-permission: agent.business.knowledge.binding.create',
    'x-sdkwork-permission: agent.business.knowledge.binding.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.list',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.create',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.start',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.complete',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.fail',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.cancel',
    'x-sdkwork-permission: agent.business.memory.store.create',
    'x-sdkwork-permission: agent.business.memory.store.retrieve',
    'x-sdkwork-permission: agent.business.memory.store.update',
    'x-sdkwork-permission: agent.business.memory.profile.create',
    'x-sdkwork-permission: agent.business.memory.profile.retrieve',
    'x-sdkwork-permission: agent.business.memory.binding.create',
    'x-sdkwork-permission: agent.business.memory.binding.retrieve',
    'x-sdkwork-permission: agent.business.memory.namespace.create',
    'x-sdkwork-permission: agent.business.memory.namespace.retrieve',
    'x-sdkwork-permission: agent.business.memory.record.list',
    'x-sdkwork-permission: agent.business.memory.record.create',
    'x-sdkwork-permission: agent.business.memory.record.retrieve',
    'x-sdkwork-permission: agent.business.memory.record.delete',
    'x-sdkwork-permission: agent.business.memory.record.restore',
    'x-sdkwork-permission: agent.business.memory.source.list',
    'x-sdkwork-permission: agent.business.memory.source.create',
    'x-sdkwork-permission: agent.business.memory.relation.list',
    'x-sdkwork-permission: agent.business.memory.relation.create',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.list',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.upsert',
    'components:',
    'application/problem+json',
    'Access-Token'
  ]) {
    if (!content.includes(required)) {
      errors.push(`${label} must include ${required}`);
    }
  }
  for (const forbidden of forbiddenAgentApiPrefixesFor(family)) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not include ${forbidden}`);
    }
  }
  if (content.includes('X-Request-Id')) {
    errors.push(`${label} must not expose X-Request-Id`);
  }
  validateOpenApiOwnership({ label, content, family, errors });
  for (const forbidden of [
    'operationId: knowledgeDocuments.list',
    'operationId: knowledgeDocuments.retrieve',
    'operationId: knowledgeChunks.update',
    'operationId: knowledgeChunks.delete',
    'operationId: knowledgeChunks.restore',
    'operationId: knowledgeIndexes.update',
    'operationId: knowledgeIndexes.delete',
    'operationId: knowledgeIndexes.restore',
    'operationId: knowledgeBindings.update',
    'operationId: knowledgeBindings.delete',
    'operationId: knowledgeBindings.restore',
    'x-sdkwork-permission: agent.business.knowledge.document.list',
    'x-sdkwork-permission: agent.business.knowledge.document.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.chunk.update',
    'x-sdkwork-permission: agent.business.knowledge.chunk.delete',
    'x-sdkwork-permission: agent.business.knowledge.chunk.restore',
    'x-sdkwork-permission: agent.business.knowledge.index.update',
    'x-sdkwork-permission: agent.business.knowledge.index.delete',
    'x-sdkwork-permission: agent.business.knowledge.index.restore',
    'x-sdkwork-permission: agent.business.knowledge.binding.update',
    'x-sdkwork-permission: agent.business.knowledge.binding.delete',
    'x-sdkwork-permission: agent.business.knowledge.binding.restore',
    'operationId: memoryStores.delete',
    'operationId: memoryStores.restore',
    'operationId: memoryProfiles.update',
    'operationId: memoryProfiles.delete',
    'operationId: memoryProfiles.restore',
    'operationId: memoryBindings.update',
    'operationId: memoryBindings.delete',
    'operationId: memoryBindings.restore',
    'operationId: memoryNamespaces.update',
    'operationId: memoryNamespaces.delete',
    'operationId: memoryNamespaces.restore',
    'operationId: memoryRecords.update',
    'operationId: memorySources.update',
    'operationId: memorySources.delete',
    'operationId: memorySources.restore',
    'operationId: memoryRelations.update',
    'operationId: memoryRelations.delete',
    'operationId: memoryRelations.restore',
    'operationId: memoryRetrievalIndexes.update',
    'operationId: memoryRetrievalIndexes.delete',
    'operationId: memoryRetrievalIndexes.restore',
    'x-sdkwork-permission: agent.business.memory.store.delete',
    'x-sdkwork-permission: agent.business.memory.store.restore',
    'x-sdkwork-permission: agent.business.memory.profile.update',
    'x-sdkwork-permission: agent.business.memory.profile.delete',
    'x-sdkwork-permission: agent.business.memory.profile.restore',
    'x-sdkwork-permission: agent.business.memory.binding.update',
    'x-sdkwork-permission: agent.business.memory.binding.delete',
    'x-sdkwork-permission: agent.business.memory.binding.restore',
    'x-sdkwork-permission: agent.business.memory.namespace.update',
    'x-sdkwork-permission: agent.business.memory.namespace.delete',
    'x-sdkwork-permission: agent.business.memory.namespace.restore',
    'x-sdkwork-permission: agent.business.memory.record.update',
    'x-sdkwork-permission: agent.business.memory.source.update',
    'x-sdkwork-permission: agent.business.memory.source.delete',
    'x-sdkwork-permission: agent.business.memory.source.restore',
    'x-sdkwork-permission: agent.business.memory.relation.update',
    'x-sdkwork-permission: agent.business.memory.relation.delete',
    'x-sdkwork-permission: agent.business.memory.relation.restore',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.update',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.delete',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.restore'
  ]) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not expose unsupported agent RAG lifecycle contract ${forbidden}`);
    }
  }
}

function validateOpenApiOwnership({ label, content, family, errors }) {
  const lines = content.split(/\r?\n/);
  let currentPath = '';
  let current = null;

  function finishCurrent() {
    if (!current) {
      return;
    }
    const block = current.lines.join('\n');
    if (!block.includes(`      x-sdkwork-owner: ${AGENT_SDK_OWNER}`)) {
      errors.push(
        `${label} ${current.method.toUpperCase()} ${current.pathKey} must declare x-sdkwork-owner ${AGENT_SDK_OWNER}`
      );
    }
    if (!block.includes(`      x-sdkwork-api-authority: ${family.authority}`)) {
      errors.push(
        `${label} ${current.method.toUpperCase()} ${current.pathKey} must declare x-sdkwork-api-authority ${family.authority}`
      );
    }
    current = null;
  }

  for (const line of lines) {
    const pathMatch = /^  (\/[^:]+):\s*$/.exec(line);
    if (pathMatch) {
      finishCurrent();
      currentPath = pathMatch[1];
      continue;
    }

    const methodMatch = /^    (get|put|post|patch|delete|head|options|trace):\s*$/.exec(line);
    if (methodMatch) {
      finishCurrent();
      current = { pathKey: currentPath, method: methodMatch[1], lines: [line] };
      continue;
    }

    if (current) {
      current.lines.push(line);
    }
  }
  finishCurrent();
}

function titleKind(key) {
  switch (key) {
    case 'open':
      return 'Open';
    case 'app':
      return 'App';
    case 'backend':
      return 'Backend';
    default:
      throw new Error(`unknown family key: ${key}`);
  }
}
