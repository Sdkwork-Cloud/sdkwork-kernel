import type { AgentStatus } from './agent-status';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { MemoryIndexKind } from './memory-index-kind';
import type { MemoryStoreKind } from './memory-store-kind';

export interface MemoryStoreRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  ownerUserId: Int64String;
  memoryStoreId: string;
  code: string;
  displayName: string;
  description?: string | null;
  providerId: string;
  storeKind: MemoryStoreKind;
  retrievalModes: MemoryIndexKind[];
  capabilityIds: string[];
  configurationProfileId: string;
  status: AgentStatus;
  visibility: AgentVisibility;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
