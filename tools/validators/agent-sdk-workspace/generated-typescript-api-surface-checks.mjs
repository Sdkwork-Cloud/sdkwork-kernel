export function validateGeneratedAgentApi({ label, content, family, errors }) {
  if (!content) {
    return;
  }
  if (family.key === 'internal') {
    validateInternalGeneratedAgentApi({ label, content, family, errors });
    return;
  }
  const scopeFreeCallSurface = usesScopeFreeCallSurface(family);
  for (const required of [
    'export class AiMemoryStoresApi',
    'export class AiMemoryProfilesApi',
    'export class AiMemoryBindingsApi',
    'export class AiMemoryNamespacesApi',
    'export class AiMemoryRecordsApi',
    'export class AiMemorySourcesApi',
    'export class AiMemoryRelationsApi',
    'export class AiMemoryRetrievalIndexesApi',
    'async create(body: CreateMemoryStoreRequest',
    'async retrieve(memoryStoreId: string',
    'async update(memoryStoreId: string, body: UpdateMemoryStoreRequest',
    'async create(memoryStoreId: string, body: CreateMemoryProfileRequest',
    'async retrieve(memoryProfileId: string',
    'async create(memoryProfileId: string, body: CreateMemoryBindingRequest',
    'async retrieve(memoryBindingId: string',
    'async create(body: CreateMemoryNamespaceRequest',
    'async retrieve(memoryNamespaceId: string',
    'async list(memoryNamespaceId: string',
    'async create(memoryNamespaceId: string, body: CreateMemoryRecordRequest',
    'async retrieve(memoryId: string',
    'async delete(memoryId: string',
    'async restore(memoryId: string, body: RestoreAgentRequest',
    scopeFreeCallSurface
      ? 'async list(memoryId: string, params?: AiMemorySourcesListParams'
      : 'async list(memoryId: string, params: AiMemorySourcesListParams',
    'async create(memoryId: string, body: CreateMemorySourceRequest',
    scopeFreeCallSurface
      ? 'async list(memoryId: string, params?: AiMemoryRelationsListParams'
      : 'async list(memoryId: string, params: AiMemoryRelationsListParams',
    'async create(memoryId: string, body: CreateMemoryRelationRequest',
    scopeFreeCallSurface
      ? 'async list(memoryId: string, params?: AiMemoryRetrievalIndexesListParams'
      : 'async list(memoryId: string, params: AiMemoryRetrievalIndexesListParams',
    'async upsert(body: UpsertMemoryRetrievalIndexRequest',
    'CreateMemoryStoreRequest',
    'UpdateMemoryStoreRequest',
    'CreateMemoryProfileRequest',
    'CreateMemoryBindingRequest',
    'CreateMemoryNamespaceRequest',
    'CreateMemoryRecordRequest',
    'CreateMemorySourceRequest',
    'CreateMemoryRelationRequest',
    'UpsertMemoryRetrievalIndexRequest',
    'MemoryStoreResponse',
    'MemoryProfileResponse',
    'MemoryBindingResponse',
    'MemoryNamespaceResponse',
    'MemoryRecordResponse',
    'MemoryRecordListResponse',
    'MemorySourceResponse',
    'MemorySourceListResponse',
    'MemoryRelationResponse',
    'MemoryRelationListResponse',
    'MemoryRetrievalIndexResponse',
    'MemoryRetrievalIndexListResponse',
    'public readonly memoryStores: AiMemoryStoresApi',
    'public readonly memoryProfiles: AiMemoryProfilesApi',
    'public readonly memoryBindings: AiMemoryBindingsApi',
    'public readonly memoryNamespaces: AiMemoryNamespacesApi',
    'public readonly memoryRecords: AiMemoryRecordsApi',
    'public readonly memorySources: AiMemorySourcesApi',
    'public readonly memoryRelations: AiMemoryRelationsApi',
    'public readonly memoryRetrievalIndexes: AiMemoryRetrievalIndexesApi',
    'export class AiKnowledgeBasesApi',
    'export class AiKnowledgeSourcesApi',
    'export class AiKnowledgeListApi',
    'export class AiKnowledgeDocumentsApi',
    'export class AiKnowledgeReadApi',
    'export class AiKnowledgeSearchApi',
    'export class AiKnowledgeChunksApi',
    'export class AiKnowledgeIndexesApi',
    'export class AiKnowledgeBindingsApi',
    'export class AiKnowledgeSyncJobsApi',
    scopeFreeCallSurface
      ? 'async list(params?: AiKnowledgeBasesListParams): Promise<KnowledgeBaseListResponse>'
      : 'async list(params: AiKnowledgeBasesListParams): Promise<KnowledgeBaseListResponse>',
    scopeFreeCallSurface
      ? 'async create(body: CreateKnowledgeBaseRequest): Promise<KnowledgeBaseResponse>'
      : 'async create(body: CreateKnowledgeBaseRequest, params: AiKnowledgeBasesCreateParams): Promise<KnowledgeBaseResponse>',
    'async retrieve(knowledgeBaseId: string',
    'async update(knowledgeBaseId: string, body: UpdateKnowledgeBaseRequest',
    'async delete(knowledgeBaseId: string',
    'async restore(knowledgeBaseId: string, body: RestoreAgentRequest',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeSourcesListParams): Promise<KnowledgeSourceListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeSourcesListParams): Promise<KnowledgeSourceListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeSourceRequest): Promise<KnowledgeSourceResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeSourceRequest, params: AiKnowledgeSourcesCreateParams): Promise<KnowledgeSourceResponse>',
    'async retrieve(knowledgeSourceId: string',
    'async update(knowledgeSourceId: string, body: UpdateKnowledgeSourceRequest',
    'async delete(knowledgeSourceId: string',
    'async restore(knowledgeSourceId: string, body: RestoreAgentRequest',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeDocumentRequest): Promise<KnowledgeDocumentResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeDocumentRequest, params: AiKnowledgeDocumentsCreateParams): Promise<KnowledgeDocumentResponse>',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeListListParams): Promise<KnowledgeDocumentListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeListListParams): Promise<KnowledgeDocumentListResponse>',
    scopeFreeCallSurface
      ? 'async read(knowledgeDocumentId: string): Promise<KnowledgeDocumentResponse>'
      : 'async read(knowledgeDocumentId: string, params: AiKnowledgeReadReadParams): Promise<KnowledgeDocumentResponse>',
    scopeFreeCallSurface
      ? 'async search(knowledgeBaseId: string, body: SearchKnowledgeRequest): Promise<KnowledgeSearchResponse>'
      : 'async search(knowledgeBaseId: string, body: SearchKnowledgeRequest, params: AiKnowledgeSearchSearchParams): Promise<KnowledgeSearchResponse>',
    'async update(knowledgeDocumentId: string, body: UpdateKnowledgeDocumentRequest',
    'async delete(knowledgeDocumentId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeDocumentId: string, params?: AiKnowledgeChunksListParams): Promise<KnowledgeChunkListResponse>'
      : 'async list(knowledgeDocumentId: string, params: AiKnowledgeChunksListParams): Promise<KnowledgeChunkListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeDocumentId: string, body: CreateKnowledgeChunkRequest): Promise<KnowledgeChunkResponse>'
      : 'async create(knowledgeDocumentId: string, body: CreateKnowledgeChunkRequest, params: AiKnowledgeChunksCreateParams): Promise<KnowledgeChunkResponse>',
    'async retrieve(knowledgeChunkId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeDocumentId: string, params?: AiKnowledgeIndexesListParams): Promise<KnowledgeIndexListResponse>'
      : 'async list(knowledgeDocumentId: string, params: AiKnowledgeIndexesListParams): Promise<KnowledgeIndexListResponse>',
    scopeFreeCallSurface
      ? 'async upsert(body: UpsertKnowledgeIndexRequest): Promise<KnowledgeIndexResponse>'
      : 'async upsert(body: UpsertKnowledgeIndexRequest, params: AiKnowledgeIndexesUpsertParams): Promise<KnowledgeIndexResponse>',
    'async retrieve(knowledgeIndexId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeBindingsListParams): Promise<KnowledgeBindingListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeBindingsListParams): Promise<KnowledgeBindingListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeBindingRequest): Promise<KnowledgeBindingResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeBindingRequest, params: AiKnowledgeBindingsCreateParams): Promise<KnowledgeBindingResponse>',
    'async retrieve(knowledgeBindingId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeSyncJobsListParams): Promise<KnowledgeSyncJobListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeSyncJobsListParams): Promise<KnowledgeSyncJobListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsCreateParams): Promise<KnowledgeSyncJobResponse>',
    'async retrieve(syncJobId: string',
    'async start(syncJobId: string, body: StartKnowledgeSyncJobRequest',
    'async complete(syncJobId: string, body: CompleteKnowledgeSyncJobRequest',
    'async fail(syncJobId: string, body: FailKnowledgeSyncJobRequest',
    'async cancel(syncJobId: string, body: CancelKnowledgeSyncJobRequest',
    'CreateKnowledgeBaseRequest',
    'CreateKnowledgeSourceRequest',
    'CreateKnowledgeDocumentRequest',
    'CreateKnowledgeChunkRequest',
    'SearchKnowledgeRequest',
    'UpsertKnowledgeIndexRequest',
    'CreateKnowledgeBindingRequest',
    'CreateKnowledgeSyncJobRequest',
    'UpdateKnowledgeBaseRequest',
    'UpdateKnowledgeSourceRequest',
    'UpdateKnowledgeDocumentRequest',
    'StartKnowledgeSyncJobRequest',
    'CompleteKnowledgeSyncJobRequest',
    'FailKnowledgeSyncJobRequest',
    'CancelKnowledgeSyncJobRequest',
    'KnowledgeBaseListResponse',
    'KnowledgeSourceListResponse',
    'KnowledgeDocumentListResponse',
    'KnowledgeChunkListResponse',
    'KnowledgeIndexListResponse',
    'KnowledgeBindingListResponse',
    'KnowledgeSyncJobListResponse',
    'public readonly knowledgeBases: AiKnowledgeBasesApi',
    'public readonly knowledgeSources: AiKnowledgeSourcesApi',
    'public readonly knowledgeList: AiKnowledgeListApi',
    'public readonly knowledgeDocuments: AiKnowledgeDocumentsApi',
    'public readonly knowledgeRead: AiKnowledgeReadApi',
    'public readonly knowledgeSearch: AiKnowledgeSearchApi',
    'public readonly knowledgeChunks: AiKnowledgeChunksApi',
    'public readonly knowledgeIndexes: AiKnowledgeIndexesApi',
    'public readonly knowledgeBindings: AiKnowledgeBindingsApi',
    'public readonly knowledgeSyncJobs: AiKnowledgeSyncJobsApi'
  ]) {
    if (!content.includes(required)) {
      errors.push(`${label} must include generated SDK surface ${required}`);
    }
  }

  const documentDeleteParams = boundedBlock(
    content,
    'export interface AiKnowledgeDocumentsDeleteParams',
    '}'
  );
  for (const required of [
    ...(scopeFreeCallSurface ? [] : ['tenantId: Int64String;']),
    'expectedVersion?: Int64String;',
    'requestedAt: string;'
  ]) {
    if (!documentDeleteParams.includes(required)) {
      errors.push(`${label} AiKnowledgeDocumentsDeleteParams must include ${required}`);
    }
  }

  const documentDeleteMethod = boundedBlock(
    content,
    'async delete(knowledgeDocumentId: string',
    'async restore(knowledgeDocumentId: string'
  );
  for (const required of [
    ...(scopeFreeCallSurface ? [] : ["{ name: 'tenant_id', value: params.tenantId"]),
    "{ name: 'expected_version', value: params.expectedVersion",
    "{ name: 'requested_at', value: params.requestedAt",
    'return this.client.delete<KnowledgeDocumentResponse>'
  ]) {
    if (!documentDeleteMethod.includes(required)) {
      errors.push(`${label} knowledge document delete method must include ${required}`);
    }
  }
  if (documentDeleteMethod.includes(', body')) {
    errors.push(`${label} knowledge document delete method must not send a request body`);
  }
  if (scopeFreeCallSurface) {
    for (const forbidden of [
      'tenantId:',
      'organizationId:',
      'ownerUserId:',
      "'tenant_id'",
      "'organization_id'",
      "'owner_user_id'",
      'AiKnowledgeBasesCreateParams',
      'AiKnowledgeDocumentsCreateParams',
      'AiKnowledgeReadReadParams',
      'AiKnowledgeSearchSearchParams'
    ]) {
      if (content.includes(forbidden)) {
        errors.push(`${label} call surface must not include caller-provided scope ${forbidden}`);
      }
    }
  }

  for (const forbidden of [
    'async update(knowledgeChunkId: string',
    'async delete(knowledgeChunkId: string',
    'async restore(knowledgeChunkId: string',
    'async update(knowledgeIndexId: string',
    'async delete(knowledgeIndexId: string',
    'async restore(knowledgeIndexId: string',
    'async update(knowledgeBindingId: string',
    'async delete(knowledgeBindingId: string',
    'async restore(knowledgeBindingId: string',
    'async delete(memoryStoreId: string',
    'async restore(memoryStoreId: string',
    'async update(memoryProfileId: string',
    'async delete(memoryProfileId: string',
    'async restore(memoryProfileId: string',
    'async update(memoryBindingId: string',
    'async delete(memoryBindingId: string',
    'async restore(memoryBindingId: string',
    'async update(memoryNamespaceId: string',
    'async delete(memoryNamespaceId: string',
    'async restore(memoryNamespaceId: string',
    'async update(memoryId: string',
    'async update(memorySourceId: string',
    'async delete(memorySourceId: string',
    'async restore(memorySourceId: string',
    'async update(memoryRelationId: string',
    'async delete(memoryRelationId: string',
    'async restore(memoryRelationId: string',
    'async update(memoryIndexId: string',
    'async delete(memoryIndexId: string',
    'async restore(memoryIndexId: string'
  ]) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not expose ${forbidden}`);
    }
  }
}

function boundedBlock(content, startMarker, endMarker) {
  const start = content.indexOf(startMarker);
  if (start < 0) {
    return '';
  }
  const afterStart = content.slice(start);
  const end = afterStart.indexOf(endMarker);
  if (end < 0) {
    return afterStart;
  }
  return afterStart.slice(0, end + endMarker.length);
}

function usesScopeFreeCallSurface(family) {
  return family?.key === 'app' || family?.key === 'open';
}

function validateInternalGeneratedAgentApi({ label, content, errors }) {
  for (const required of [
    'export class IntelligenceRuntimeSnapshotApi',
    'export class IntelligenceRuntimeSessionsApi',
    'export class IntelligenceRuntimeSessionsMessagesApi',
    'async load(params?: IntelligenceRuntimeSnapshotLoadParams, requestOptions?: ApiRequestOptions): Promise<RuntimeSnapshot>',
    'async create(body: CreateSessionRequest',
    'async send(sessionId: string, body: SendMessageRequest',
    'CreateSessionRequest',
    'RuntimeSnapshot',
    'public readonly runtime: IntelligenceRuntimeApi'
  ]) {
    if (!content.includes(required)) {
      errors.push(`${label} missing ${required}`);
    }
  }
}
