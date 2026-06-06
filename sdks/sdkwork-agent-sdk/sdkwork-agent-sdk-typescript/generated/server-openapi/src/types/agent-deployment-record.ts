import type { AgentImplementationKind } from './agent-implementation-kind';
import type { DeploymentStatus } from './deployment-status';
import type { Int64String } from './int64-string';

export interface AgentDeploymentRecord {
  tenantId: Int64String;
  agentId: string;
  deploymentId: string;
  bindingId: string;
  providerIdSnapshot: string;
  implementationKindSnapshot: AgentImplementationKind;
  configurationProfileIdSnapshot: string;
  capabilitiesSnapshot: string[];
  status: DeploymentStatus;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
}
