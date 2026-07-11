export interface ReviewFinding {
  findingId: string;
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info';
  filePath: string;
  line?: number | null;
  message: string;
  remediation?: string | null;
  missingTest?: string | null;
}
