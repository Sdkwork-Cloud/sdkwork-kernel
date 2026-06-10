import { agentApiPath } from './paths';
import type { HttpClient } from '../http/client';

import type { ActivateAgentProviderBindingRequest, AgentDeploymentListResponse, AgentDeploymentResponse, AgentListResponse, AgentProviderBindingListResponse, AgentProviderBindingResponse, AgentResponse, AgentRuntimeExecutionResponse, CancelKnowledgeSyncJobRequest, CompleteKnowledgeSyncJobRequest, CreateAgentDeploymentRequest, CreateAgentPreviewResponseRequest, CreateAgentPromptOptimizationRequest, CreateAgentProviderBindingRequest, CreateAgentRequest, CreateKnowledgeBaseRequest, CreateKnowledgeBindingRequest, CreateKnowledgeChunkRequest, CreateKnowledgeDocumentRequest, CreateKnowledgeSourceRequest, CreateKnowledgeSyncJobRequest, CreateMemoryBindingRequest, CreateMemoryNamespaceRequest, CreateMemoryProfileRequest, CreateMemoryRecordRequest, CreateMemoryRelationRequest, CreateMemorySourceRequest, CreateMemoryStoreRequest, FailKnowledgeSyncJobRequest, Int64String, KnowledgeBaseListResponse, KnowledgeBaseResponse, KnowledgeBindingListResponse, KnowledgeBindingResponse, KnowledgeChunkListResponse, KnowledgeChunkResponse, KnowledgeDocumentListResponse, KnowledgeDocumentResponse, KnowledgeIndexListResponse, KnowledgeIndexResponse, KnowledgeSearchResponse, KnowledgeSourceListResponse, KnowledgeSourceResponse, KnowledgeSyncJobListResponse, KnowledgeSyncJobResponse, MemoryBindingResponse, MemoryNamespaceResponse, MemoryProfileResponse, MemoryRecordListResponse, MemoryRecordResponse, MemoryRelationListResponse, MemoryRelationResponse, MemoryRetrievalIndexListResponse, MemoryRetrievalIndexResponse, MemorySourceListResponse, MemorySourceResponse, MemoryStoreResponse, RestoreAgentRequest, SearchKnowledgeRequest, StartKnowledgeSyncJobRequest, UpdateAgentRequest, UpdateKnowledgeBaseRequest, UpdateKnowledgeDocumentRequest, UpdateKnowledgeSourceRequest, UpdateMemoryStoreRequest, UpsertKnowledgeIndexRequest, UpsertMemoryRetrievalIndexRequest } from '../types';


export interface AiMemoryRetrievalIndexesListParams {
  page?: number;
  pageSize?: number;
}

export class AiMemoryRetrievalIndexesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List retrieval indexes for one agent memory record */
  async list(memoryId: string, params?: AiMemoryRetrievalIndexesListParams): Promise<MemoryRetrievalIndexListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRetrievalIndexListResponse>(appendQueryString(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/retrieval_indexes`), query));
  }

/** Upsert an agent memory retrieval index */
  async upsert(body: UpsertMemoryRetrievalIndexRequest): Promise<MemoryRetrievalIndexResponse> {
    return this.client.post<MemoryRetrievalIndexResponse>(agentApiPath(`/ai/memory_retrieval_indexes`), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemoryRelationsListParams {
  page?: number;
  pageSize?: number;
}

export class AiMemoryRelationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List graph relations for one agent memory record */
  async list(memoryId: string, params?: AiMemoryRelationsListParams): Promise<MemoryRelationListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRelationListResponse>(appendQueryString(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/relations`), query));
  }

/** Create a graph relation for one agent memory record */
  async create(memoryId: string, body: CreateMemoryRelationRequest): Promise<MemoryRelationResponse> {
    return this.client.post<MemoryRelationResponse>(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/relations`), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemorySourcesListParams {
  page?: number;
  pageSize?: number;
}

export class AiMemorySourcesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List provenance sources for one agent memory record */
  async list(memoryId: string, params?: AiMemorySourcesListParams): Promise<MemorySourceListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemorySourceListResponse>(appendQueryString(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/sources`), query));
  }

