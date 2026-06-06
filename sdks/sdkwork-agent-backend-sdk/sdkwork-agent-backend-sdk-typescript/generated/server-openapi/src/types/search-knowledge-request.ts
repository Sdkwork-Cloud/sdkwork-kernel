import type { KnowledgeIndexKind } from './knowledge-index-kind';

export interface SearchKnowledgeRequest {
  query: string;
  topK?: number;
  retrievalModes?: KnowledgeIndexKind[];
  includeExternal?: boolean;
}
