import type { KnowledgeDocumentRecord } from './knowledge-document-record';

export interface KnowledgeDocumentResponse {
  data: KnowledgeDocumentRecord;
  requestId?: string;
}