/** Create a provenance source for one agent memory record */
  async create(memoryId: string, body: CreateMemorySourceRequest): Promise<MemorySourceResponse> {
    return this.client.post<MemorySourceResponse>(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/sources`), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemoryRecordsListParams {
  page?: number;
  pageSize?: number;
}

export interface AiMemoryRecordsDeleteParams {
  expectedVersion?: Int64String;
  requestedAt: string;
}

export class AiMemoryRecordsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List agent memory records in one namespace */
  async list(memoryNamespaceId: string, params?: AiMemoryRecordsListParams): Promise<MemoryRecordListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRecordListResponse>(appendQueryString(agentApiPath(`/ai/memory_namespaces/${serializePathParameter(memoryNamespaceId, { name: 'memoryNamespaceId', style: 'simple', explode: false })}/records`), query));
  }

/** Create an agent memory record in one namespace */
  async create(memoryNamespaceId: string, body: CreateMemoryRecordRequest): Promise<MemoryRecordResponse> {
    return this.client.post<MemoryRecordResponse>(agentApiPath(`/ai/memory_namespaces/${serializePathParameter(memoryNamespaceId, { name: 'memoryNamespaceId', style: 'simple', explode: false })}/records`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory record */
  async retrieve(memoryId: string): Promise<MemoryRecordResponse> {
    return this.client.get<MemoryRecordResponse>(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}`));
  }

/** Soft-delete one agent memory record */
  async delete(memoryId: string, params: AiMemoryRecordsDeleteParams): Promise<MemoryRecordResponse> {
    const query = buildQueryString([
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<MemoryRecordResponse>(appendQueryString(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent memory record */
  async restore(memoryId: string, body: RestoreAgentRequest): Promise<MemoryRecordResponse> {
    return this.client.post<MemoryRecordResponse>(agentApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/restore`), body, undefined, undefined, 'application/json');
  }
}

export class AiMemoryNamespacesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory namespace */
  async create(body: CreateMemoryNamespaceRequest): Promise<MemoryNamespaceResponse> {
    return this.client.post<MemoryNamespaceResponse>(agentApiPath(`/ai/memory_namespaces`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory namespace */
  async retrieve(memoryNamespaceId: string): Promise<MemoryNamespaceResponse> {
    return this.client.get<MemoryNamespaceResponse>(agentApiPath(`/ai/memory_namespaces/${serializePathParameter(memoryNamespaceId, { name: 'memoryNamespaceId', style: 'simple', explode: false })}`));
  }
}

export class AiMemoryBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory profile binding */
  async create(memoryProfileId: string, body: CreateMemoryBindingRequest): Promise<MemoryBindingResponse> {
    return this.client.post<MemoryBindingResponse>(agentApiPath(`/ai/memory_profiles/${serializePathParameter(memoryProfileId, { name: 'memoryProfileId', style: 'simple', explode: false })}/bindings`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory binding */
  async retrieve(memoryBindingId: string): Promise<MemoryBindingResponse> {
    return this.client.get<MemoryBindingResponse>(agentApiPath(`/ai/memory_bindings/${serializePathParameter(memoryBindingId, { name: 'memoryBindingId', style: 'simple', explode: false })}`));
  }
}

export class AiMemoryProfilesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory profile for one store */
  async create(memoryStoreId: string, body: CreateMemoryProfileRequest): Promise<MemoryProfileResponse> {
    return this.client.post<MemoryProfileResponse>(agentApiPath(`/ai/memory_stores/${serializePathParameter(memoryStoreId, { name: 'memoryStoreId', style: 'simple', explode: false })}/profiles`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory profile */
  async retrieve(memoryProfileId: string): Promise<MemoryProfileResponse> {
    return this.client.get<MemoryProfileResponse>(agentApiPath(`/ai/memory_profiles/${serializePathParameter(memoryProfileId, { name: 'memoryProfileId', style: 'simple', explode: false })}`));
  }
}

export class AiMemoryStoresApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory store */
  async create(body: CreateMemoryStoreRequest): Promise<MemoryStoreResponse> {
    return this.client.post<MemoryStoreResponse>(agentApiPath(`/ai/memory_stores`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory store */
  async retrieve(memoryStoreId: string): Promise<MemoryStoreResponse> {
    return this.client.get<MemoryStoreResponse>(agentApiPath(`/ai/memory_stores/${serializePathParameter(memoryStoreId, { name: 'memoryStoreId', style: 'simple', explode: false })}`));
  }

/** Update one agent memory store */
  async update(memoryStoreId: string, body: UpdateMemoryStoreRequest): Promise<MemoryStoreResponse> {
    return this.client.patch<MemoryStoreResponse>(agentApiPath(`/ai/memory_stores/${serializePathParameter(memoryStoreId, { name: 'memoryStoreId', style: 'simple', explode: false })}`), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeSyncJobsListParams {
  page?: number;
  pageSize?: number;
}

export class AiKnowledgeSyncJobsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List sync jobs for one agent knowledge base */
  async list(knowledgeBaseId: string, params?: AiKnowledgeSyncJobsListParams): Promise<KnowledgeSyncJobListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeSyncJobListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sync_jobs`), query));
  }

/** Create a sync job for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse> {
    return this.client.post<KnowledgeSyncJobResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sync_jobs`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge sync job */
  async retrieve(syncJobId: string): Promise<KnowledgeSyncJobResponse> {
    return this.client.get<KnowledgeSyncJobResponse>(agentApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}`));
  }

/** Start one agent knowledge sync job */
  async start(syncJobId: string, body: StartKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse> {
    return this.client.post<KnowledgeSyncJobResponse>(agentApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/start`), body, undefined, undefined, 'application/json');
  }

/** Complete one agent knowledge sync job */
  async complete(syncJobId: string, body: CompleteKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse> {
    return this.client.post<KnowledgeSyncJobResponse>(agentApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/complete`), body, undefined, undefined, 'application/json');
  }

/** Fail one agent knowledge sync job */
  async fail(syncJobId: string, body: FailKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse> {
    return this.client.post<KnowledgeSyncJobResponse>(agentApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/fail`), body, undefined, undefined, 'application/json');
  }

/** Cancel one agent knowledge sync job */
  async cancel(syncJobId: string, body: CancelKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse> {
    return this.client.post<KnowledgeSyncJobResponse>(agentApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/cancel`), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeBindingsListParams {
  page?: number;
  pageSize?: number;
}

export class AiKnowledgeBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List bindings for one agent knowledge base */
  async list(knowledgeBaseId: string, params?: AiKnowledgeBindingsListParams): Promise<KnowledgeBindingListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeBindingListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/bindings`), query));
  }

