import type { KnowledgeBaseRecord } from './knowledge-base-record';

export interface KnowledgeBaseResponse {
  data: KnowledgeBaseRecord;
  requestId?: string;
}
