import type { KnowledgeSourceKind } from './knowledge-source-kind';

export interface CreateKnowledgeSourceRequest {
  knowledgeSourceId: string;
  sourceKind: KnowledgeSourceKind;
  sourceRef: string;
  sourceHash: string;
  syncPolicy?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  requestedAt: string;
}
