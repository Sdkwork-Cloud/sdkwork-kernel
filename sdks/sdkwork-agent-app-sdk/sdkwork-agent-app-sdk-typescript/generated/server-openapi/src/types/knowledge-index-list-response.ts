import type { KnowledgeIndexRecord } from './knowledge-index-record';
import type { PageInfo } from './page-info';

export interface KnowledgeIndexListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
