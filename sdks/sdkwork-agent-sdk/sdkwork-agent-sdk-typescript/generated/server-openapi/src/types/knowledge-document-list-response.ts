import type { KnowledgeDocumentRecord } from './knowledge-document-record';
import type { PageInfo } from './page-info';

export interface KnowledgeDocumentListResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
