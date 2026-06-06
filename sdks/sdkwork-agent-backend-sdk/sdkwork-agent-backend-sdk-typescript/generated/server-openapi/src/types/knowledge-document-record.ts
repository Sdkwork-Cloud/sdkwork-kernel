import type { AgentStatus } from './agent-status';
import type { AgentVisibility } from './agent-visibility';
import type { Int64String } from './int64-string';
import type { KnowledgeDocumentKind } from './knowledge-document-kind';

export interface KnowledgeDocumentRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  knowledgeDocumentId: string;
  knowledgeBaseId: string;
  knowledgeSourceId?: string | null;
  documentKind: KnowledgeDocumentKind;
  title: string;
  contentRef: string;
  contentHash: string;
  summary?: string | null;
  metadata: Record<string, unknown>;
  tags: string[];
  categories: string[];
  trustLevel: number;
  redactionClassification: string;
  chunkCount: number;
  status: AgentStatus;
  visibility: AgentVisibility;
  version: Int64String;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}
