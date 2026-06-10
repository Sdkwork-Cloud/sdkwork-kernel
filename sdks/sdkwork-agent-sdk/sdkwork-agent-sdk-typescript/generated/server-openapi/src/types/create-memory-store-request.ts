import type { AgentVisibility } from './agent-visibility';
import type { MemoryIndexKind } from './memory-index-kind';
import type { MemoryStoreKind } from './memory-store-kind';

export interface CreateMemoryStoreRequest {
  memoryStoreId: string;
  code: string;
  displayName: string;
  description?: string | null;
  providerId: string;
  storeKind: MemoryStoreKind;
  retrievalModes: MemoryIndexKind[];
  capabilityIds?: string[];
  configurationProfileId: string;
  visibility: AgentVisibility;
  requestedAt: string;
}
