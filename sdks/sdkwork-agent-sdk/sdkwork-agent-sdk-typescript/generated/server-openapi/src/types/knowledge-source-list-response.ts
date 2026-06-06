import type { KnowledgeSourceRecord } from './knowledge-source-record';
import type { PageInfo } from './page-info';

export interface KnowledgeSourceListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
