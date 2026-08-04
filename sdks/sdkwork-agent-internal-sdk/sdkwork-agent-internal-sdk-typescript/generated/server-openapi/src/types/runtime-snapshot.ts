import type { KernelEvent } from './kernel-event';
import type { KernelRuntime } from './kernel-runtime';
import type { PatchSet } from './patch-set';
import type { PermissionRequest } from './permission-request';
import type { ReviewFinding } from './review-finding';
import type { TerminalCommand } from './terminal-command';
import type { TerminalOutputChunk } from './terminal-output-chunk';
import type { VerificationReport } from './verification-report';
import type { Workspace } from './workspace';

/** Kernel runtime aggregate snapshot.  is null because the kernel runtime owns no workspace surface; the product layer (agents/BirdCoder) projects its own workspace snapshot when mounted. Patch/verification/terminal/review projections are kernel-absent and reported empty. */
export interface RuntimeSnapshot {
  runtime: KernelRuntime;
  events: KernelEvent[];
  permissions: PermissionRequest[];
  workspace?: Workspace | null;
  patches: PatchSet[];
  verificationReports: VerificationReport[];
  terminalCommands: TerminalCommand[];
  terminalOutput: TerminalOutputChunk[];
  reviewFindings: ReviewFinding[];
}
