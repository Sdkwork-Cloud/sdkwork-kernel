import type { KnowledgeIndexRecord } from './knowledge-index-record';

export interface KnowledgeIndexResponse {
  data: KnowledgeIndexRecord;
  requestId?: string;
}