/** Create a binding for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeBindingRequest): Promise<KnowledgeBindingResponse> {
    return this.client.post<KnowledgeBindingResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/bindings`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge binding */
  async retrieve(knowledgeBindingId: string): Promise<KnowledgeBindingResponse> {
    return this.client.get<KnowledgeBindingResponse>(agentApiPath(`/ai/knowledge_bindings/${serializePathParameter(knowledgeBindingId, { name: 'knowledgeBindingId', style: 'simple', explode: false })}`));
  }
}

export interface AiKnowledgeIndexesListParams {
  page?: number;
  pageSize?: number;
}

export class AiKnowledgeIndexesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List indexes for one agent knowledge document */
  async list(knowledgeDocumentId: string, params?: AiKnowledgeIndexesListParams): Promise<KnowledgeIndexListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeIndexListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/indexes`), query));
  }

/** Upsert an agent knowledge retrieval index */
  async upsert(body: UpsertKnowledgeIndexRequest): Promise<KnowledgeIndexResponse> {
    return this.client.post<KnowledgeIndexResponse>(agentApiPath(`/ai/knowledge_indexes`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge retrieval index */
  async retrieve(knowledgeIndexId: string): Promise<KnowledgeIndexResponse> {
    return this.client.get<KnowledgeIndexResponse>(agentApiPath(`/ai/knowledge_indexes/${serializePathParameter(knowledgeIndexId, { name: 'knowledgeIndexId', style: 'simple', explode: false })}`));
  }
}

export interface AiKnowledgeChunksListParams {
  page?: number;
  pageSize?: number;
}

export class AiKnowledgeChunksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List chunks for one agent knowledge document */
  async list(knowledgeDocumentId: string, params?: AiKnowledgeChunksListParams): Promise<KnowledgeChunkListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeChunkListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/chunks`), query));
  }

/** Create a chunk for one agent knowledge document */
  async create(knowledgeDocumentId: string, body: CreateKnowledgeChunkRequest): Promise<KnowledgeChunkResponse> {
    return this.client.post<KnowledgeChunkResponse>(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/chunks`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge chunk */
  async retrieve(knowledgeChunkId: string): Promise<KnowledgeChunkResponse> {
    return this.client.get<KnowledgeChunkResponse>(agentApiPath(`/ai/knowledge_chunks/${serializePathParameter(knowledgeChunkId, { name: 'knowledgeChunkId', style: 'simple', explode: false })}`));
  }
}

export class AiKnowledgeReadApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Read one provider-neutral knowledge document */
  async read(knowledgeDocumentId: string): Promise<KnowledgeDocumentResponse> {
    return this.client.get<KnowledgeDocumentResponse>(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}`));
  }
}

