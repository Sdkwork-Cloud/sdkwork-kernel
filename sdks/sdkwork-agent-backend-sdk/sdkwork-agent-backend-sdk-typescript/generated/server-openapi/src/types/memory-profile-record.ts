import type { AgentStatus } from './agent-status';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';

export interface MemoryProfileRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  ownerUserId: Int64String;
  memoryProfileId: string;
  memoryStoreId: string;
  code: string;
  displayName: string;
  description?: string | null;
  writePolicy: Record<string, unknown>;
  retrievalPolicy: Record<string, unknown>;
  compactionPolicy: Record<string, unknown>;
  retentionPolicy: Record<string, unknown>;
  privacyPolicy: Record<string, unknown>;
  status: AgentStatus;
  visibility: AgentVisibility;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
