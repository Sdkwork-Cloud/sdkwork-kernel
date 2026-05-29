import { KernelPanel, MetricStrip, StatusBadge } from '@sdkwork/kernel-ui-commons';
import { useCodeKernelSummary } from '../hooks/useCodeKernelSummary';
import { reviewSeverityTone, verificationStatusTone } from '../service/code-kernel-ui.service';
import type { CodeKernelPanelProps } from '../types/code-kernel-ui.types';

export function CodeKernelPanel(props: CodeKernelPanelProps) {
  const { patches, verificationReports, reviewFindings } = props;
  const summary = useCodeKernelSummary(props);

  return (
    <KernelPanel
      title="Code Kernel"
      eyebrow="patch / verify / review"
      actions={<StatusBadge tone={summary.findingTone}>{summary.findingCount} findings</StatusBadge>}
    >
      <MetricStrip
        items={[
          { label: 'patches', value: summary.patchCount },
          { label: 'verification', value: summary.verificationCount },
          { label: 'findings', value: summary.findingCount, tone: summary.findingTone },
          {
            label: 'failed checks',
            value: summary.failedVerificationCount,
            tone: summary.failedVerificationCount ? 'bad' : 'good'
          }
        ]}
      />
      <div className="kernel-grid">
        <section>
          <h3>Patch Sets</h3>
          {patches.map((patch) => (
            <div className="list-row" key={patch.patchId}>
              <span>{patch.summary}</span>
              <StatusBadge tone={patch.requiresPolicy ? 'warn' : 'neutral'}>{patch.status}</StatusBadge>
            </div>
          ))}
        </section>
        <section>
          <h3>Verification</h3>
          {verificationReports.map((report) => (
            <div className="list-row" key={report.reportId}>
              <span>{report.command}</span>
              <StatusBadge tone={verificationStatusTone(report)}>{report.status}</StatusBadge>
            </div>
          ))}
        </section>
        <section>
          <h3>Review</h3>
          {reviewFindings.map((finding) => (
            <div className="list-row" key={finding.findingId}>
              <span>{finding.filePath}</span>
              <StatusBadge tone={reviewSeverityTone(finding)}>{finding.severity}</StatusBadge>
            </div>
          ))}
        </section>
      </div>
    </KernelPanel>
  );
}
