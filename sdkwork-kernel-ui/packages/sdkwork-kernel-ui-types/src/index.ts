export type KernelRuntimeState = 'ready' | 'degraded' | 'failed' | 'stopping' | 'stopped';

export type CapabilityStatus = 'available' | 'degraded' | 'missing';

export interface KernelCapabilityView {
  capabilityId: string;
  providerId: string;
  status: CapabilityStatus;
  required: boolean;
}

export interface KernelRuntimeSnapshot {
  runtimeId: string;
  agentId: string;
  kernelVersion: string;
  state: KernelRuntimeState;
  health: 'healthy' | 'degraded' | 'failed';
  capabilities: KernelCapabilityView[];
  missingRequiredCapabilities: string[];
  degradedCapabilities: string[];
}

export interface KernelEventView {
  eventId: string;
  eventType: string;
  severity: 'debug' | 'info' | 'warn' | 'error';
  summary: string;
  sequence: number;
  traceId?: string;
}

export type PermissionDecisionValue = 'allow' | 'deny' | 'needs_approval';

export interface PermissionRequestView {
  permissionRequestId: string;
  category: string;
  resource: string;
  sideEffectLevel: 'read_only' | 'side_effectful' | 'destructive' | 'external_send' | 'privileged';
  reason: string;
  status: 'pending' | PermissionDecisionValue;
}

export interface CodeWorkspaceView {
  workspaceId: string;
  root: string;
  branch: string;
  dirty: boolean;
  changedFiles: string[];
}

export interface PatchSetView {
  patchId: string;
  summary: string;
  status: 'draft' | 'validated' | 'applied' | 'rejected';
  changedFiles: string[];
  requiresPolicy: boolean;
}

export interface VerificationReportView {
  reportId: string;
  status: 'passed' | 'failed' | 'cancelled' | 'timed_out';
  command: string;
  failures: string[];
  evidence: string;
}

export interface TerminalCommandView {
  commandId: string;
  command: string;
  workingDirectory: string;
  status: 'queued' | 'running' | 'passed' | 'failed' | 'cancelled' | 'timed_out';
  exitCode?: number;
  durationMs?: number;
  requiresPolicy: boolean;
}

export interface TerminalOutputChunkView {
  commandId: string;
  sequence: number;
  channel: 'stdout' | 'stderr' | 'system';
  content: string;
  redacted: boolean;
}

export interface ReviewFindingView {
  findingId: string;
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info';
  filePath: string;
  line?: number;
  message: string;
  missingTest?: string;
}

// ============================================================================
// Session Types
// ============================================================================

export type SessionState = 'created' | 'active' | 'paused' | 'waiting' | 'working' | 'closed' | 'failed' | 'archived';

export type SessionKind = 'main' | 'subagent' | 'background' | 'direct' | 'group' | 'task' | 'ephemeral';

export type SessionSource = 'cli' | 'api' | 'web' | 'telegram' | 'slack' | 'discord' | 'ide' | 'desktop' | 'mobile' | 'scheduled' | 'unknown';

export interface SessionTokenUsage {
  inputTokens: number;
  outputTokens: number;
  cachedTokens: number;
  reasoningTokens: number;
  totalTokens: number;
}

export interface SessionChangeSummary {
  additions: number;
  deletions: number;
  filesChanged: number;
}

export interface SessionView {
  sessionId: string;
  parentSessionId?: string;
  forkedFromId?: string;
  slug?: string;
  source: SessionSource;
  kind: SessionKind;
  agentId?: string;
  userRef?: string;
  tenantId?: string;
  title?: string;
  preview?: string;
  goal?: string;
  summary?: string;
  state: SessionState;
  createdAt?: string;
  updatedAt?: string;
  endedAt?: string;
  archivedAt?: string;
  model?: string;
  modelProvider?: string;
  cwd?: string;
  workspaceRoots: string[];
  instructions?: string;
  tokenUsage: SessionTokenUsage;
  messageCount: number;
  toolCallCount: number;
  compressionCount: number;
  costCents?: number;
  changeSummary: SessionChangeSummary;
  childSessionIds: string[];
  timeoutMs?: number;
  metadata: Record<string, string>;
}

export interface SessionConfig {
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
  source?: SessionSource;
  kind?: SessionKind;
  timeoutMs?: number;
  metadata?: Record<string, string>;
}

// ============================================================================
// Message Types
// ============================================================================

export type MessageRole = 'user' | 'assistant' | 'system' | 'tool';

export interface MessagePartView {
  partId: string;
  kind: 'text' | 'tool_call_ref' | 'artifact_ref' | 'code_block' | 'image';
  content: string;
  toolCallId?: string;
  artifactId?: string;
  mimeType?: string;
}

export interface MessageView {
  messageId: string;
  sessionId: string;
  role: MessageRole;
  parts: MessagePartView[];
  createdAt?: string;
  metadata: Record<string, string>;
}

// ============================================================================
// Task Types
// ============================================================================

export type TaskState = 'created' | 'accepted' | 'planned' | 'running' | 'awaiting_permission' | 'paused' | 'completed' | 'failed' | 'cancelled';

export interface TaskView {
  taskId: string;
  sessionId: string;
  instruction: string;
  state: TaskState;
  createdAt?: string;
  updatedAt?: string;
}

// ============================================================================
// Tool Types
// ============================================================================

