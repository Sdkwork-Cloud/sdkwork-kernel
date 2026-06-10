import type { Int64String } from './int64-string';

export interface AgentRuntimeExecutionRecord {
  tenantId: Int64String;
  agentId: string;
  executionId: string;
  operation: 'preview_response' | 'prompt_optimization';
  status: 'completed';
  inputPayload: Record<string, unknown>;
  outputPayload: Record<string, unknown>;
  requestedAt: string;
  completedAt: string;
}
