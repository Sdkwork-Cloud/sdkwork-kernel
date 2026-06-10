import { appApiPath } from './paths';
import type { HttpClient } from '../http/client';

import type { ActivateAgentProviderBindingRequest, AgentDeploymentListResponse, AgentDeploymentResponse, AgentListResponse, AgentProviderBindingListResponse, AgentProviderBindingResponse, AgentResponse, AgentRuntimeExecutionResponse, CancelKnowledgeSyncJobRequest, CompleteKnowledgeSyncJobRequest, CreateAgentDeploymentRequest, CreateAgentPreviewResponseRequest, CreateAgentPromptOptimizationRequest, CreateAgentProviderBindingRequest, CreateAgentRequest, CreateKnowledgeBaseRequest, CreateKnowledgeBindingRequest, CreateKnowledgeChunkRequest, CreateKnowledgeDocumentRequest, CreateKnowledgeSourceRequest, CreateKnowledgeSyncJobRequest, CreateMemoryBindingRequest, CreateMemoryNamespaceRequest, CreateMemoryProfileRequest, CreateMemoryRecordRequest, CreateMemoryRelationRequest, CreateMemorySourceRequest, CreateMemoryStoreRequest, FailKnowledgeSyncJobRequest, Int64String, KnowledgeBaseListResponse, KnowledgeBaseResponse, KnowledgeBindingListResponse, KnowledgeBindingResponse, KnowledgeChunkListResponse, KnowledgeChunkResponse, KnowledgeDocumentListResponse, KnowledgeDocumentResponse, KnowledgeIndexListResponse, KnowledgeIndexResponse, KnowledgeSearchResponse, KnowledgeSourceListResponse, KnowledgeSourceResponse, KnowledgeSyncJobListResponse, KnowledgeSyncJobResponse, MemoryBindingResponse, MemoryNamespaceResponse, MemoryProfileResponse, MemoryRecordListResponse, MemoryRecordResponse, MemoryRelationListResponse, MemoryRelationResponse, MemoryRetrievalIndexListResponse, MemoryRetrievalIndexResponse, MemorySourceListResponse, MemorySourceResponse, MemoryStoreResponse, RestoreAgentRequest, SearchKnowledgeRequest, StartKnowledgeSyncJobRequest, UpdateAgentRequest, UpdateKnowledgeBaseRequest, UpdateKnowledgeDocumentRequest, UpdateKnowledgeSourceRequest, UpdateMemoryStoreRequest, UpsertKnowledgeIndexRequest, UpsertMemoryRetrievalIndexRequest } from '../types';


export interface AiMemoryRetrievalIndexesListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiMemoryRetrievalIndexesUpsertParams {
  tenantId: Int64String;
}

export class AiMemoryRetrievalIndexesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List retrieval indexes for one agent memory record */
  async list(memoryId: string, params: AiMemoryRetrievalIndexesListParams): Promise<MemoryRetrievalIndexListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRetrievalIndexListResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/retrieval_indexes`), query));
  }

/** Upsert an agent memory retrieval index */
  async upsert(body: UpsertMemoryRetrievalIndexRequest, params: AiMemoryRetrievalIndexesUpsertParams): Promise<MemoryRetrievalIndexResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryRetrievalIndexResponse>(appendQueryString(appApiPath(`/ai/memory_retrieval_indexes`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemoryRelationsListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiMemoryRelationsCreateParams {
  tenantId: Int64String;
}

export class AiMemoryRelationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List graph relations for one agent memory record */
  async list(memoryId: string, params: AiMemoryRelationsListParams): Promise<MemoryRelationListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRelationListResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/relations`), query));
  }

/** Create a graph relation for one agent memory record */
  async create(memoryId: string, body: CreateMemoryRelationRequest, params: AiMemoryRelationsCreateParams): Promise<MemoryRelationResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryRelationResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/relations`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemorySourcesListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiMemorySourcesCreateParams {
  tenantId: Int64String;
}

export class AiMemorySourcesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List provenance sources for one agent memory record */
  async list(memoryId: string, params: AiMemorySourcesListParams): Promise<MemorySourceListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemorySourceListResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/sources`), query));
  }

