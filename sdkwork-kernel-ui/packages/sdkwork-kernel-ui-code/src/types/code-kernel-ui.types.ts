import type { StatusTone } from '@sdkwork/kernel-ui-commons';
import type { PatchSetView, ReviewFindingView, VerificationReportView } from '@sdkwork/kernel-ui-types';

export interface CodeKernelPanelProps {
  patches: PatchSetView[];
  verificationReports: VerificationReportView[];
  reviewFindings: ReviewFindingView[];
}

export interface CodeKernelSummary {
  patchCount: number;
  verificationCount: number;
  findingCount: number;
  failedVerificationCount: number;
  findingTone: StatusTone;
}
