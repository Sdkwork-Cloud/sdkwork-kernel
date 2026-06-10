import type { AgentDeploymentRecord } from './agent-deployment-record';
import type { PageInfo } from './page-info';

export interface AgentDeploymentListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