export class AiKnowledgeSearchApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Search an agent knowledge base for provider-neutral RAG candidates */
  async search(knowledgeBaseId: string, body: SearchKnowledgeRequest): Promise<KnowledgeSearchResponse> {
    return this.client.post<KnowledgeSearchResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/search`), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeDocumentsDeleteParams {
  expectedVersion?: Int64String;
  requestedAt: string;
}

export class AiKnowledgeDocumentsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a document for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeDocumentRequest): Promise<KnowledgeDocumentResponse> {
    return this.client.post<KnowledgeDocumentResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/documents`), body, undefined, undefined, 'application/json');
  }

/** Update one agent knowledge document */
  async update(knowledgeDocumentId: string, body: UpdateKnowledgeDocumentRequest): Promise<KnowledgeDocumentResponse> {
    return this.client.patch<KnowledgeDocumentResponse>(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}`), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one agent knowledge document */
  async delete(knowledgeDocumentId: string, params: AiKnowledgeDocumentsDeleteParams): Promise<KnowledgeDocumentResponse> {
    const query = buildQueryString([
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<KnowledgeDocumentResponse>(appendQueryString(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent knowledge document */
  async restore(knowledgeDocumentId: string, body: RestoreAgentRequest): Promise<KnowledgeDocumentResponse> {
    return this.client.post<KnowledgeDocumentResponse>(agentApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/restore`), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeListListParams {
  page?: number;
  pageSize?: number;
}

export class AiKnowledgeListApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List provider-neutral knowledge documents for one agent knowledge base */
  async list(knowledgeBaseId: string, params?: AiKnowledgeListListParams): Promise<KnowledgeDocumentListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeDocumentListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/documents`), query));
  }
}

export interface AiKnowledgeSourcesListParams {
  page?: number;
  pageSize?: number;
}

export interface AiKnowledgeSourcesDeleteParams {
  expectedVersion?: Int64String;
  requestedAt: string;
}

export class AiKnowledgeSourcesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List sources for one agent knowledge base */
  async list(knowledgeBaseId: string, params?: AiKnowledgeSourcesListParams): Promise<KnowledgeSourceListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeSourceListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sources`), query));
  }

/** Create a source for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeSourceRequest): Promise<KnowledgeSourceResponse> {
    return this.client.post<KnowledgeSourceResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sources`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge source */
  async retrieve(knowledgeSourceId: string): Promise<KnowledgeSourceResponse> {
    return this.client.get<KnowledgeSourceResponse>(agentApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}`));
  }

/** Update one agent knowledge source */
  async update(knowledgeSourceId: string, body: UpdateKnowledgeSourceRequest): Promise<KnowledgeSourceResponse> {
    return this.client.patch<KnowledgeSourceResponse>(agentApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}`), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one agent knowledge source */
  async delete(knowledgeSourceId: string, params: AiKnowledgeSourcesDeleteParams): Promise<KnowledgeSourceResponse> {
    const query = buildQueryString([
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<KnowledgeSourceResponse>(appendQueryString(agentApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent knowledge source */
  async restore(knowledgeSourceId: string, body: RestoreAgentRequest): Promise<KnowledgeSourceResponse> {
    return this.client.post<KnowledgeSourceResponse>(agentApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}/restore`), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeBasesListParams {
  includeDeleted?: boolean;
  page?: number;
  pageSize?: number;
  q?: string;
}

export interface AiKnowledgeBasesDeleteParams {
  expectedVersion?: Int64String;
  requestedAt: string;
}

export class AiKnowledgeBasesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List agent knowledge bases */
  async list(params?: AiKnowledgeBasesListParams): Promise<KnowledgeBaseListResponse> {
    const query = buildQueryString([
      { name: 'include_deleted', value: params?.includeDeleted, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeBaseListResponse>(appendQueryString(agentApiPath(`/ai/knowledge_bases`), query));
  }

/** Create an agent knowledge base */
  async create(body: CreateKnowledgeBaseRequest): Promise<KnowledgeBaseResponse> {
    return this.client.post<KnowledgeBaseResponse>(agentApiPath(`/ai/knowledge_bases`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge base */
  async retrieve(knowledgeBaseId: string): Promise<KnowledgeBaseResponse> {
    return this.client.get<KnowledgeBaseResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}`));
  }

/** Update one agent knowledge base */
  async update(knowledgeBaseId: string, body: UpdateKnowledgeBaseRequest): Promise<KnowledgeBaseResponse> {
    return this.client.patch<KnowledgeBaseResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}`), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one agent knowledge base */
  async delete(knowledgeBaseId: string, params: AiKnowledgeBasesDeleteParams): Promise<KnowledgeBaseResponse> {
    const query = buildQueryString([
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<KnowledgeBaseResponse>(appendQueryString(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent knowledge base */
  async restore(knowledgeBaseId: string, body: RestoreAgentRequest): Promise<KnowledgeBaseResponse> {
    return this.client.post<KnowledgeBaseResponse>(agentApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/restore`), body, undefined, undefined, 'application/json');
  }
}

export class AiAgentsPromptOptimizationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a prompt optimization for one managed agent */
  async create(agentId: string, body: CreateAgentPromptOptimizationRequest): Promise<AgentRuntimeExecutionResponse> {
    return this.client.post<AgentRuntimeExecutionResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/prompt_optimizations`), body, undefined, undefined, 'application/json');
  }
}

export class AiAgentsPreviewResponsesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a preview response for one managed agent */
  async create(agentId: string, body: CreateAgentPreviewResponseRequest): Promise<AgentRuntimeExecutionResponse> {
    return this.client.post<AgentRuntimeExecutionResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/preview_responses`), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsDeploymentsListParams {
  page?: number;
  pageSize?: number;
}

export class AiAgentsDeploymentsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List deployments for one managed agent */
  async list(agentId: string, params?: AiAgentsDeploymentsListParams): Promise<AgentDeploymentListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentDeploymentListResponse>(appendQueryString(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/deployments`), query));
  }

/** Create a deployment snapshot for one managed agent provider binding */
  async create(agentId: string, body: CreateAgentDeploymentRequest): Promise<AgentDeploymentResponse> {
    return this.client.post<AgentDeploymentResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/deployments`), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsProviderBindingsListParams {
  page?: number;
  pageSize?: number;
}

export class AiAgentsProviderBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List provider bindings for one managed agent */
  async list(agentId: string, params?: AiAgentsProviderBindingsListParams): Promise<AgentProviderBindingListResponse> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentProviderBindingListResponse>(appendQueryString(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/provider_bindings`), query));
  }

/** Create a provider binding for one managed agent */
  async create(agentId: string, body: CreateAgentProviderBindingRequest): Promise<AgentProviderBindingResponse> {
    return this.client.post<AgentProviderBindingResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/provider_bindings`), body, undefined, undefined, 'application/json');
  }

/** Activate one managed agent provider binding */
  async activate(agentId: string, bindingId: string, body: ActivateAgentProviderBindingRequest): Promise<AgentProviderBindingResponse> {
    return this.client.post<AgentProviderBindingResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}/activate`), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsListParams {
  includeDeleted?: boolean;
  page?: number;
  pageSize?: number;
  q?: string;
}

export class AiAgentsApi {
  private client: HttpClient;
  public readonly providerBindings: AiAgentsProviderBindingsApi;
  public readonly deployments: AiAgentsDeploymentsApi;
  public readonly previewResponses: AiAgentsPreviewResponsesApi;
  public readonly promptOptimizations: AiAgentsPromptOptimizationsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.providerBindings = new AiAgentsProviderBindingsApi(client);
    this.deployments = new AiAgentsDeploymentsApi(client);
    this.previewResponses = new AiAgentsPreviewResponsesApi(client);
    this.promptOptimizations = new AiAgentsPromptOptimizationsApi(client);
  }


/** List managed agents */
  async list(params?: AiAgentsListParams): Promise<AgentListResponse> {
    const query = buildQueryString([
      { name: 'include_deleted', value: params?.includeDeleted, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentListResponse>(appendQueryString(agentApiPath(`/ai/agents`), query));
  }

/** Create a managed agent */
  async create(body: CreateAgentRequest): Promise<AgentResponse> {
    return this.client.post<AgentResponse>(agentApiPath(`/ai/agents`), body, undefined, undefined, 'application/json');
  }

/** Retrieve one managed agent */
  async retrieve(agentId: string): Promise<AgentResponse> {
    return this.client.get<AgentResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}`));
  }

