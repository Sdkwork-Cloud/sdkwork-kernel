import type { AgentImplementationKind } from './agent-implementation-kind';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';

export interface CreateAgentRequest {
  agentId: string;
  organizationId: Int64String;
  ownerUserId: Int64String;
  code: string;
  displayName: string;
  description?: string | null;
  manifest: Record<string, unknown>;
  defaultCodeTaskIntent?: Record<string, unknown>;
  implementationProviderId?: string | null;
  implementationKind?: AgentImplementationKind | null;
  visibility: AgentVisibility;
  tags?: string[];
  requestedAt: string;
}
