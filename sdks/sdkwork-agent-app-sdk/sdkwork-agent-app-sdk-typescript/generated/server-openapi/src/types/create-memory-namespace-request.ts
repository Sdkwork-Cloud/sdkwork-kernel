import type { AgentVisibility } from './agent-visibility';
import type { MemoryNamespaceKind } from './memory-namespace-kind';

export interface CreateMemoryNamespaceRequest {
  memoryNamespaceId: string;
  agentId?: string | null;
  userRef?: string | null;
  sessionRef?: string | null;
  threadRef?: string | null;
  namespaceKind: MemoryNamespaceKind;
  visibility: AgentVisibility;
  requestedAt: string;
}
