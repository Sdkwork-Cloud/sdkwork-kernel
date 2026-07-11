export interface CommandResult {
  commandId: string;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
  cancelled: boolean;
  timedOut: boolean;
}
