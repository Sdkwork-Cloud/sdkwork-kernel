import type { Int64String } from './int64-string';
import type { KnowledgeDocumentKind } from './knowledge-document-kind';
import type { KnowledgeIndexKind } from './knowledge-index-kind';

/** Provider-neutral RAG retrieval candidate with provenance. It is not a generated answer and does not require vector retrieval. */
export interface KnowledgeSearchResult {
  tenantId: Int64String;
  knowledgeBaseId: string;
  providerId: string;
  knowledgeIndexId: string;
  indexProviderId: string;
  retrievalMethod: KnowledgeIndexKind;
  knowledgeDocumentId?: string | null;
  documentKind?: KnowledgeDocumentKind | null;
  knowledgeChunkId?: string | null;
  title: string;
  snippet?: string | null;
  score?: number | null;
  sourceRef?: string | null;
  contentRef?: string | null;
  externalRef?: string | null;
  trustLevel: number;
  redactionClassification: string;
  metadata: Record<string, unknown>;
}
