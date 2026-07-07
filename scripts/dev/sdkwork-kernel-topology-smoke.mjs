#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import process from 'node:process';

import {
  loadProfile,
  mergeRuntimeEnv,
  resolveSurfaceHttpUrl,
  waitForHttpHealthy,
} from '../lib/kernel-topology.mjs';

const PROFILE_ID = 'standalone.unified-process.development';
const HEALTH_PATH = '/healthz';
const INTERNAL_RUNTIME_SNAPSHOT_PATH = '/internal/v3/api/intelligence/runtime/snapshot';
const STARTUP_TIMEOUT_MS = 120_000;

async function probeHttpHealthy(url, options) {
  try {
    return await waitForHttpHealthy(url, options);
  } catch {
    return false;
  }
}

function cargoCommand() {
  return process.platform === 'win32' ? 'cargo.exe' : 'cargo';
}

function terminateProcessTree(child) {
  if (!child?.pid) {
    return;
  }
  if (process.platform === 'win32') {
    spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], {
      stdio: 'ignore',
      windowsHide: true,
    });
    return;
  }
  child.kill('SIGTERM');
}

async function main() {
  const profileEnv = loadProfile(PROFILE_ID);
  const runtimeEnv = mergeRuntimeEnv(process.env, profileEnv, {
    SDKWORK_KERNEL_PROFILE_ID: PROFILE_ID,
  });
  const healthUrl = resolveSurfaceHttpUrl(runtimeEnv, 'application.public-ingress');
  if (!healthUrl) {
    throw new Error('application.public-ingress URL missing from profile env');
  }

  const child = spawn(
    cargoCommand(),
    ['run', '-q', '-p', 'sdkwork-agent-server', '--bin', 'sdkwork-agent-server'],
    {
      env: runtimeEnv,
      stdio: 'ignore',
      shell: false,
      windowsHide: true,
    },
  );

  let exitCode = null;
  child.on('exit', (code) => {
    exitCode = code;
  });

  try {
    const startedAt = Date.now();
    let ready = false;
    while (Date.now() - startedAt < STARTUP_TIMEOUT_MS) {
      if (exitCode != null) {
        throw new Error(`sdkwork-agent-server exited before health check (code=${exitCode})`);
      }
      ready = await probeHttpHealthy(healthUrl, {
        path: HEALTH_PATH,
        timeoutMs: 1500,
        attempts: 1,
        intervalMs: 250,
      });
      if (ready) {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }

    if (!ready) {
      throw new Error(`timed out waiting for ${healthUrl}${HEALTH_PATH}`);
    }

    async function probeSnapshot(path) {
      const snapshotUrl = `${healthUrl}${path}`;
      const snapshotResponse = await fetch(snapshotUrl, {
        signal: AbortSignal.timeout(1500),
      });
      if (!snapshotResponse.ok) {
        throw new Error(`kernel snapshot probe failed for ${path}: ${snapshotResponse.status}`);
      }
      const payload = await snapshotResponse.json();
      const snapshot = payload?.data?.item ?? payload;
      if (!snapshot?.runtime?.health) {
        throw new Error(`kernel snapshot response missing runtime.health for ${path}`);
      }
      return snapshot;
    }

    const snapshot = await probeSnapshot(INTERNAL_RUNTIME_SNAPSHOT_PATH);

    console.log(
      `[sdkwork-kernel-topology-smoke] ok profile=${PROFILE_ID} url=${healthUrl}${HEALTH_PATH} internal=${INTERNAL_RUNTIME_SNAPSHOT_PATH} snapshot=${snapshot.runtime.health}`,
    );
  } finally {
    terminateProcessTree(child);
  }
}

main().catch((error) => {
  console.error(`[sdkwork-kernel-topology-smoke] ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
