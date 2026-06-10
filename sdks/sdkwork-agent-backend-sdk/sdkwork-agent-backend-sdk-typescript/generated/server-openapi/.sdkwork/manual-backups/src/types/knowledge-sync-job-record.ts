import type { Int64String } from './int64-string';
import type { KnowledgeSyncJobKind } from './knowledge-sync-job-kind';
import type { KnowledgeSyncJobStatus } from './knowledge-sync-job-status';

export interface KnowledgeSyncJobRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  syncJobId: string;
  knowledgeBaseId: string;
  knowledgeSourceId?: string | null;
  jobKind: KnowledgeSyncJobKind;
  status: KnowledgeSyncJobStatus;
  inputRef: string;
  input: Record<string, unknown>;
  output?: Record<string, unknown>;
  error?: Record<string, unknown>;
  requestedAt: string;
  startedAt?: string;
  completedAt?: string;
  createdAt: string;
  updatedAt: string;
}
