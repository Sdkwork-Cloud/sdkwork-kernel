import type { AgentImplementationKind } from './agent-implementation-kind';
import type { AgentImplementationType } from './agent-implementation-type';
import type { AgentManagementProfile } from './agent-management-profile';
import type { AgentVisibility } from './agent-visibility';

export interface CreateAgentRequest {
  agentId: string;
  code: string;
  displayName: string;
  description?: string | null;
  manifest: Record<string, unknown>;
  defaultCodeTaskIntent?: Record<string, unknown>;
  managementProfile?: AgentManagementProfile | null;
  implementationProviderId?: string | null;
  implementationKind?: AgentImplementationKind | null;
  implementationType?: AgentImplementationType;
  visibility: AgentVisibility;
  tags?: string[];
  requestedAt: string;
}
