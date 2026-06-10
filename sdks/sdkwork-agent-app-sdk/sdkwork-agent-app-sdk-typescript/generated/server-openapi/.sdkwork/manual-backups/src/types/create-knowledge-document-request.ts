import type { Int64String } from './int64-string';
import type { KnowledgeDocumentKind } from './knowledge-document-kind';

export interface CreateKnowledgeDocumentRequest {
  knowledgeDocumentId: string;
  organizationId: Int64String;
  knowledgeSourceId?: string | null;
  documentKind: KnowledgeDocumentKind;
  title: string;
  contentRef: string;
  contentHash: string;
  summary?: string | null;
  metadata?: Record<string, unknown>;
  tags?: string[];
  categories?: string[];
  trustLevel?: number;
  redactionClassification?: string;
  requestedAt: string;
}
