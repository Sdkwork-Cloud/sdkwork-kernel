import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { MemoryNamespaceKind } from './memory-namespace-kind';

export interface CreateMemoryNamespaceRequest {
  memoryNamespaceId: string;
  organizationId: Int64String;
  agentId?: string | null;
  userRef?: string | null;
  sessionRef?: string | null;
  threadRef?: string | null;
  namespaceKind: MemoryNamespaceKind;
  visibility: AgentVisibility;
  requestedAt: string;
}
