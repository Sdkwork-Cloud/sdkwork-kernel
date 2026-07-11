import type { ModelToolCall } from './model-tool-call';

export interface InvokeModelResponse {
  modelRequestId: string;
  providerId: string;
  status: string;
  messages: string[];
  toolCalls: ModelToolCall[];
}
