import type { KnowledgeSearchResult } from './knowledge-search-result';

export interface KnowledgeSearchResponse {
  data: Record<string, unknown>;
  requestId?: string;
}
