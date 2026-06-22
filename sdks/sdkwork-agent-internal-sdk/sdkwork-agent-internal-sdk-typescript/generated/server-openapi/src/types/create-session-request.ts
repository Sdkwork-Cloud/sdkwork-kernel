export interface CreateSessionRequest {
  agentId: string;
  tenantId?: string;
  userRef?: string;
  model?: string;
  modelProvider?: string;
  title?: string;
  goal?: string;
  instructions?: string;
  cwd?: string;
  workspaceRoots?: string[];
  source?: string;
  kind?: string;
  /** Session timeout in milliseconds. */
  timeoutMs?: number;
  metadata?: Record<string, string>;
}
