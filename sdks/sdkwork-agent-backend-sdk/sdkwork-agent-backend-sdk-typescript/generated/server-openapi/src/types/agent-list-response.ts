import type { AgentRecord } from './agent-record';
import type { PageInfo } from './page-info';

export interface AgentListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
