#!/usr/bin/env node
/**
 * Opt-in Hermes Agent TUI gateway staging proof.
 *
 * This complements engine-sdk-live-staging.mjs, which covers Node/gateway SDK
 * providers. Hermes uses a Python/TUI JSON-RPC path, so commercial release
 * verification must prove that path separately instead of treating
 * --framework all as Hermes coverage.
 */

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import readline from 'node:readline';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const kernelRoot = path.resolve(scriptDir, '../..');

const STAGING_HERMES_GATEWAY_ENV = 'SDKWORK_KERNEL_STAGING_HERMES_GATEWAY';
const HERMES_SOURCE_ROOT_ENV = 'SDKWORK_HERMES_AGENT_SOURCE_ROOT';
const HERMES_PYTHON_ENV = 'SDKWORK_HERMES_PYTHON';
const DEFAULT_TIMEOUT_MS = 20_000;

function truthy(value) {
  return ['1', 'true', 'yes', 'on'].includes(String(value ?? '').trim().toLowerCase());
}

export function hermesGatewayStagingEnabled(env = process.env) {
  return truthy(env[STAGING_HERMES_GATEWAY_ENV]);
}

function defaultHermesRoot(env = process.env) {
  const configured = env[HERMES_SOURCE_ROOT_ENV]?.trim();
  if (configured) {
    return path.resolve(configured);
  }
  return path.join(kernelRoot, 'external', 'hermes-agent');
}

function assertHermesGatewaySource(hermesRoot) {
  const entryPath = path.join(hermesRoot, 'tui_gateway', 'entry.py');
  if (!fs.existsSync(entryPath)) {
    throw new Error(
      `missing Hermes Agent gateway source: ${entryPath}; set ${HERMES_SOURCE_ROOT_ENV} to a checkout containing tui_gateway/entry.py`,
    );
  }
}

function withPrependedPath(existing, nextPath) {
  if (!existing?.trim()) {
    return nextPath;
  }
  return `${nextPath}${path.delimiter}${existing}`;
}

export function buildHermesGatewaySpawnOptions(options = {}) {
  const env = options.env ?? process.env;
  const hermesRoot = path.resolve(options.hermesRoot ?? defaultHermesRoot(env));
  assertHermesGatewaySource(hermesRoot);

  const hermesHome =
    options.hermesHome ?? fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-hermes-staging-'));
  const command = env[HERMES_PYTHON_ENV]?.trim() || 'python';
  const gatewayEnv = {
    ...process.env,
    ...env,
    HERMES_HOME: hermesHome,
    HERMES_TUI_GATEWAY_SHUTDOWN_GRACE_S: '0.25',
    PYTHONUNBUFFERED: '1',
    PYTHONPATH: withPrependedPath(env.PYTHONPATH ?? process.env.PYTHONPATH, hermesRoot),
  };

  return {
    command,
    args: ['-u', '-m', 'tui_gateway.entry'],
    options: {
      cwd: hermesRoot,
      env: gatewayEnv,
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    },
  };
}

function formatBufferedLines(lines) {
  return lines.slice(-40).join('');
}

function runGatewayJsonRpcProbe(spawnOptions, timeoutMs) {
  return new Promise((resolve, reject) => {
    const child = spawn(spawnOptions.command, spawnOptions.args, spawnOptions.options);
    const stderr = [];
    let readySeen = false;
    let done = false;

    const timer = setTimeout(() => {
      finish(
        new Error(
          `Hermes staging gateway proof timed out after ${timeoutMs}ms; stderr: ${formatBufferedLines(stderr)}`,
        ),
      );
    }, timeoutMs);

    function cleanup() {
      clearTimeout(timer);
      child.stdin.destroy();
      child.stdout.destroy();
      child.stderr.destroy();
      if (!child.killed) {
        child.kill();
      }
    }

    function finish(error, result) {
      if (done) {
        return;
      }
      done = true;
      cleanup();
      if (error) {
        reject(error);
      } else {
        resolve(result);
      }
    }

    child.stderr.on('data', (chunk) => {
      stderr.push(String(chunk));
      if (stderr.length > 80) {
        stderr.splice(0, stderr.length - 80);
      }
    });
    child.once('error', (error) => finish(error));
    child.once('exit', (code, signal) => {
      if (!done) {
        finish(
          new Error(
            `Hermes staging gateway exited before proof completed (code=${code}, signal=${signal}); stderr: ${formatBufferedLines(stderr)}`,
          ),
        );
      }
    });

    const lines = readline.createInterface({ input: child.stdout });
    lines.on('line', (line) => {
      let frame;
      try {
        frame = JSON.parse(line);
      } catch (error) {
        finish(new Error(`Hermes staging gateway emitted non-JSON stdout: ${line}`));
        return;
      }

      if (frame.method === 'event' && frame.params?.type === 'gateway.ready') {
        readySeen = true;
        child.stdin.write(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: 'sdkwork-hermes-profile',
            method: 'config.get',
            params: { key: 'profile' },
          })}\n`,
        );
        return;
      }

      if (frame.id !== 'sdkwork-hermes-profile') {
        return;
      }

      if (!readySeen) {
        finish(new Error('Hermes staging gateway responded before gateway.ready event'));
        return;
      }
      if (frame.error) {
        finish(
          new Error(
            `Hermes staging gateway config.get failed: ${frame.error.message ?? JSON.stringify(frame.error)}`,
          ),
        );
        return;
      }
      if (!frame.result?.home || !frame.result?.display) {
        finish(new Error(`Hermes staging gateway returned invalid profile result: ${JSON.stringify(frame)}`));
        return;
      }

      finish(null, {
        ready: true,
        profileHome: frame.result.home,
        profileDisplay: frame.result.display,
      });
    });
  });
}

export async function runHermesGatewayStagingProof(options = {}) {
  const env = options.env ?? process.env;
  if (!hermesGatewayStagingEnabled(env)) {
    console.log(`[skip] ${STAGING_HERMES_GATEWAY_ENV} is not enabled; Hermes gateway staging proof skipped intentionally`);
    return { status: 'skipped', reason: 'flag-disabled' };
  }

  const spawnOptions = buildHermesGatewaySpawnOptions(options);
  const result = await runGatewayJsonRpcProbe(
    spawnOptions,
    options.timeoutMs ?? DEFAULT_TIMEOUT_MS,
  );
  return {
    status: 'passed',
    gateway: 'tui_gateway.entry',
    result,
  };
}

if (process.argv[1]?.replace(/\\/g, '/').endsWith('hermes-gateway-staging.mjs')) {
  runHermesGatewayStagingProof()
    .then((report) => {
      console.log(`hermes-gateway-staging finished: ${report.status}`);
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
