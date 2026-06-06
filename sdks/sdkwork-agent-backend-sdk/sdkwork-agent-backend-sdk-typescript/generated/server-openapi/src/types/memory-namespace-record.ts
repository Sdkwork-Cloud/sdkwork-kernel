import type { AgentStatus } from './agent-status';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { MemoryNamespaceKind } from './memory-namespace-kind';

export interface MemoryNamespaceRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  memoryNamespaceId: string;
  agentId?: string | null;
  userRef?: string | null;
  sessionRef?: string | null;
  threadRef?: string | null;
  namespaceKind: MemoryNamespaceKind;
  status: AgentStatus;
  visibility: AgentVisibility;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
