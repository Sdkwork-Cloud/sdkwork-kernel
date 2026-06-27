export interface InvokeModelResponse {
  modelRequestId: string;
  providerId: string;
  status: string;
  messages: string[];
  toolCalls: Record<string, unknown>[];
}
