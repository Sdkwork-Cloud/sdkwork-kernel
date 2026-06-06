import type { KnowledgeSyncJobRecord } from './knowledge-sync-job-record';
import type { PageInfo } from './page-info';

export interface KnowledgeSyncJobListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
