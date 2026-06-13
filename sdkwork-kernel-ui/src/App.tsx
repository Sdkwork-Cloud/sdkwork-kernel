import { useEffect, useMemo, useState } from 'react';
import { AgentRuntimePanel } from '@sdkwork/kernel-ui-agent';
import { CodeKernelPanel } from '@sdkwork/kernel-ui-code';
import { createKernelUiRuntime } from '@sdkwork/kernel-ui-core';
import { PermissionQueue } from '@sdkwork/kernel-ui-permissions';
import { createKernelUiClient, createMockKernelUiClient } from '@sdkwork/kernel-ui-services';
import type { KernelUiClient } from '@sdkwork/kernel-ui-types';
import { TerminalKernelPanel } from '@sdkwork/kernel-ui-terminal';
import { TelemetryEventStream } from '@sdkwork/kernel-ui-telemetry';
import type { KernelUiSnapshot, PermissionDecisionValue } from '@sdkwork/kernel-ui-types';
import { WorkspaceKernelPanel } from '@sdkwork/kernel-ui-workspace';
import './styles.css';

function createClient(): KernelUiClient {
  const apiUrl = import.meta.env.VITE_KERNEL_API_URL as string | undefined;
  if (apiUrl) {
    return createKernelUiClient({ baseUrl: apiUrl });
  }
  return createMockKernelUiClient();
}

export function App() {
  const runtime = useMemo(() => createKernelUiRuntime(createClient()), []);
  const [snapshot, setSnapshot] = useState<KernelUiSnapshot | null>(null);

  useEffect(() => {
    void runtime.loadSnapshot().then(setSnapshot);
  }, [runtime]);

  if (!snapshot) {
    return <main className="kernel-ui-shell">Loading kernel UI</main>;
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
          <h1>Agent And Code Kernel UI</h1>
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
