import type { AgentAuditEvent } from './agent-audit-event';
import type { PageInfo } from './page-info';

export interface AgentAuditEventListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
