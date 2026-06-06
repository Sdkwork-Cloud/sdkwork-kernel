import type { KnowledgeSyncJobRecord } from './knowledge-sync-job-record';

export interface KnowledgeSyncJobResponse {
  data: KnowledgeSyncJobRecord;
  requestId?: string;
}
