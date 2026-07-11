import { customApiPath } from '../generated/server-openapi/src/api/paths.ts';
import type { HttpClient } from '../generated/server-openapi/src/http/client';
import type { StreamModelRequest as GeneratedStreamModelRequest } from '../generated/server-openapi/src/types';
import {
  decodeSseJsonEvents,
  SdkworkSseProtocolError,
  type SdkworkSseEvent,
} from './sse-parser.ts';

interface GeneratedSseRequestOptions {
  method?: string;
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

interface GeneratedSseTransport {
  stream(path: string, options?: GeneratedSseRequestOptions): AsyncIterable<string>;
}

export interface AgentInternalIngressContext {
  xSdkworkTenantId?: string;
  xSdkworkUserId?: string;
  xSdkworkIdentityMac?: string;
}

export interface AgentInternalStreamModelRequest
  extends AgentInternalIngressContext,
    GeneratedStreamModelRequest {
  sessionId: string;
  signal?: AbortSignal;
}

export interface AgentInternalStreamSessionEventsRequest extends AgentInternalIngressContext {
  sessionId: string;
  lastEventId?: string;
  live?: boolean;
  signal?: AbortSignal;
}

export interface AgentInternalModelStreamChunk {
  modelRequestId: string;
  sequence: number;
  content: string;
  finishReason?: string | null;
}

export interface AgentInternalModelStreamError {
  message: string;
}

export type AgentInternalModelStreamEvent =
  | { event: 'model.chunk'; id?: string; data: AgentInternalModelStreamChunk }
  | { event: 'model.done'; id?: string; data: Record<string, never> }
  | { event: 'model.error'; id?: string; data: AgentInternalModelStreamError };

export interface AgentInternalSessionRuntimeStreamData {
  eventId: string;
  eventType: string;
  sequence: number;
  payload: string;
  timestamp?: string | null;
}

export interface AgentInternalSessionRuntimeStreamEvent {
  event: string;
  id?: string;
  data: AgentInternalSessionRuntimeStreamData;
}

export interface AgentInternalStreamingApi {
  model(request: AgentInternalStreamModelRequest): AsyncIterable<AgentInternalModelStreamEvent>;
  sessionEvents(
    request: AgentInternalStreamSessionEventsRequest,
  ): AsyncIterable<AgentInternalSessionRuntimeStreamEvent>;
}

export function createAgentInternalStreamingApi(http: HttpClient): AgentInternalStreamingApi {
  const transport = http as HttpClient & GeneratedSseTransport;
  return {
    model: (request) => streamModel(transport, request),
    sessionEvents: (request) => streamSessionEvents(transport, request),
  };
}

async function* streamModel(
  http: GeneratedSseTransport,
  request: AgentInternalStreamModelRequest,
): AsyncIterable<AgentInternalModelStreamEvent> {
  const path = customApiPath(
    `/intelligence/runtime/sessions/${encodeURIComponent(request.sessionId)}/model/stream`,
  );
  const lines = http.stream(path, {
    method: 'POST',
    body: {
      modelId: request.modelId,
      messages: request.messages,
    },
    headers: buildIngressHeaders(request),
    signal: request.signal,
  });

  for await (const rawEvent of decodeSseJsonEvents(lines)) {
    yield parseModelStreamEvent(rawEvent);
  }
}

async function* streamSessionEvents(
  http: GeneratedSseTransport,
  request: AgentInternalStreamSessionEventsRequest,
): AsyncIterable<AgentInternalSessionRuntimeStreamEvent> {
  const query = new URLSearchParams();
  if (request.lastEventId) {
    query.set('lastEventId', request.lastEventId);
  }
  if (request.live !== undefined) {
    query.set('live', String(request.live));
  }
  const suffix = query.size > 0 ? `?${query.toString()}` : '';
  const path = customApiPath(
    `/intelligence/runtime/sessions/${encodeURIComponent(request.sessionId)}/events/stream${suffix}`,
  );
  const lines = http.stream(path, {
    method: 'GET',
    headers: buildIngressHeaders(request),
    signal: request.signal,
  });

  for await (const rawEvent of decodeSseJsonEvents(lines)) {
    const data = requireSessionRuntimeData(rawEvent.data);
    yield {
      event: rawEvent.event ?? data.eventType,
      id: rawEvent.id ?? data.eventId,
      data,
    };
  }
}

function parseModelStreamEvent(rawEvent: SdkworkSseEvent): AgentInternalModelStreamEvent {
  switch (rawEvent.event) {
    case 'model.chunk':
      return {
        event: 'model.chunk',
        id: rawEvent.id,
        data: requireModelStreamChunk(rawEvent.data),
      };
    case 'model.done':
      requireRecord(rawEvent.data, 'model.done data');
      return {
        event: 'model.done',
        id: rawEvent.id,
        data: {},
      };
    case 'model.error': {
      const record = requireRecord(rawEvent.data, 'model.error data');
      return {
        event: 'model.error',
        id: rawEvent.id,
        data: { message: requireString(record.message, 'model.error.message') },
      };
    }
    default:
      throw new SdkworkSseProtocolError(
        `unsupported model SSE event: ${rawEvent.event ?? '<missing>'}`,
      );
  }
}

function requireModelStreamChunk(value: unknown): AgentInternalModelStreamChunk {
  const record = requireRecord(value, 'model.chunk data');
  const chunk: AgentInternalModelStreamChunk = {
    modelRequestId: requireString(record.modelRequestId, 'modelRequestId'),
    sequence: requireNonNegativeInteger(record.sequence, 'sequence'),
    content: requireString(record.content, 'content'),
  };
  const finishReason = requireOptionalString(record.finishReason, 'finishReason');
  if (finishReason !== undefined) {
    chunk.finishReason = finishReason;
  }
  return chunk;
}

function requireSessionRuntimeData(value: unknown): AgentInternalSessionRuntimeStreamData {
  const record = requireRecord(value, 'session event data');
  const data: AgentInternalSessionRuntimeStreamData = {
    eventId: requireString(record.eventId, 'eventId'),
    eventType: requireString(record.eventType, 'eventType'),
    sequence: requireNonNegativeInteger(record.sequence, 'sequence'),
    payload: requireString(record.payload, 'payload'),
  };
  const timestamp = requireOptionalString(record.timestamp, 'timestamp');
  if (timestamp !== undefined) {
    data.timestamp = timestamp;
  }
  return data;
}

function buildIngressHeaders(
  context: AgentInternalIngressContext,
): Record<string, string> | undefined {
  const headers: Record<string, string> = {};
  if (context.xSdkworkTenantId) {
    headers['x-sdkwork-tenant-id'] = context.xSdkworkTenantId;
  }
  if (context.xSdkworkUserId) {
    headers['x-sdkwork-user-id'] = context.xSdkworkUserId;
  }
  if (context.xSdkworkIdentityMac) {
    headers['x-sdkwork-identity-mac'] = context.xSdkworkIdentityMac;
  }
  return Object.keys(headers).length > 0 ? headers : undefined;
}

function requireRecord(value: unknown, name: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new SdkworkSseProtocolError(`${name} must be a JSON object`);
  }
  return value as Record<string, unknown>;
}

function requireString(value: unknown, name: string): string {
  if (typeof value !== 'string') {
    throw new SdkworkSseProtocolError(`${name} must be a string`);
  }
  return value;
}

function requireOptionalString(value: unknown, name: string): string | null | undefined {
  if (value === undefined || value === null) {
    return value;
  }
  return requireString(value, name);
}

function requireNonNegativeInteger(value: unknown, name: string): number {
  if (!Number.isSafeInteger(value) || Number(value) < 0) {
    throw new SdkworkSseProtocolError(`${name} must be a non-negative safe integer`);
  }
  return Number(value);
}
