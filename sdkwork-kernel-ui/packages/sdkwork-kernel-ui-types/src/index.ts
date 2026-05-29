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

export interface KernelUiClient {
  loadSnapshot(): Promise<KernelUiSnapshot>;
  decidePermission(
    permissionRequestId: string,
    decision: PermissionDecisionValue
  ): Promise<PermissionRequestView>;
}
