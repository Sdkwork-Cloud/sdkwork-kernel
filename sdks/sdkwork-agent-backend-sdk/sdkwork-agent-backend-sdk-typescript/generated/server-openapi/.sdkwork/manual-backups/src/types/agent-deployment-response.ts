import type { AgentDeploymentRecord } from './agent-deployment-record';

export interface AgentDeploymentResponse {
  data: AgentDeploymentRecord;
  requestId?: string;
}
