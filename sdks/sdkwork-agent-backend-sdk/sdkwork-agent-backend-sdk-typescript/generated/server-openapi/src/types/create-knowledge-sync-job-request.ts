import type { Int64String } from './int64-string';
import type { KnowledgeSyncJobKind } from './knowledge-sync-job-kind';

export interface CreateKnowledgeSyncJobRequest {
  syncJobId: string;
  organizationId: Int64String;
  knowledgeSourceId?: string | null;
  jobKind: KnowledgeSyncJobKind;
  inputRef: string;
  input?: Record<string, unknown>;
  requestedAt: string;
}
