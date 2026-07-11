export interface TerminalCommand {
  commandId: string;
  command: string;
  args: string[];
  workingDirectory: string;
  timeoutMs?: string | null;
  policyCategories?: string[];
}
