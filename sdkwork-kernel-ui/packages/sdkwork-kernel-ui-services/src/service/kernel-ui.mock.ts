import type { KernelUiSnapshot } from '@sdkwork/kernel-ui-types';

export const kernelUiMockSnapshot: KernelUiSnapshot = {
  runtime: {
    runtimeId: 'runtime.local',
    agentId: 'agent.intelligence.general',
    kernelVersion: '0.1.0',
    state: 'ready',
    health: 'healthy',
    capabilities: [
      {
        capabilityId: 'model.chat',
        providerId: 'provider.model.fake',
        status: 'available',
        required: true
      },
      {
        capabilityId: 'policy.evaluate',
        providerId: 'provider.policy.fake',
        status: 'available',
        required: true
      },
      {
        capabilityId: 'code.patch.apply',
        providerId: 'provider.patch.fake',
        status: 'available',
        required: false
      }
    ],
    missingRequiredCapabilities: [],
    degradedCapabilities: []
  },
  events: [
    {
      eventId: 'event.1',
      eventType: 'agent.runtime.ready',
      severity: 'info',
      summary: 'runtime_id=runtime.local',
      sequence: 1,
      traceId: 'trace.kernel.1'
    },
    {
      eventId: 'event.2',
      eventType: 'code.patch.validated',
      severity: 'info',
      summary: 'patch_id=patch.1',
      sequence: 2,
      traceId: 'trace.kernel.1'
    },
    {
      eventId: 'event.3',
      eventType: 'agent.policy.approval_requested',
      severity: 'warn',
      summary: 'category=code.terminal.run',
      sequence: 3,
      traceId: 'trace.kernel.1'
    }
  ],
  permissions: [
    {
      permissionRequestId: 'permission.1',
      category: 'code.terminal.run',
      resource: 'cargo test',
      sideEffectLevel: 'side_effectful',
      reason: 'verification command',
      status: 'pending'
    }
  ],
  workspace: {
    workspaceId: 'workspace.1',
    root: 'D:/repo',
    branch: 'main',
    dirty: true,
    changedFiles: ['src/lib.rs', 'tests/kernel_contracts.rs']
  },
  patches: [
    {
      patchId: 'patch.1',
      summary: 'Add host provider SPI',
      status: 'validated',
      changedFiles: ['src/host.rs', 'tests/host_provider_contracts.rs'],
      requiresPolicy: true
    }
  ],
  verificationReports: [
    {
      reportId: 'verification.1',
      status: 'passed',
      command: 'cargo test',
      failures: [],
      evidence: '40 tests passed'
    }
  ],
  terminalCommands: [
    {
      commandId: 'command.1',
      command: 'cargo test',
      workingDirectory: 'D:/repo',
      status: 'passed',
      exitCode: 0,
      durationMs: 1240,
      requiresPolicy: true
    }
  ],
  terminalOutput: [
    {
      commandId: 'command.1',
      sequence: 1,
      channel: 'stdout',
      content: 'running 40 tests',
      redacted: false
    },
    {
      commandId: 'command.1',
      sequence: 2,
      channel: 'stdout',
      content: 'test result: ok',
      redacted: false
    }
  ],
  reviewFindings: [
    {
      findingId: 'finding.1',
      severity: 'medium',
      filePath: 'src/manifest.rs',
      line: 21,
      message: 'Manifest parsing still needs schema-backed validation',
      missingTest: 'escaped JSON string parsing'
    }
  ]
};
