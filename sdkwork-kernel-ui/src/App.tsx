import { useEffect, useMemo, useState } from 'react';
import { AgentRuntimePanel } from '@sdkwork/kernel-ui-agent';
import { CodeKernelPanel } from '@sdkwork/kernel-ui-code';
import { translateKernelUi } from '@sdkwork/kernel-ui-commons';
import { createKernelUiRuntime } from '@sdkwork/kernel-ui-core';
import { PermissionQueue } from '@sdkwork/kernel-ui-permissions';
import { TerminalKernelPanel } from '@sdkwork/kernel-ui-terminal';
import { TelemetryEventStream } from '@sdkwork/kernel-ui-telemetry';
import type { KernelUiSnapshot, PermissionDecisionValue } from '@sdkwork/kernel-ui-types';
import { WorkspaceKernelPanel } from '@sdkwork/kernel-ui-workspace';
import { KernelUiSessionPanel } from './KernelUiSessionPanel';
import { createKernelUiShellClient, needsKernelUiSessionGate } from './kernel-ui-client';
import './styles.css';

export function App() {
  const [sessionGateOpen, setSessionGateOpen] = useState(() => needsKernelUiSessionGate());
  const [clientVersion, setClientVersion] = useState(0);
  const client = useMemo(() => createKernelUiShellClient(), [clientVersion]);
  const runtime = useMemo(() => createKernelUiRuntime(client), [client]);
  const [snapshot, setSnapshot] = useState<KernelUiSnapshot | null>(null);

  useEffect(() => {
    if (sessionGateOpen) {
      return;
    }
    void runtime.loadSnapshot().then(setSnapshot);
  }, [runtime, sessionGateOpen]);

  if (sessionGateOpen) {
    return (
      <KernelUiSessionPanel
        onSessionSaved={() => {
          setSessionGateOpen(false);
          setClientVersion((version) => version + 1);
        }}
      />
    );
  }

  if (!snapshot) {
    return <main className="kernel-ui-shell">{translateKernelUi('app.loading')}</main>;
  }

  const handleDecision = (permissionRequestId: string, decision: PermissionDecisionValue) => {
    void runtime.client.decidePermission(permissionRequestId, decision).then(() => {
      void runtime.loadSnapshot().then(setSnapshot);
    });
  };

  return (
    <main className="kernel-ui-shell">
      <header className="kernel-ui-shell__header">
        <div>
          <p>SDKWork Kernel Standard</p>
          <h1>{translateKernelUi('app.title')}</h1>
        </div>
      </header>
      <section className="kernel-ui-shell__grid">
        <AgentRuntimePanel runtime={snapshot.runtime} />
        <PermissionQueue permissions={snapshot.permissions} onDecision={handleDecision} />
        <WorkspaceKernelPanel workspace={snapshot.workspace} />
        <CodeKernelPanel
          patches={snapshot.patches}
          verificationReports={snapshot.verificationReports}
          reviewFindings={snapshot.reviewFindings}
        />
        <TerminalKernelPanel commands={snapshot.terminalCommands} output={snapshot.terminalOutput} />
        <TelemetryEventStream events={snapshot.events} />
      </section>
    </main>
  );
}
