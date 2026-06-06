import type { AgentRuntimeExecutionRecord } from './agent-runtime-execution-record';

export interface AgentRuntimeExecutionResponse {
  data: AgentRuntimeExecutionRecord;
  requestId?: string;
}
