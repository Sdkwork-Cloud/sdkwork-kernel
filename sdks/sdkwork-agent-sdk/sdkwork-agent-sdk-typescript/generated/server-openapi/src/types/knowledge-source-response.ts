import type { KnowledgeSourceRecord } from './knowledge-source-record';

export interface KnowledgeSourceResponse {
  data: KnowledgeSourceRecord;
  requestId?: string;
}
