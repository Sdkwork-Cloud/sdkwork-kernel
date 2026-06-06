import type { KnowledgeBindingRecord } from './knowledge-binding-record';

export interface KnowledgeBindingResponse {
  data: KnowledgeBindingRecord;
  requestId?: string;
}
