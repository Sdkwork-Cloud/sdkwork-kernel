import type { KnowledgeSyncJobKind } from './knowledge-sync-job-kind';

export interface CreateKnowledgeSyncJobRequest {
  syncJobId: string;
  knowledgeSourceId?: string | null;
  jobKind: KnowledgeSyncJobKind;
  inputRef: string;
  input?: Record<string, unknown>;
  requestedAt: string;
}