/** Update one managed agent */
  async update(agentId: string, body: UpdateAgentRequest): Promise<AgentResponse> {
    return this.client.patch<AgentResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}`), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one managed agent */
  async delete(agentId: string): Promise<AgentResponse> {
    return this.client.delete<AgentResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}`));
  }

/** Restore one soft-deleted managed agent */
  async restore(agentId: string, body: RestoreAgentRequest): Promise<AgentResponse> {
    return this.client.post<AgentResponse>(agentApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/restore`), body, undefined, undefined, 'application/json');
  }
}

export class AiApi {
  private client: HttpClient;
  public readonly agents: AiAgentsApi;
  public readonly knowledgeBases: AiKnowledgeBasesApi;
  public readonly knowledgeSources: AiKnowledgeSourcesApi;
  public readonly knowledgeList: AiKnowledgeListApi;
  public readonly knowledgeDocuments: AiKnowledgeDocumentsApi;
  public readonly knowledgeSearch: AiKnowledgeSearchApi;
  public readonly knowledgeRead: AiKnowledgeReadApi;
  public readonly knowledgeChunks: AiKnowledgeChunksApi;
  public readonly knowledgeIndexes: AiKnowledgeIndexesApi;
  public readonly knowledgeBindings: AiKnowledgeBindingsApi;
  public readonly knowledgeSyncJobs: AiKnowledgeSyncJobsApi;
  public readonly memoryStores: AiMemoryStoresApi;
  public readonly memoryProfiles: AiMemoryProfilesApi;
  public readonly memoryBindings: AiMemoryBindingsApi;
  public readonly memoryNamespaces: AiMemoryNamespacesApi;
  public readonly memoryRecords: AiMemoryRecordsApi;
  public readonly memorySources: AiMemorySourcesApi;
  public readonly memoryRelations: AiMemoryRelationsApi;
  public readonly memoryRetrievalIndexes: AiMemoryRetrievalIndexesApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.agents = new AiAgentsApi(client);
    this.knowledgeBases = new AiKnowledgeBasesApi(client);
    this.knowledgeSources = new AiKnowledgeSourcesApi(client);
    this.knowledgeList = new AiKnowledgeListApi(client);
    this.knowledgeDocuments = new AiKnowledgeDocumentsApi(client);
    this.knowledgeSearch = new AiKnowledgeSearchApi(client);
    this.knowledgeRead = new AiKnowledgeReadApi(client);
    this.knowledgeChunks = new AiKnowledgeChunksApi(client);
    this.knowledgeIndexes = new AiKnowledgeIndexesApi(client);
    this.knowledgeBindings = new AiKnowledgeBindingsApi(client);
    this.knowledgeSyncJobs = new AiKnowledgeSyncJobsApi(client);
    this.memoryStores = new AiMemoryStoresApi(client);
    this.memoryProfiles = new AiMemoryProfilesApi(client);
    this.memoryBindings = new AiMemoryBindingsApi(client);
    this.memoryNamespaces = new AiMemoryNamespacesApi(client);
    this.memoryRecords = new AiMemoryRecordsApi(client);
    this.memorySources = new AiMemorySourcesApi(client);
    this.memoryRelations = new AiMemoryRelationsApi(client);
    this.memoryRetrievalIndexes = new AiMemoryRetrievalIndexesApi(client);
  }

}

export function createAiApi(client: HttpClient): AiApi {
  return new AiApi(client);
}

function appendQueryString(path: string, rawQueryString: string): string {
  const query = rawQueryString.replace(/^\?+/, '');
  if (!query) {
    return path;
  }
  return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
}

interface PathParameterSpec {
  name: string;
  style: string;
  explode: boolean;
}

function serializePathParameter(value: unknown, spec: PathParameterSpec): string {
  if (value === undefined || value === null) {
    return '';
  }

  const style = spec.style || 'simple';
  if (Array.isArray(value)) {
    return serializePathArray(spec.name, value, style, spec.explode);
  }
  if (typeof value === 'object') {
    return serializePathObject(spec.name, value as Record<string, unknown>, style, spec.explode);
  }
  return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}

function serializePathArray(name: string, values: unknown[], style: string, explode: boolean): string {
  const serialized = values
    .filter((item) => item !== undefined && item !== null)
    .map((item) => encodePathValue(serializePathPrimitive(item)));
  if (serialized.length === 0) {
    return pathPrefix(name, style, false);
  }
  if (style === 'matrix') {
    return explode
      ? serialized.map((item) => `;${name}=${item}`).join('')
      : `;${name}=${serialized.join(',')}`;
  }
  return pathPrefix(name, style, false) + serialized.join(explode ? '.' : ',');
}

function serializePathObject(name: string, value: Record<string, unknown>, style: string, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return pathPrefix(name, style, true);
  }
  if (style === 'matrix') {
    return explode
      ? entries.map(([key, entryValue]) => `;${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join('')
      : `;${name}=${entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',')}`;
  }
  const serialized = explode
    ? entries.map(([key, entryValue]) => `${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join(style === 'label' ? '.' : ',')
    : entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',');
  return pathPrefix(name, style, true) + serialized;
}

function pathPrefix(name: string, style: string, _objectValue: boolean): string {
  if (style === 'label') return '.';
  if (style === 'matrix') return `;${name}`;
  return '';
}

function encodePathValue(value: string): string {
  return encodeURIComponent(value);
}

function serializePathPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}
interface QueryParameterSpec {
  name: string;
  value: unknown;
  style: string;
  explode: boolean;
  allowReserved: boolean;
  contentType?: string;
}

function buildQueryString(parameters: QueryParameterSpec[]): string {
  const pairs: string[] = [];
  for (const parameter of parameters) {
    appendSerializedParameter(pairs, parameter);
  }
  return pairs.join('&');
}

function appendSerializedParameter(pairs: string[], parameter: QueryParameterSpec): void {
  if (parameter.value === undefined || parameter.value === null) {
    return;
  }

  if (parameter.contentType) {
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(JSON.stringify(parameter.value), parameter.allowReserved)}`);
    return;
  }

  const style = parameter.style || 'form';
  if (style === 'deepObject') {
    appendDeepObjectParameter(pairs, parameter.name, parameter.value, parameter.allowReserved);
    return;
  }

  if (Array.isArray(parameter.value)) {
    appendArrayParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
    return;
  }

  if (typeof parameter.value === 'object') {
    appendObjectParameter(pairs, parameter.name, parameter.value as Record<string, unknown>, style, parameter.explode, parameter.allowReserved);
    return;
  }

  pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}

