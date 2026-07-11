import type { KernelEvent } from './kernel-event';
import type { KernelRuntime } from './kernel-runtime';
import type { PatchSet } from './patch-set';
import type { PermissionRequest } from './permission-request';
import type { ReviewFinding } from './review-finding';
import type { TerminalCommand } from './terminal-command';
import type { TerminalOutputChunk } from './terminal-output-chunk';
import type { VerificationReport } from './verification-report';
import type { Workspace } from './workspace';

export interface RuntimeSnapshot {
  runtime: KernelRuntime;
  events: KernelEvent[];
  permissions: PermissionRequest[];
  workspace: Workspace;
  patches: PatchSet[];
  verificationReports: VerificationReport[];
  terminalCommands: TerminalCommand[];
  terminalOutput: TerminalOutputChunk[];
  reviewFindings: ReviewFinding[];
}
