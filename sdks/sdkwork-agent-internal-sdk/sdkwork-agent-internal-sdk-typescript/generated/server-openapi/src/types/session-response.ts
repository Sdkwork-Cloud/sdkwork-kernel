import type { ChangeSummary } from './change-summary';
import type { TokenUsage } from './token-usage';

export interface SessionResponse {
  sessionId: string;
  source: string;
  kind: string;
  agentId: string;
  userRef?: string | null;
  tenantId?: string | null;
  title?: string | null;
  goal?: string | null;
  state: string;
  createdAt?: string | null;
  updatedAt?: string | null;
  model?: string | null;
  modelProvider?: string | null;
  cwd?: string | null;
  workspaceRoots: string[];
  instructions?: string | null;
  tokenUsage: TokenUsage;
  messageCount: number;
  toolCallCount: number;
  compressionCount: number;
  changeSummary: ChangeSummary;
  childSessionIds: string[];
  metadata: Record<string, string>;
}