/** Create a provenance source for one agent memory record */
  async create(memoryId: string, body: CreateMemorySourceRequest, params: AiMemorySourcesCreateParams): Promise<MemorySourceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemorySourceResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/sources`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemoryRecordsListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiMemoryRecordsCreateParams {
  tenantId: Int64String;
}

export interface AiMemoryRecordsRetrieveParams {
  tenantId: Int64String;
}

export interface AiMemoryRecordsDeleteParams {
  tenantId: Int64String;
  expectedVersion?: Int64String;
  requestedAt: string;
}

export interface AiMemoryRecordsRestoreParams {
  tenantId: Int64String;
}

export class AiMemoryRecordsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List agent memory records in one namespace */
  async list(memoryNamespaceId: string, params: AiMemoryRecordsListParams): Promise<MemoryRecordListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRecordListResponse>(appendQueryString(appApiPath(`/ai/memory_namespaces/${serializePathParameter(memoryNamespaceId, { name: 'memoryNamespaceId', style: 'simple', explode: false })}/records`), query));
  }

/** Create an agent memory record in one namespace */
  async create(memoryNamespaceId: string, body: CreateMemoryRecordRequest, params: AiMemoryRecordsCreateParams): Promise<MemoryRecordResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryRecordResponse>(appendQueryString(appApiPath(`/ai/memory_namespaces/${serializePathParameter(memoryNamespaceId, { name: 'memoryNamespaceId', style: 'simple', explode: false })}/records`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory record */
  async retrieve(memoryId: string, params: AiMemoryRecordsRetrieveParams): Promise<MemoryRecordResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryRecordResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}`), query));
  }

