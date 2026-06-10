import type { AgentStatus } from './agent-status';
import type { Int64String } from './int64-string';

export interface KnowledgeChunkRecord {
  id: Int64String;
  tenantId: Int64String;
  organizationId: Int64String;
  knowledgeChunkId: string;
  knowledgeDocumentId: string;
  parentChunkId?: string | null;
  chunkOrdinal: number;
  heading?: string | null;
  contentRef: string;
  contentHash: string;
  tokenEstimate: number;
  summary?: string | null;
  metadata: Record<string, unknown>;
  status: AgentStatus;
  createdAt: string;
}
