import type { KnowledgeBindingRecord } from './knowledge-binding-record';
import type { PageInfo } from './page-info';

export interface KnowledgeBindingListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