/** Soft-delete one agent memory record */
  async delete(memoryId: string, params: AiMemoryRecordsDeleteParams): Promise<MemoryRecordResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<MemoryRecordResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent memory record */
  async restore(memoryId: string, body: RestoreAgentRequest, params: AiMemoryRecordsRestoreParams): Promise<MemoryRecordResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryRecordResponse>(appendQueryString(appApiPath(`/ai/memory_records/${serializePathParameter(memoryId, { name: 'memoryId', style: 'simple', explode: false })}/restore`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiMemoryNamespacesCreateParams {
  tenantId: Int64String;
}

export interface AiMemoryNamespacesRetrieveParams {
  tenantId: Int64String;
}

export class AiMemoryNamespacesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory namespace */
  async create(body: CreateMemoryNamespaceRequest, params: AiMemoryNamespacesCreateParams): Promise<MemoryNamespaceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryNamespaceResponse>(appendQueryString(appApiPath(`/ai/memory_namespaces`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory namespace */
  async retrieve(memoryNamespaceId: string, params: AiMemoryNamespacesRetrieveParams): Promise<MemoryNamespaceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryNamespaceResponse>(appendQueryString(appApiPath(`/ai/memory_namespaces/${serializePathParameter(memoryNamespaceId, { name: 'memoryNamespaceId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiMemoryBindingsCreateParams {
  tenantId: Int64String;
}

export interface AiMemoryBindingsRetrieveParams {
  tenantId: Int64String;
}

export class AiMemoryBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory profile binding */
  async create(memoryProfileId: string, body: CreateMemoryBindingRequest, params: AiMemoryBindingsCreateParams): Promise<MemoryBindingResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryBindingResponse>(appendQueryString(appApiPath(`/ai/memory_profiles/${serializePathParameter(memoryProfileId, { name: 'memoryProfileId', style: 'simple', explode: false })}/bindings`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory binding */
  async retrieve(memoryBindingId: string, params: AiMemoryBindingsRetrieveParams): Promise<MemoryBindingResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryBindingResponse>(appendQueryString(appApiPath(`/ai/memory_bindings/${serializePathParameter(memoryBindingId, { name: 'memoryBindingId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiMemoryProfilesCreateParams {
  tenantId: Int64String;
}

export interface AiMemoryProfilesRetrieveParams {
  tenantId: Int64String;
}

export class AiMemoryProfilesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory profile for one store */
  async create(memoryStoreId: string, body: CreateMemoryProfileRequest, params: AiMemoryProfilesCreateParams): Promise<MemoryProfileResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryProfileResponse>(appendQueryString(appApiPath(`/ai/memory_stores/${serializePathParameter(memoryStoreId, { name: 'memoryStoreId', style: 'simple', explode: false })}/profiles`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory profile */
  async retrieve(memoryProfileId: string, params: AiMemoryProfilesRetrieveParams): Promise<MemoryProfileResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryProfileResponse>(appendQueryString(appApiPath(`/ai/memory_profiles/${serializePathParameter(memoryProfileId, { name: 'memoryProfileId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiMemoryStoresCreateParams {
  tenantId: Int64String;
}

export interface AiMemoryStoresRetrieveParams {
  tenantId: Int64String;
}

export interface AiMemoryStoresUpdateParams {
  tenantId: Int64String;
}

export class AiMemoryStoresApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create an agent memory store */
  async create(body: CreateMemoryStoreRequest, params: AiMemoryStoresCreateParams): Promise<MemoryStoreResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<MemoryStoreResponse>(appendQueryString(appApiPath(`/ai/memory_stores`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent memory store */
  async retrieve(memoryStoreId: string, params: AiMemoryStoresRetrieveParams): Promise<MemoryStoreResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MemoryStoreResponse>(appendQueryString(appApiPath(`/ai/memory_stores/${serializePathParameter(memoryStoreId, { name: 'memoryStoreId', style: 'simple', explode: false })}`), query));
  }

/** Update one agent memory store */
  async update(memoryStoreId: string, body: UpdateMemoryStoreRequest, params: AiMemoryStoresUpdateParams): Promise<MemoryStoreResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.patch<MemoryStoreResponse>(appendQueryString(appApiPath(`/ai/memory_stores/${serializePathParameter(memoryStoreId, { name: 'memoryStoreId', style: 'simple', explode: false })}`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeSyncJobsListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiKnowledgeSyncJobsCreateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSyncJobsRetrieveParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSyncJobsStartParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSyncJobsCompleteParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSyncJobsFailParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSyncJobsCancelParams {
  tenantId: Int64String;
}

export class AiKnowledgeSyncJobsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List sync jobs for one agent knowledge base */
  async list(knowledgeBaseId: string, params: AiKnowledgeSyncJobsListParams): Promise<KnowledgeSyncJobListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeSyncJobListResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sync_jobs`), query));
  }

/** Create a sync job for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsCreateParams): Promise<KnowledgeSyncJobResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSyncJobResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sync_jobs`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge sync job */
  async retrieve(syncJobId: string, params: AiKnowledgeSyncJobsRetrieveParams): Promise<KnowledgeSyncJobResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeSyncJobResponse>(appendQueryString(appApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}`), query));
  }

/** Start one agent knowledge sync job */
  async start(syncJobId: string, body: StartKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsStartParams): Promise<KnowledgeSyncJobResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSyncJobResponse>(appendQueryString(appApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/start`), query), body, undefined, undefined, 'application/json');
  }

/** Complete one agent knowledge sync job */
  async complete(syncJobId: string, body: CompleteKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsCompleteParams): Promise<KnowledgeSyncJobResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSyncJobResponse>(appendQueryString(appApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/complete`), query), body, undefined, undefined, 'application/json');
  }

/** Fail one agent knowledge sync job */
  async fail(syncJobId: string, body: FailKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsFailParams): Promise<KnowledgeSyncJobResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSyncJobResponse>(appendQueryString(appApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/fail`), query), body, undefined, undefined, 'application/json');
  }

/** Cancel one agent knowledge sync job */
  async cancel(syncJobId: string, body: CancelKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsCancelParams): Promise<KnowledgeSyncJobResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSyncJobResponse>(appendQueryString(appApiPath(`/ai/knowledge_sync_jobs/${serializePathParameter(syncJobId, { name: 'syncJobId', style: 'simple', explode: false })}/cancel`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeBindingsListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiKnowledgeBindingsCreateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeBindingsRetrieveParams {
  tenantId: Int64String;
}

export class AiKnowledgeBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List bindings for one agent knowledge base */
  async list(knowledgeBaseId: string, params: AiKnowledgeBindingsListParams): Promise<KnowledgeBindingListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeBindingListResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/bindings`), query));
  }

/** Create a binding for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeBindingRequest, params: AiKnowledgeBindingsCreateParams): Promise<KnowledgeBindingResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeBindingResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/bindings`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge binding */
  async retrieve(knowledgeBindingId: string, params: AiKnowledgeBindingsRetrieveParams): Promise<KnowledgeBindingResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeBindingResponse>(appendQueryString(appApiPath(`/ai/knowledge_bindings/${serializePathParameter(knowledgeBindingId, { name: 'knowledgeBindingId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiKnowledgeIndexesListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiKnowledgeIndexesUpsertParams {
  tenantId: Int64String;
}

export interface AiKnowledgeIndexesRetrieveParams {
  tenantId: Int64String;
}

export class AiKnowledgeIndexesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List indexes for one agent knowledge document */
  async list(knowledgeDocumentId: string, params: AiKnowledgeIndexesListParams): Promise<KnowledgeIndexListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeIndexListResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/indexes`), query));
  }

/** Upsert an agent knowledge retrieval index */
  async upsert(body: UpsertKnowledgeIndexRequest, params: AiKnowledgeIndexesUpsertParams): Promise<KnowledgeIndexResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeIndexResponse>(appendQueryString(appApiPath(`/ai/knowledge_indexes`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge retrieval index */
  async retrieve(knowledgeIndexId: string, params: AiKnowledgeIndexesRetrieveParams): Promise<KnowledgeIndexResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeIndexResponse>(appendQueryString(appApiPath(`/ai/knowledge_indexes/${serializePathParameter(knowledgeIndexId, { name: 'knowledgeIndexId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiKnowledgeChunksListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiKnowledgeChunksCreateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeChunksRetrieveParams {
  tenantId: Int64String;
}

export class AiKnowledgeChunksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List chunks for one agent knowledge document */
  async list(knowledgeDocumentId: string, params: AiKnowledgeChunksListParams): Promise<KnowledgeChunkListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeChunkListResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/chunks`), query));
  }

/** Create a chunk for one agent knowledge document */
  async create(knowledgeDocumentId: string, body: CreateKnowledgeChunkRequest, params: AiKnowledgeChunksCreateParams): Promise<KnowledgeChunkResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeChunkResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/chunks`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge chunk */
  async retrieve(knowledgeChunkId: string, params: AiKnowledgeChunksRetrieveParams): Promise<KnowledgeChunkResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeChunkResponse>(appendQueryString(appApiPath(`/ai/knowledge_chunks/${serializePathParameter(knowledgeChunkId, { name: 'knowledgeChunkId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiKnowledgeReadReadParams {
  tenantId: Int64String;
}

export class AiKnowledgeReadApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Read one provider-neutral knowledge document */
  async read(knowledgeDocumentId: string, params: AiKnowledgeReadReadParams): Promise<KnowledgeDocumentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeDocumentResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}`), query));
  }
}

export interface AiKnowledgeSearchSearchParams {
  tenantId: Int64String;
}

export class AiKnowledgeSearchApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Search an agent knowledge base for provider-neutral RAG candidates */
  async search(knowledgeBaseId: string, body: SearchKnowledgeRequest, params: AiKnowledgeSearchSearchParams): Promise<KnowledgeSearchResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSearchResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/search`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeDocumentsCreateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeDocumentsUpdateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeDocumentsDeleteParams {
  tenantId: Int64String;
  expectedVersion?: Int64String;
  requestedAt: string;
}

export interface AiKnowledgeDocumentsRestoreParams {
  tenantId: Int64String;
}

export class AiKnowledgeDocumentsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a document for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeDocumentRequest, params: AiKnowledgeDocumentsCreateParams): Promise<KnowledgeDocumentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeDocumentResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/documents`), query), body, undefined, undefined, 'application/json');
  }

/** Update one agent knowledge document */
  async update(knowledgeDocumentId: string, body: UpdateKnowledgeDocumentRequest, params: AiKnowledgeDocumentsUpdateParams): Promise<KnowledgeDocumentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.patch<KnowledgeDocumentResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}`), query), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one agent knowledge document */
  async delete(knowledgeDocumentId: string, params: AiKnowledgeDocumentsDeleteParams): Promise<KnowledgeDocumentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<KnowledgeDocumentResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent knowledge document */
  async restore(knowledgeDocumentId: string, body: RestoreAgentRequest, params: AiKnowledgeDocumentsRestoreParams): Promise<KnowledgeDocumentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeDocumentResponse>(appendQueryString(appApiPath(`/ai/knowledge_documents/${serializePathParameter(knowledgeDocumentId, { name: 'knowledgeDocumentId', style: 'simple', explode: false })}/restore`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeListListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export class AiKnowledgeListApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List provider-neutral knowledge documents for one agent knowledge base */
  async list(knowledgeBaseId: string, params: AiKnowledgeListListParams): Promise<KnowledgeDocumentListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeDocumentListResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/documents`), query));
  }
}

export interface AiKnowledgeSourcesListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiKnowledgeSourcesCreateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSourcesRetrieveParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSourcesUpdateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeSourcesDeleteParams {
  tenantId: Int64String;
  expectedVersion?: Int64String;
  requestedAt: string;
}

export interface AiKnowledgeSourcesRestoreParams {
  tenantId: Int64String;
}

export class AiKnowledgeSourcesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List sources for one agent knowledge base */
  async list(knowledgeBaseId: string, params: AiKnowledgeSourcesListParams): Promise<KnowledgeSourceListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeSourceListResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sources`), query));
  }

/** Create a source for one agent knowledge base */
  async create(knowledgeBaseId: string, body: CreateKnowledgeSourceRequest, params: AiKnowledgeSourcesCreateParams): Promise<KnowledgeSourceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSourceResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/sources`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge source */
  async retrieve(knowledgeSourceId: string, params: AiKnowledgeSourcesRetrieveParams): Promise<KnowledgeSourceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeSourceResponse>(appendQueryString(appApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}`), query));
  }

/** Update one agent knowledge source */
  async update(knowledgeSourceId: string, body: UpdateKnowledgeSourceRequest, params: AiKnowledgeSourcesUpdateParams): Promise<KnowledgeSourceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.patch<KnowledgeSourceResponse>(appendQueryString(appApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}`), query), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one agent knowledge source */
  async delete(knowledgeSourceId: string, params: AiKnowledgeSourcesDeleteParams): Promise<KnowledgeSourceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<KnowledgeSourceResponse>(appendQueryString(appApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent knowledge source */
  async restore(knowledgeSourceId: string, body: RestoreAgentRequest, params: AiKnowledgeSourcesRestoreParams): Promise<KnowledgeSourceResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeSourceResponse>(appendQueryString(appApiPath(`/ai/knowledge_sources/${serializePathParameter(knowledgeSourceId, { name: 'knowledgeSourceId', style: 'simple', explode: false })}/restore`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiKnowledgeBasesListParams {
  tenantId: Int64String;
  organizationId?: Int64String;
  ownerUserId?: Int64String;
  includeDeleted?: boolean;
  page?: number;
  pageSize?: number;
  q?: string;
}

export interface AiKnowledgeBasesCreateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeBasesRetrieveParams {
  tenantId: Int64String;
}

export interface AiKnowledgeBasesUpdateParams {
  tenantId: Int64String;
}

export interface AiKnowledgeBasesDeleteParams {
  tenantId: Int64String;
  expectedVersion?: Int64String;
  requestedAt: string;
}

export interface AiKnowledgeBasesRestoreParams {
  tenantId: Int64String;
}

export class AiKnowledgeBasesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List agent knowledge bases */
  async list(params: AiKnowledgeBasesListParams): Promise<KnowledgeBaseListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'organization_id', value: params.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'owner_user_id', value: params.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'include_deleted', value: params.includeDeleted, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params.q, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeBaseListResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases`), query));
  }

/** Create an agent knowledge base */
  async create(body: CreateKnowledgeBaseRequest, params: AiKnowledgeBasesCreateParams): Promise<KnowledgeBaseResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeBaseResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one agent knowledge base */
  async retrieve(knowledgeBaseId: string, params: AiKnowledgeBasesRetrieveParams): Promise<KnowledgeBaseResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<KnowledgeBaseResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}`), query));
  }

/** Update one agent knowledge base */
  async update(knowledgeBaseId: string, body: UpdateKnowledgeBaseRequest, params: AiKnowledgeBasesUpdateParams): Promise<KnowledgeBaseResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.patch<KnowledgeBaseResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}`), query), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one agent knowledge base */
  async delete(knowledgeBaseId: string, params: AiKnowledgeBasesDeleteParams): Promise<KnowledgeBaseResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'expected_version', value: params.expectedVersion, style: 'form', explode: true, allowReserved: false },
      { name: 'requested_at', value: params.requestedAt, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<KnowledgeBaseResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted agent knowledge base */
  async restore(knowledgeBaseId: string, body: RestoreAgentRequest, params: AiKnowledgeBasesRestoreParams): Promise<KnowledgeBaseResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<KnowledgeBaseResponse>(appendQueryString(appApiPath(`/ai/knowledge_bases/${serializePathParameter(knowledgeBaseId, { name: 'knowledgeBaseId', style: 'simple', explode: false })}/restore`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsPromptOptimizationsCreateParams {
  tenantId: Int64String;
}

export class AiAgentsPromptOptimizationsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a prompt optimization for one managed agent */
  async create(agentId: string, body: CreateAgentPromptOptimizationRequest, params: AiAgentsPromptOptimizationsCreateParams): Promise<AgentRuntimeExecutionResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentRuntimeExecutionResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/prompt_optimizations`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsPreviewResponsesCreateParams {
  tenantId: Int64String;
}

export class AiAgentsPreviewResponsesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a preview response for one managed agent */
  async create(agentId: string, body: CreateAgentPreviewResponseRequest, params: AiAgentsPreviewResponsesCreateParams): Promise<AgentRuntimeExecutionResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentRuntimeExecutionResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/preview_responses`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsDeploymentsListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiAgentsDeploymentsCreateParams {
  tenantId: Int64String;
}

export class AiAgentsDeploymentsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List deployments for one managed agent */
  async list(agentId: string, params: AiAgentsDeploymentsListParams): Promise<AgentDeploymentListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentDeploymentListResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/deployments`), query));
  }

/** Create a deployment snapshot for one managed agent provider binding */
  async create(agentId: string, body: CreateAgentDeploymentRequest, params: AiAgentsDeploymentsCreateParams): Promise<AgentDeploymentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentDeploymentResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/deployments`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsProviderBindingsListParams {
  tenantId: Int64String;
  page?: number;
  pageSize?: number;
}

export interface AiAgentsProviderBindingsCreateParams {
  tenantId: Int64String;
}

export interface AiAgentsProviderBindingsActivateParams {
  tenantId: Int64String;
}

export class AiAgentsProviderBindingsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List provider bindings for one managed agent */
  async list(agentId: string, params: AiAgentsProviderBindingsListParams): Promise<AgentProviderBindingListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentProviderBindingListResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/provider_bindings`), query));
  }

/** Create a provider binding for one managed agent */
  async create(agentId: string, body: CreateAgentProviderBindingRequest, params: AiAgentsProviderBindingsCreateParams): Promise<AgentProviderBindingResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentProviderBindingResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/provider_bindings`), query), body, undefined, undefined, 'application/json');
  }

