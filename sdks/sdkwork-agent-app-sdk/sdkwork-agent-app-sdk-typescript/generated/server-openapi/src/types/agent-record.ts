import type { AgentImplementationKind } from './agent-implementation-kind';
import type { AgentStatus } from './agent-status';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';

export interface AgentRecord {
  id: Int64String;
  agentId: string;
  tenantId: Int64String;
  organizationId: Int64String;
  ownerUserId: Int64String;
  code: string;
  displayName: string;
  description?: string | null;
  manifest: Record<string, unknown>;
  defaultCodeTaskIntent?: Record<string, unknown>;
  implementationProviderId?: string | null;
  implementationKind?: AgentImplementationKind | null;
  status: AgentStatus;
  visibility: AgentVisibility;
  tags: string[];
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
