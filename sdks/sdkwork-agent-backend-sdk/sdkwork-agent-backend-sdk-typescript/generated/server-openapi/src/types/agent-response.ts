import type { AgentRecord } from './agent-record';

export interface AgentResponse {
  data: AgentRecord;
  requestId?: string;
}
