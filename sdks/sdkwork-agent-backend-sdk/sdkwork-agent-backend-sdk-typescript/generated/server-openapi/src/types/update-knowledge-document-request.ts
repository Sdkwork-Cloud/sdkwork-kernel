import type { Int64String } from './int64-string';
import type { KnowledgeDocumentKind } from './knowledge-document-kind';
import type { KnowledgeDocumentProfile } from './knowledge-document-profile';

export interface UpdateKnowledgeDocumentRequest {
  expectedVersion?: Int64String;
  knowledgeSourceId?: string | null;
  documentKind?: KnowledgeDocumentKind;
  title?: string;
  contentRef?: string;
  contentHash?: string;
  summary?: string | null;
  metadata?: Record<string, unknown>;
  documentProfile?: KnowledgeDocumentProfile | null;
  tags?: string[];
  categories?: string[];
  trustLevel?: number;
  redactionClassification?: string;
  requestedAt: string;
}
