import type { CommandResult } from './command-result';

export interface VerificationReport {
  reportId: string;
  verificationId: string;
  commandResults: CommandResult[];
  failures: string[];
}
