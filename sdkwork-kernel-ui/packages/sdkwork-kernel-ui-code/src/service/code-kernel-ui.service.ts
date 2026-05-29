import type { StatusTone } from '@sdkwork/kernel-ui-commons';
import type { ReviewFindingView, VerificationReportView } from '@sdkwork/kernel-ui-types';
import type { CodeKernelPanelProps, CodeKernelSummary } from '../types/code-kernel-ui.types';

export function summarizeCodeKernel({
  patches,
  verificationReports,
  reviewFindings
}: CodeKernelPanelProps): CodeKernelSummary {
  const failedVerificationCount = verificationReports.filter((report) => report.status !== 'passed').length;

  return {
    patchCount: patches.length,
    verificationCount: verificationReports.length,
    findingCount: reviewFindings.length,
    failedVerificationCount,
    findingTone: reviewFindings.length ? 'warn' : 'good'
  };
}

export function verificationStatusTone(report: VerificationReportView): StatusTone {
  return report.status === 'passed' ? 'good' : 'bad';
}

export function reviewSeverityTone(finding: ReviewFindingView): StatusTone {
  return finding.severity === 'high' || finding.severity === 'critical' ? 'bad' : 'warn';
}