function appendArrayParameter(
  pairs: string[],
  name: string,
  value: unknown[],
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const values = value
    .filter((item) => item !== undefined && item !== null)
    .map((item) => serializePrimitive(item));
  if (values.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const item of values) {
      pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(item, allowReserved)}`);
    }
    return;
  }

  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(values.join(','), allowReserved)}`);
}

function appendObjectParameter(
  pairs: string[],
  name: string,
  value: Record<string, unknown>,
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const [key, entryValue] of entries) {
      pairs.push(`${encodeQueryComponent(key)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
    return;
  }

  const serialized = entries.flatMap(([key, entryValue]) => [key, serializePrimitive(entryValue)]).join(',');
  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serialized, allowReserved)}`);
}

function appendDeepObjectParameter(
  pairs: string[],
  name: string,
  value: unknown,
  allowReserved: boolean,
): void {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
    return;
  }

  for (const [key, entryValue] of Object.entries(value as Record<string, unknown>)) {
    if (entryValue === undefined || entryValue === null) {
      continue;
    }
    pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
  }
}

function serializePrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

function encodeQueryComponent(value: string): string {
  return encodeURIComponent(value);
}

function encodeQueryValue(value: string, allowReserved: boolean): string {
  const encoded = encodeURIComponent(value);
  if (!allowReserved) {
    return encoded;
  }
  return encoded.replace(/%3A/gi, ':')
    .replace(/%2F/gi, '/')
    .replace(/%3F/gi, '?')
    .replace(/%23/gi, '#')
    .replace(/%5B/gi, '[')
    .replace(/%5D/gi, ']')
    .replace(/%40/gi, '@')
    .replace(/%21/gi, '!')
    .replace(/%24/gi, '$')
    .replace(/%26/gi, '&')
    .replace(/%27/gi, "'")
    .replace(/%28/gi, '(')
    .replace(/%29/gi, ')')
    .replace(/%2A/gi, '*')
    .replace(/%2B/gi, '+')
    .replace(/%2C/gi, ',')
    .replace(/%3B/gi, ';')
    .replace(/%3D/gi, '=');
}
