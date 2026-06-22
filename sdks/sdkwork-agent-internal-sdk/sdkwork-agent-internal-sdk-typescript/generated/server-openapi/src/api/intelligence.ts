import { customApiPath } from './paths';
import type { HttpClient } from '../http/client';

import type { CreateSessionRequest, ExecuteToolRequest, ExecuteToolResponse, InvokeModelRequest, InvokeModelResponse, KernelUiSnapshot, MessageListResponse, MessageResponse, ModelListResponse, PermissionDecisionRequest, PermissionRequest, SendMessageRequest, SessionListResponse, SessionResponse, SubmitTaskRequest, TaskListResponse, TaskResponse, ToolListResponse } from '../types';


export class IntelligenceRuntimeModelsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List available models */
  async list(): Promise<ModelListResponse> {
    return this.client.get<ModelListResponse>(customApiPath(`/intelligence/runtime/models`));
  }
}

export class IntelligenceRuntimeTasksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve a task */
  async retrieve(taskId: string): Promise<TaskResponse> {
    return this.client.get<TaskResponse>(customApiPath(`/intelligence/runtime/tasks/${serializePathParameter(taskId, { name: 'taskId', style: 'simple', explode: false })}`));
  }

/** Cancel a task */
  async cancel(taskId: string): Promise<TaskResponse> {
    return this.client.post<TaskResponse>(customApiPath(`/intelligence/runtime/tasks/${serializePathParameter(taskId, { name: 'taskId', style: 'simple', explode: false })}/cancel`));
  }
}

export interface IntelligenceRuntimeSessionsEventsStreamParams {
  lastEventId?: string;
  live?: boolean;
}

export class IntelligenceRuntimeSessionsEventsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Stream session events */
  async stream(sessionId: string, params?: IntelligenceRuntimeSessionsEventsStreamParams): Promise<AsyncIterable<string>> {
    const query = buildQueryString([
      { name: 'lastEventId', value: params?.lastEventId, style: 'form', explode: true, allowReserved: false },
      { name: 'live', value: params?.live, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.streamJson<string>(appendQueryString(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/events/stream`), query), { method: 'GET' as any });
  }
}

export class IntelligenceRuntimeSessionsToolsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List session tools */
  async list(sessionId: string): Promise<ToolListResponse> {
    return this.client.get<ToolListResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tools`));
  }

/** Execute a tool for a session */
  async execute(sessionId: string, toolName: string, body: ExecuteToolRequest): Promise<ExecuteToolResponse> {
    return this.client.post<ExecuteToolResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tools/${serializePathParameter(toolName, { name: 'toolName', style: 'simple', explode: false })}/execute`), body, undefined, undefined, 'application/json');
  }
}

export class IntelligenceRuntimeSessionsModelApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Invoke a model for a session */
  async invoke(sessionId: string, body: InvokeModelRequest): Promise<InvokeModelResponse> {
    return this.client.post<InvokeModelResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/model/invoke`), body, undefined, undefined, 'application/json');
  }
}

export class IntelligenceRuntimeSessionsTasksApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List session tasks */
  async list(sessionId: string): Promise<TaskListResponse> {
    return this.client.get<TaskListResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tasks`));
  }

/** Submit a session task */
  async submit(sessionId: string, body: SubmitTaskRequest): Promise<TaskResponse> {
    return this.client.post<TaskResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/tasks`), body, undefined, undefined, 'application/json');
  }
}

export interface IntelligenceRuntimeSessionsMessagesListParams {
  limit?: number;
  offset?: number;
}

export class IntelligenceRuntimeSessionsMessagesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List session messages */
  async list(sessionId: string, params?: IntelligenceRuntimeSessionsMessagesListParams): Promise<MessageListResponse> {
    const query = buildQueryString([
      { name: 'limit', value: params?.limit, style: 'form', explode: true, allowReserved: false },
      { name: 'offset', value: params?.offset, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<MessageListResponse>(appendQueryString(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/messages`), query));
  }

/** Send a session message */
  async send(sessionId: string, body: SendMessageRequest): Promise<MessageResponse> {
    return this.client.post<MessageResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/messages`), body, undefined, undefined, 'application/json');
  }
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
  async list(): Promise<SessionListResponse> {
    return this.client.get<SessionListResponse>(customApiPath(`/intelligence/runtime/sessions`));
  }

/** Create a runtime session */
  async create(body: CreateSessionRequest): Promise<SessionResponse> {
    return this.client.post<SessionResponse>(customApiPath(`/intelligence/runtime/sessions`), body, undefined, undefined, 'application/json');
  }

/** Retrieve a runtime session */
  async retrieve(sessionId: string): Promise<SessionResponse> {
    return this.client.get<SessionResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}`));
  }

/** Delete a runtime session */
  async delete(sessionId: string): Promise<void> {
    return this.client.delete<void>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}`));
  }

/** Close a runtime session */
  async close(sessionId: string): Promise<SessionResponse> {
    return this.client.post<SessionResponse>(customApiPath(`/intelligence/runtime/sessions/${serializePathParameter(sessionId, { name: 'sessionId', style: 'simple', explode: false })}/close`));
  }
}

export class IntelligenceRuntimePermissionsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Decide a permission request */
  async decide(permissionRequestId: string, body: PermissionDecisionRequest): Promise<PermissionRequest> {
    return this.client.post<PermissionRequest>(customApiPath(`/intelligence/runtime/permissions/${serializePathParameter(permissionRequestId, { name: 'permissionRequestId', style: 'simple', explode: false })}`), body, undefined, undefined, 'application/json');
  }
}

export class IntelligenceRuntimeSnapshotApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Load kernel UI aggregate snapshot */
  async load(): Promise<KernelUiSnapshot> {
    return this.client.get<KernelUiSnapshot>(customApiPath(`/intelligence/runtime/snapshot`));
  }
}

export class IntelligenceRuntimeApi {
  private client: HttpClient;
  public readonly snapshot: IntelligenceRuntimeSnapshotApi;
  public readonly permissions: IntelligenceRuntimePermissionsApi;
  public readonly sessions: IntelligenceRuntimeSessionsApi;
  public readonly tasks: IntelligenceRuntimeTasksApi;
  public readonly models: IntelligenceRuntimeModelsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.snapshot = new IntelligenceRuntimeSnapshotApi(client);
    this.permissions = new IntelligenceRuntimePermissionsApi(client);
    this.sessions = new IntelligenceRuntimeSessionsApi(client);
    this.tasks = new IntelligenceRuntimeTasksApi(client);
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
