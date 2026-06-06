import type { AgentStatus } from './agent-status';
import type { Int64String } from './int64-string';

export interface UpdateAgentStatusRequest {
  targetStatus: AgentStatus;
  expectedVersion?: Int64String;
  requestedAt: string;
}
