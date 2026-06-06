import type { AgentStatus } from './agent-status';
import type { Int64String } from './int64-string';
import type { MemoryIndexKind } from './memory-index-kind';

export interface MemoryRetrievalIndexRecord {
  id: Int64String;
  tenantId: Int64String;
  memoryIndexId: string;
  memoryId: string;
  indexKind: MemoryIndexKind;
  indexProviderId: string;
  externalRef: string;
  embeddingModelId?: string | null;
  vectorDimension?: number | null;
  contentHash: string;
  indexedAt: string;
  status: AgentStatus;
}
