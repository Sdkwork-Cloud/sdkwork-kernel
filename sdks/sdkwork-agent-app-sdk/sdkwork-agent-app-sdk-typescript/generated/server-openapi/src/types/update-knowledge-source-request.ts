import type { Int64String } from './int64-string';
import type { KnowledgeSourceKind } from './knowledge-source-kind';

export interface UpdateKnowledgeSourceRequest {
  expectedVersion?: Int64String;
  sourceKind?: KnowledgeSourceKind;
  sourceRef?: string;
  sourceHash?: string;
  syncPolicy?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  requestedAt: string;
}
