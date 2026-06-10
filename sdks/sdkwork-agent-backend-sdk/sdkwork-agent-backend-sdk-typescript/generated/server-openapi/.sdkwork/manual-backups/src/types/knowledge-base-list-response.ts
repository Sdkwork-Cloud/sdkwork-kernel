import type { KnowledgeBaseRecord } from './knowledge-base-record';
import type { PageInfo } from './page-info';

export interface KnowledgeBaseListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
