import type { KernelEvent } from './kernel-event';
import type { KernelRuntime } from './kernel-runtime';
import type { PermissionRequest } from './permission-request';
import type { Workspace } from './workspace';

export interface RuntimeSnapshot {
  runtime: KernelRuntime;
  events: KernelEvent[];
  permissions: PermissionRequest[];
  workspace: Workspace;
  patches: Record<string, unknown>[];
  verificationReports: Record<string, unknown>[];
  terminalCommands: Record<string, unknown>[];
  terminalOutput: Record<string, unknown>[];
  reviewFindings: Record<string, unknown>[];
}
