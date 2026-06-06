import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { MemoryIndexKind } from './memory-index-kind';
import type { MemoryStoreKind } from './memory-store-kind';

export interface UpdateMemoryStoreRequest {
  expectedVersion?: Int64String;
  displayName?: string;
  description?: string | null;
  providerId?: string;
  storeKind?: MemoryStoreKind;
  retrievalModes?: MemoryIndexKind[];
  capabilityIds?: string[];
  configurationProfileId?: string;
  visibility?: AgentVisibility;
  requestedAt: string;
}