export interface ToolDescriptorView {
  toolId: string;
  providerId: string;
  name?: string;
  displayName: string;
  description?: string;
  sideEffectLevel: 'read_only' | 'side_effectful' | 'destructive';
  policyCategories: string[];
  timeoutMs?: number;
}

export interface ToolCallView {
  toolCallId: string;
  toolId: string;
  input: string;
  status: 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled';
  output?: string;
  error?: string;
  durationMs?: number;
}

// ============================================================================
// Model Types
// ============================================================================

export interface ModelDescriptorView {
  modelId: string;
  providerId: string;
  displayName: string;
  family: string;
  contextWindowTokens?: number;
  maxOutputTokens?: number;
  capabilities: string[];
}

export interface ModelResponseView {
  modelRequestId: string;
  providerId: string;
  status: 'succeeded' | 'failed' | 'cancelled';
  messages: string[];
  toolCalls: ToolCallView[];
  usage?: {
    inputTokens: number;
    outputTokens: number;
  };
}

// ============================================================================
// Streaming Types
// ============================================================================

export interface StreamEventView {
  eventId: string;
  eventType: string;
  sequence: number;
  payload: string;
  timestamp?: string;
}

export type EventSubscription = {
  unsubscribe: () => void;
};

export interface EventSubscriptionOptions {
  lastEventId?: string;
  /** When false, replay buffered events only (no live tail). Defaults to server default (live). */
  live?: boolean;
  onError?: (error: Error) => void;
}

// ============================================================================
// Runtime Manifest / Health / Diagnostics
// ============================================================================

export interface ProviderManifestView {
  providerId: string;
  providerFamily: string;
  name: string;
  version: string;
  capabilities: string[];
  healthStatus?: string;
}

export interface RuntimeManifestView {
  runtimeId: string;
  agentId: string;
  kernelVersion: string;
  securityProfile: string;
  capabilities: KernelCapabilityView[];
  providers: ProviderManifestView[];
  missingRequiredCapabilities: string[];
  degradedCapabilities: string[];
}

export interface RuntimeHealthView {
  runtimeId: string;
  state: KernelRuntimeState;
  health: 'healthy' | 'degraded';
  persistenceHealthy: boolean;
  degradedCapabilities: string[];
}

export interface ProviderDiagnosticView {
  providerId: string;
  providerFamily: string;
  providerVersion: string;
  typedRegistered: boolean;
  healthStatus?: string;
  capabilities: string[];
}

export interface RuntimeDiagnosticsView {
  runtimeId: string;
  agentId: string;
  state: KernelRuntimeState;
  providerCount: number;
  capabilityCount: number;
  typedProviderCount: number;
  manifestOnlyProviderCount: number;
  missingRequiredCapabilities: string[];
  degradedCapabilities: string[];
  providerDiagnostics: ProviderDiagnosticView[];
}

// ============================================================================
// Snapshot
// ============================================================================

export interface KernelUiSnapshot {
  runtime: KernelRuntimeSnapshot;
  events: KernelEventView[];
  permissions: PermissionRequestView[];
  workspace: CodeWorkspaceView;
  patches: PatchSetView[];
  verificationReports: VerificationReportView[];
  terminalCommands: TerminalCommandView[];
  terminalOutput: TerminalOutputChunkView[];
  reviewFindings: ReviewFindingView[];
}

// ============================================================================
// Client Interface
// ============================================================================

export interface KernelUiClient {
  // Runtime introspection (AGENT_RUNTIME_SPEC §4)
  getRuntimeManifest(): Promise<RuntimeManifestView>;
  getRuntimeHealth(): Promise<RuntimeHealthView>;
  getRuntimeDiagnostics(): Promise<RuntimeDiagnosticsView>;

  // Existing
  loadSnapshot(): Promise<KernelUiSnapshot>;
  decidePermission(
    permissionRequestId: string,
    decision: PermissionDecisionValue
  ): Promise<PermissionRequestView>;

  // Session management
  createSession(config: SessionConfig): Promise<SessionView>;
  getSession(sessionId: string): Promise<SessionView>;
  listSessions(): Promise<SessionView[]>;
  closeSession(sessionId: string): Promise<SessionView>;
  deleteSession(sessionId: string): Promise<void>;

  // Message operations
  sendMessage(sessionId: string, content: string): Promise<MessageView>;
  getMessages(sessionId: string, limit?: number, offset?: number): Promise<MessageView[]>;

  // Task operations
  submitTask(sessionId: string, instruction: string): Promise<TaskView>;
  getTask(taskId: string): Promise<TaskView>;
  listTasks(sessionId: string): Promise<TaskView[]>;
  cancelTask(taskId: string): Promise<TaskView>;

  // Model operations
  listModels(): Promise<ModelDescriptorView[]>;
  invokeModel(sessionId: string, modelId?: string): Promise<ModelResponseView>;

  // Tool operations
  listTools(sessionId: string): Promise<ToolDescriptorView[]>;
  executeTool(sessionId: string, toolName: string, args: string): Promise<ToolCallView>;

  // Streaming
  subscribeEvents(
    sessionId: string,
    callback: (event: StreamEventView) => void,
    options?: EventSubscriptionOptions
  ): EventSubscription;
}

export type { KernelUiAuthProvider, KernelUiAuthSession } from './auth/kernel-ui-auth.types';