/** Activate one managed agent provider binding */
  async activate(agentId: string, bindingId: string, body: ActivateAgentProviderBindingRequest, params: AiAgentsProviderBindingsActivateParams): Promise<AgentProviderBindingResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentProviderBindingResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/provider_bindings/${serializePathParameter(bindingId, { name: 'bindingId', style: 'simple', explode: false })}/activate`), query), body, undefined, undefined, 'application/json');
  }
}

export interface AiAgentsListParams {
  tenantId: Int64String;
  organizationId?: Int64String;
  ownerUserId?: Int64String;
  includeDeleted?: boolean;
  page?: number;
  pageSize?: number;
  q?: string;
}

export interface AiAgentsCreateParams {
  tenantId: Int64String;
}

export interface AiAgentsRetrieveParams {
  tenantId: Int64String;
}

export interface AiAgentsUpdateParams {
  tenantId: Int64String;
}

export interface AiAgentsDeleteParams {
  tenantId: Int64String;
}

export interface AiAgentsRestoreParams {
  tenantId: Int64String;
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
  async list(params: AiAgentsListParams): Promise<AgentListResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
      { name: 'organization_id', value: params.organizationId, style: 'form', explode: true, allowReserved: false },
      { name: 'owner_user_id', value: params.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'include_deleted', value: params.includeDeleted, style: 'form', explode: true, allowReserved: false },
      { name: 'page', value: params.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params.q, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentListResponse>(appendQueryString(appApiPath(`/ai/agents`), query));
  }

/** Create a managed agent */
  async create(body: CreateAgentRequest, params: AiAgentsCreateParams): Promise<AgentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentResponse>(appendQueryString(appApiPath(`/ai/agents`), query), body, undefined, undefined, 'application/json');
  }

/** Retrieve one managed agent */
  async retrieve(agentId: string, params: AiAgentsRetrieveParams): Promise<AgentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<AgentResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}`), query));
  }

/** Update one managed agent */
  async update(agentId: string, body: UpdateAgentRequest, params: AiAgentsUpdateParams): Promise<AgentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.patch<AgentResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}`), query), body, undefined, undefined, 'application/json');
  }

/** Soft-delete one managed agent */
  async delete(agentId: string, params: AiAgentsDeleteParams): Promise<AgentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.delete<AgentResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}`), query));
  }

/** Restore one soft-deleted managed agent */
  async restore(agentId: string, body: RestoreAgentRequest, params: AiAgentsRestoreParams): Promise<AgentResponse> {
    const query = buildQueryString([
      { name: 'tenant_id', value: params.tenantId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.post<AgentResponse>(appendQueryString(appApiPath(`/ai/agents/${serializePathParameter(agentId, { name: 'agentId', style: 'simple', explode: false })}/restore`), query), body, undefined, undefined, 'application/json');
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
