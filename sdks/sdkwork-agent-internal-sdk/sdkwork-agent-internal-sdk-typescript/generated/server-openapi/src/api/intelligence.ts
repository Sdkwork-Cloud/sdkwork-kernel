import { customApiPath } from './paths';
import type { ApiRequestOptions, HttpClient } from '../http/client';

import type { CancelModelRequest, CancelModelResponse, CreateSessionRequest, ExecuteToolRequest, ExecuteToolResponse, InvokeModelRequest, InvokeModelResponse, MessageResponse, MessageTurnResponse, ModelDescriptor, PermissionDecisionRequest, PermissionRequest, RunResponse, RuntimeDiagnostics, RuntimeHealth, RuntimeManifest, RuntimeSnapshot, SdkWorkAsyncData, SdkWorkPageData, SendMessageRequest, SessionResponse, StreamModelRequest, SubmitTaskRequest, TaskResponse, ToolDescriptor } from '../types';


export interface IntelligenceRuntimeModelsListParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeModelsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List available models */
  async list(params?: IntelligenceRuntimeModelsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData & { items?: ModelDescriptor[]; }> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkPageData & { items?: ModelDescriptor[]; }>(customApiPath(`/intelligence/runtime/models`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'page' });
  }
}

export interface IntelligenceRuntimeRunsRetrieveParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeRunsPauseParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeRunsResumeParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeRunsCancelParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeRunsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve a runtime task run */
  async retrieve(runId: string, params?: IntelligenceRuntimeRunsRetrieveParams, requestOptions?: ApiRequestOptions): Promise<RunResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RunResponse>(customApiPath(`/intelligence/runtime/runs/${serializePathParameter(runId, { name: 'runId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }

/** Pause a runtime task run */
  async pause(runId: string, params: IntelligenceRuntimeRunsPauseParams, requestOptions?: ApiRequestOptions): Promise<RunResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RunResponse>(customApiPath(`/intelligence/runtime/runs/${serializePathParameter(runId, { name: 'runId', style: 'simple', explode: false })}/pause`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }

/** Resume a paused runtime task run */
  async resume(runId: string, params: IntelligenceRuntimeRunsResumeParams, requestOptions?: ApiRequestOptions): Promise<RunResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RunResponse>(customApiPath(`/intelligence/runtime/runs/${serializePathParameter(runId, { name: 'runId', style: 'simple', explode: false })}/resume`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }

/** Cancel a runtime task run */
  async cancel(runId: string, params: IntelligenceRuntimeRunsCancelParams, requestOptions?: ApiRequestOptions): Promise<RunResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RunResponse>(customApiPath(`/intelligence/runtime/runs/${serializePathParameter(runId, { name: 'runId', style: 'simple', explode: false })}/cancel`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeTasksRetrieveParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeTasksCancelParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeTasksRetryParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeTasksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve a task */
  async retrieve(taskId: string, params?: IntelligenceRuntimeTasksRetrieveParams, requestOptions?: ApiRequestOptions): Promise<TaskResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<TaskResponse>(customApiPath(`/intelligence/runtime/tasks/${serializePathParameter(taskId, { name: 'taskId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }

/** Cancel a task */
  async cancel(taskId: string, params: IntelligenceRuntimeTasksCancelParams, requestOptions?: ApiRequestOptions): Promise<TaskResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<TaskResponse>(customApiPath(`/intelligence/runtime/tasks/${serializePathParameter(taskId, { name: 'taskId', style: 'simple', explode: false })}/cancel`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }

/** Retry a failed or cancelled task */
  async retry(taskId: string, params: IntelligenceRuntimeTasksRetryParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkAsyncData> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkAsyncData>(customApiPath(`/intelligence/runtime/tasks/${serializePathParameter(taskId, { name: 'taskId', style: 'simple', explode: false })}/retry`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, headers: requestHeaders, sdkworkUnwrapKind: 'command' });
  }
}

export interface IntelligenceRuntimeSessionsEventsStreamParams {
  lastEventId?: string;
  live?: boolean;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSessionsEventsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Stream session events */
  async stream(sessionId: string, params?: IntelligenceRuntimeSessionsEventsStreamParams, requestOptions?: ApiRequestOptions): Promise<AsyncIterable<string>> {
    const query = buildQueryString([
      { name: 'lastEventId', value: params?.lastEventId, style: 'form', explode: true, allowReserved: false },
      { name: 'live', value: params?.live, style: 'form', explode: true, allowReserved: false },
    ]);
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.streamJson<string>(appendQueryString(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/events/stream`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders });
  }
}

export interface IntelligenceRuntimeSessionsToolsListParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsToolsExecuteParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSessionsToolsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List session tools */
  async list(sessionId: string, params?: IntelligenceRuntimeSessionsToolsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData & { items?: ToolDescriptor[]; }> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkPageData & { items?: ToolDescriptor[]; }>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tools`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'page' });
  }

/** Execute a tool for a session */
  async execute(sessionId: string, toolName: string, body: ExecuteToolRequest, params: IntelligenceRuntimeSessionsToolsExecuteParams, requestOptions?: ApiRequestOptions): Promise<ExecuteToolResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<ExecuteToolResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tools/${serializePathParameter(toolName, { name: 'toolName', style: 'simple', explode: false })}/execute`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeSessionsModelInvokeParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsModelStreamParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsModelCancelParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSessionsModelApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Invoke a model for a session */
  async invoke(sessionId: string, body: InvokeModelRequest, params: IntelligenceRuntimeSessionsModelInvokeParams, requestOptions?: ApiRequestOptions): Promise<InvokeModelResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<InvokeModelResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/model/invoke`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Stream a model response via SSE */
  async stream(sessionId: string, body: StreamModelRequest, params?: IntelligenceRuntimeSessionsModelStreamParams, requestOptions?: ApiRequestOptions): Promise<AsyncIterable<string>> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.streamJson<string>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/model/stream`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json' });
  }

/** Cancel an in-flight model invocation */
  async cancel(sessionId: string, body: CancelModelRequest, params: IntelligenceRuntimeSessionsModelCancelParams, requestOptions?: ApiRequestOptions): Promise<CancelModelResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<CancelModelResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/model/cancel`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeSessionsTasksListParams {
  pageSize?: number;
  cursor?: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsTasksSubmitParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSessionsTasksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List session tasks */
  async list(sessionId: string, params?: IntelligenceRuntimeSessionsTasksListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData & { items: TaskResponse[]; }> {
    const query = buildQueryString([
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
    ]);
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkPageData & { items: TaskResponse[]; }>(appendQueryString(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tasks`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'page' });
  }

/** Submit a session task */
  async submit(sessionId: string, body: SubmitTaskRequest, params: IntelligenceRuntimeSessionsTasksSubmitParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkAsyncData> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkAsyncData>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tasks`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'command' });
  }
}

export interface IntelligenceRuntimeSessionsMessagesListParams {
  pageSize?: number;
  cursor?: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsMessagesSendParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSessionsMessagesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List session messages */
  async list(sessionId: string, params?: IntelligenceRuntimeSessionsMessagesListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData & { items: MessageResponse[]; }> {
    const query = buildQueryString([
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
    ]);
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkPageData & { items: MessageResponse[]; }>(appendQueryString(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/messages`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'page' });
  }

/** Send a session message */
  async send(sessionId: string, body: SendMessageRequest, params: IntelligenceRuntimeSessionsMessagesSendParams, requestOptions?: ApiRequestOptions): Promise<MessageTurnResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<MessageTurnResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/messages`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeSessionsListParams {
  pageSize?: number;
  cursor?: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsCreateParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsRetrieveParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsDeleteParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface IntelligenceRuntimeSessionsCloseParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSessionsApi {
  private client: HttpClient;
  public readonly messages: IntelligenceRuntimeSessionsMessagesApi;
  public readonly tasks: IntelligenceRuntimeSessionsTasksApi;
  public readonly model: IntelligenceRuntimeSessionsModelApi;
  public readonly tools: IntelligenceRuntimeSessionsToolsApi;
  public readonly events: IntelligenceRuntimeSessionsEventsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.messages = new IntelligenceRuntimeSessionsMessagesApi(client);
    this.tasks = new IntelligenceRuntimeSessionsTasksApi(client);
    this.model = new IntelligenceRuntimeSessionsModelApi(client);
    this.tools = new IntelligenceRuntimeSessionsToolsApi(client);
    this.events = new IntelligenceRuntimeSessionsEventsApi(client);
  }


/** List runtime sessions */
  async list(params?: IntelligenceRuntimeSessionsListParams, requestOptions?: ApiRequestOptions): Promise<SdkWorkPageData & { items: SessionResponse[]; }> {
    const query = buildQueryString([
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
    ]);
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SdkWorkPageData & { items: SessionResponse[]; }>(appendQueryString(customApiPath(`/intelligence/runtime/sessions`), query), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'page' });
  }

/** Create a runtime session */
  async create(body: CreateSessionRequest, params: IntelligenceRuntimeSessionsCreateParams, requestOptions?: ApiRequestOptions): Promise<SessionResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SessionResponse>(customApiPath(`/intelligence/runtime/sessions`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }

/** Retrieve a runtime session */
  async retrieve(sessionId: string, params?: IntelligenceRuntimeSessionsRetrieveParams, requestOptions?: ApiRequestOptions): Promise<SessionResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SessionResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }

/** Delete a runtime session */
  async delete(sessionId: string, params?: IntelligenceRuntimeSessionsDeleteParams, requestOptions?: ApiRequestOptions): Promise<void> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<void>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'DELETE' as any, headers: requestHeaders });
  }

/** Close a runtime session */
  async close(sessionId: string, params: IntelligenceRuntimeSessionsCloseParams, requestOptions?: ApiRequestOptions): Promise<SessionResponse> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<SessionResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/close`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimePermissionsDecideParams {
  idempotencyKey: string;
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimePermissionsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Decide a permission request */
  async decide(permissionRequestId: string, body: PermissionDecisionRequest, params: IntelligenceRuntimePermissionsDecideParams, requestOptions?: ApiRequestOptions): Promise<PermissionRequest> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        'x-sdkwork-tenant-id': { value: params.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<PermissionRequest>(customApiPath(`/intelligence/runtime/permissions/${serializePathParameter(permissionRequestId, { name: 'permissionRequestId', style: 'simple', explode: false })}`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'POST' as any, body, headers: requestHeaders, contentType: 'application/json', sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeSnapshotLoadParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeSnapshotApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Load runtime aggregate snapshot */
  async load(params?: IntelligenceRuntimeSnapshotLoadParams, requestOptions?: ApiRequestOptions): Promise<RuntimeSnapshot> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RuntimeSnapshot>(customApiPath(`/intelligence/runtime/snapshot`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeDiagnosticsGetParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeDiagnosticsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Load runtime diagnostics report */
  async get(params?: IntelligenceRuntimeDiagnosticsGetParams, requestOptions?: ApiRequestOptions): Promise<RuntimeDiagnostics> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RuntimeDiagnostics>(customApiPath(`/intelligence/runtime/diagnostics`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeHealthGetParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeHealthApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Runtime liveness and readiness probe */
  async get(params?: IntelligenceRuntimeHealthGetParams, requestOptions?: ApiRequestOptions): Promise<RuntimeHealth> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RuntimeHealth>(customApiPath(`/intelligence/runtime/health`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }
}

export interface IntelligenceRuntimeManifestGetParams {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export class IntelligenceRuntimeManifestApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Load runtime capability manifest */
  async get(params?: IntelligenceRuntimeManifestGetParams, requestOptions?: ApiRequestOptions): Promise<RuntimeManifest> {
    const requestHeaders = buildRequestHeaders(
      {
        'x-sdkwork-tenant-id': { value: params?.xSdkworkTenantId, style: 'simple', explode: false },
        'x-sdkwork-user-id': { value: params?.xSdkworkUserId, style: 'simple', explode: false },
        'x-sdkwork-identity-mac': { value: params?.xSdkworkIdentityMac, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.request<RuntimeManifest>(customApiPath(`/intelligence/runtime/manifest`), { signal: requestOptions?.signal, timeout: requestOptions?.timeout, method: 'GET' as any, headers: requestHeaders, sdkworkUnwrapKind: 'item' });
  }
}

export class IntelligenceRuntimeApi {
  private client: HttpClient;
  public readonly manifest: IntelligenceRuntimeManifestApi;
  public readonly health: IntelligenceRuntimeHealthApi;
  public readonly diagnostics: IntelligenceRuntimeDiagnosticsApi;
  public readonly snapshot: IntelligenceRuntimeSnapshotApi;
  public readonly permissions: IntelligenceRuntimePermissionsApi;
  public readonly sessions: IntelligenceRuntimeSessionsApi;
  public readonly tasks: IntelligenceRuntimeTasksApi;
  public readonly runs: IntelligenceRuntimeRunsApi;
  public readonly models: IntelligenceRuntimeModelsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.manifest = new IntelligenceRuntimeManifestApi(client);
    this.health = new IntelligenceRuntimeHealthApi(client);
    this.diagnostics = new IntelligenceRuntimeDiagnosticsApi(client);
    this.snapshot = new IntelligenceRuntimeSnapshotApi(client);
    this.permissions = new IntelligenceRuntimePermissionsApi(client);
    this.sessions = new IntelligenceRuntimeSessionsApi(client);
    this.tasks = new IntelligenceRuntimeTasksApi(client);
    this.runs = new IntelligenceRuntimeRunsApi(client);
    this.models = new IntelligenceRuntimeModelsApi(client);
  }

}

export class IntelligenceApi {
  private client: HttpClient;
  public readonly runtime: IntelligenceRuntimeApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.runtime = new IntelligenceRuntimeApi(client);
  }

}

export function createIntelligenceApi(client: HttpClient): IntelligenceApi {
  return new IntelligenceApi(client);
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
function buildRequestHeaders(
  headers: Record<string, HeaderParameterSpec | undefined>,
  cookies: Record<string, HeaderParameterSpec | undefined> = {},
): Record<string, string> | undefined {
  const requestHeaders: Record<string, string> = {};

  for (const [name, parameter] of Object.entries(headers)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      requestHeaders[name] = serialized;
    }
  }

  const cookieHeader = buildCookieHeader(cookies);
  if (cookieHeader) {
    requestHeaders.Cookie = requestHeaders.Cookie
      ? `${requestHeaders.Cookie}; ${cookieHeader}`
      : cookieHeader;
  }

  return Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined;
}

interface HeaderParameterSpec {
  value: unknown;
  style: string;
  explode: boolean;
  contentType?: string;
}

function buildCookieHeader(cookies: Record<string, HeaderParameterSpec | undefined>): string | undefined {
  const pairs: string[] = [];
  for (const [name, parameter] of Object.entries(cookies)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(serialized)}`);
    }
  }
  return pairs.length > 0 ? pairs.join('; ') : undefined;
}

function serializeParameterValue(parameter: HeaderParameterSpec | undefined): string | undefined {
  const value = parameter?.value;
  if (value === undefined || value === null) {
    return undefined;
  }
  if (parameter?.contentType) {
    return JSON.stringify(value);
  }
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (Array.isArray(value)) {
    return value.map((item) => serializeHeaderPrimitive(item)).join(',');
  }
  if (typeof value === 'object' && value !== null) {
    return serializeHeaderObject(value as Record<string, unknown>, parameter?.explode === true);
  }
  return serializeHeaderPrimitive(value);
}

function serializeHeaderObject(value: Record<string, unknown>, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (explode) {
    return entries.map(([key, entryValue]) => `${key}=${serializeHeaderPrimitive(entryValue)}`).join(',');
  }
  return entries.flatMap(([key, entryValue]) => [key, serializeHeaderPrimitive(entryValue)]).join(',');
}

function serializeHeaderPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  return String(value);
}
