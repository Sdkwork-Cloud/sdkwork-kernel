#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const kernelRoot = process.cwd();

const requiredArtifacts = [
  '.github/workflows/kernel-verification.yml',
  'sdkwork-agent-business/specs/AGENT_BUSINESS_HTTP_TRUST_BOUNDARY.md',
  'sdkwork-agent-server/specs/AGENT_SERVER_HTTP_SURFACE.md',
  'sdkwork-kernel-ui/src/KernelUiSessionPanel.tsx',
  'sdkwork-kernel-ui/src/kernel-ui-client.ts',
  'scripts/verify-kernel-audit-remediation.mjs'
];

for (const artifact of requiredArtifacts) {
  const artifactPath = path.join(kernelRoot, artifact);
  if (!fs.existsSync(artifactPath)) {
    console.error(`MISSING audit remediation artifact: ${artifact}`);
    process.exit(1);
  }
}

const commands = [
  ['node', ['scripts/check-kernel-standards.mjs']],
  ['node', ['scripts/check-agent-sdk-workspace.mjs']],
  ['node', ['sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs']],
  ['node', ['--test', 'scripts/dev/sdkwork-kernel-utils-standard.test.mjs']],
  ['node', ['scripts/sdk-backend-workers/engine-sdk-live.test.mjs']],
  ['node', ['--test', 'tests/kernel_workspace_structure.test.mjs']],
  ['node', ['--test', 'tests/kernel_topology_alignment.test.mjs']],
  ['node', ['--test', 'tests/kernel_ui_server_api_alignment.test.mjs']],
  ['node', ['--test', 'scripts/dev/sdkwork-kernel-topology-baggage.test.mjs']],
  [
    'node',
    [
      '--test',
      'sdkwork-kernel-plugins/tests/kernel_plugin_structure.test.mjs',
      'sdkwork-kernel-ui/tests/kernel-ui-services.contract.test.mjs'
    ]
  ],
  ['cargo', ['test', '--manifest-path', 'sdkwork-agent-kernel/Cargo.toml', '-q']],
  ['cargo', ['test', '--manifest-path', 'sdkwork-agent-server/Cargo.toml', '-q']],
  [
    'cargo',
    [
      'test',
      '--test',
      'http_kernel_contracts',
      '--manifest-path',
      'sdkwork-agent-server/Cargo.toml',
      '-q'
    ]
  ],
  ['cargo', ['test', '--manifest-path', 'sdkwork-agent-client/Cargo.toml', '-q']],
  ['cargo', ['test', '--doc', '--manifest-path', 'sdkwork-agent-kernel/Cargo.toml', '-q']],
  ['cargo', ['test', '--manifest-path', 'sdkwork-code-kernel/Cargo.toml', '-q']],
  ['cargo', ['test', '--manifest-path', 'sdkwork-agent-business/Cargo.toml', '-q']],
  [
    'cargo',
    [
      'test',
      '--features',
      'http-axum',
      '--test',
      'http_axum_contracts',
      '--manifest-path',
      'sdkwork-agent-business/Cargo.toml',
      '-q'
    ]
  ],
  [
    'cargo',
    [
      'test',
      '--features',
      'postgres-sync',
      '--test',
      'agent_postgres_sync_contracts',
      '--manifest-path',
      'sdkwork-agent-business/Cargo.toml',
      '-q'
    ]
  ],
  [
    'cargo',
    [
      'test',
      '--features',
      'http-axum,postgres-sync',
      '--manifest-path',
      'sdkwork-agent-business/Cargo.toml',
      '-q'
    ]
  ],
  ['pnpm', ['--dir', 'sdkwork-kernel-ui', 'typecheck']]
];

const postgresUri = process.env.SDKWORK_AGENT_BUSINESS_POSTGRES_URI;
if (postgresUri) {
  commands.push([
    'cargo',
    [
      'test',
      '--features',
      'postgres-sync',
      '--test',
      'agent_postgres_sync_contracts',
      '--manifest-path',
      'sdkwork-agent-business/Cargo.toml',
      '-q',
      'live_postgres_memory_relation_get_roundtrip_when_uri_configured',
      '--',
      '--nocapture'
    ]
  ]);
} else {
  console.log(
    'SKIP: live PostgreSQL contract (set SDKWORK_AGENT_BUSINESS_POSTGRES_URI to enable locally; CI postgres-live job covers this).'
  );
}

let failed = 0;

for (const [cmd, args] of commands) {
  const label = `${cmd} ${args.join(' ')}`;
  const result = spawnSync(cmd, args, {
    cwd: kernelRoot,
    stdio: 'inherit',
    shell: process.platform === 'win32'
  });
  if (result.status !== 0) {
    console.error(`FAILED: ${label}`);
    failed += 1;
  } else {
    console.log(`PASSED: ${label}`);
  }
}

if (failed > 0) {
  console.error(`Kernel audit verification failed (${failed} command groups).`);
  process.exit(1);
}

console.log('Kernel audit verification passed.');
