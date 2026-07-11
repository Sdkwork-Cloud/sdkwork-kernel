#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

import {
  API_GATEWAY_REPO,
  bridgeLegacyServiceEnv,
  DEFAULT_DEV_PROFILE_ID,
  IAM_APPLICATION_BOOTSTRAP_ENV,
  listHealthSurfaces,
  listOrchestrationProcesses,
  loadProfile,
  mergeRuntimeEnv,
  REPO_ROOT,
  resolveDevProfileId,
  resolveGatewayBind,
  resolveSurfaceHttpUrl,
  shouldAutostartGateway,
  waitForHttpHealthy,
} from './lib/kernel-topology.mjs';

const HEALTH_PATH = '/healthz';
const HEALTH_TIMEOUT_MS = 2000;

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

function pnpmCommand() {
  return process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';
}

function parseArgs(argv) {
  const settings = {
    deploymentProfile: 'standalone',
    environment: 'development',
    dryRun: false,
    help: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--help' || arg === '-h') {
      settings.help = true;
      continue;
    }
    if (arg === '--deployment-profile') {
      settings.deploymentProfile = argv[index + 1] ?? settings.deploymentProfile;
      index += 1;
      continue;
    }
    if (arg === '--environment') {
      settings.environment = argv[index + 1] ?? settings.environment;
      index += 1;
      continue;
    }
    if (arg === '--service-layout') {
      throw new Error(
        '--service-layout is retired; use --deployment-profile and --environment',
      );
    }
    if (arg === '--hosting') {
      throw new Error(
        '--hosting is retired; use --deployment-profile (standalone | cloud)',
      );
    }
    if (arg === '--topology') {
      throw new Error(
        '--topology is retired; use --deployment-profile and --environment',
      );
    }
    if (arg === '--dry-run') {
      settings.dryRun = true;
    }
  }

  return settings;
}

function printHelp() {
  console.log(`Usage: node scripts/kernel-dev.mjs [options]

Topology-aware kernel dev entry. Loads configs/topology profile env via @sdkwork/app-topology.

Options:
  --deployment-profile <standalone|cloud>           Default: standalone
  --environment <development|test|staging|production> Default: development
  --dry-run                                         Print plan without executing
  --help, -h
`);
}

function spawnProcessEntry(entry) {
  return spawn(entry.command, entry.args, {
    cwd: entry.cwd ?? REPO_ROOT,
    env: entry.env,
    stdio: 'inherit',
    shell: false,
    windowsHide: true,
  });
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
  child.kill();
}

function createCargoProcess({ label, packageName, binary, env }) {
  const args = ['run', '-p', packageName];
  if (binary) {
    args.push('--bin', binary);
  }
  return {
    label,
    command: cargoCommand(),
    args,
    cwd: REPO_ROOT,
    env,
  };
}

function createPnpmProcess({ label, packageName, script, env }) {
  return {
    label,
    command: pnpmCommand(),
    args: ['--dir', packageName, script],
    cwd: REPO_ROOT,
    env,
  };
}

function createPlatformGatewayProcess(env) {
  const bind = resolveGatewayBind(
    env,
    env.SDKWORK_KERNEL_DEPLOYMENT_PROFILE ?? 'standalone',
  );
  return {
    label: 'sdkwork-api-cloud-gateway',
    command: cargoCommand(),
    args: [
      'run',
      '-p',
      'sdkwork-api-cloud-gateway',
      '--bin',
      'sdkwork-api-cloud-gateway',
    ],
    cwd: API_GATEWAY_REPO,
    env: {
      ...env,
      SDKWORK_API_CLOUD_GATEWAY_BIND: bind,
    },
  };
}

function buildProcessEntries(profileId, env) {
  const entries = [];
  if (shouldAutostartGateway(env)) {
    entries.push(createPlatformGatewayProcess(env));
  }

  for (const processSpec of listOrchestrationProcesses(profileId)) {
    if (processSpec.crate && processSpec.binary) {
      entries.push(
        createCargoProcess({
          label: processSpec.id,
          packageName: processSpec.crate,
          binary: processSpec.binary,
          env,
        }),
      );
      continue;
    }
    if (processSpec.package && processSpec.script) {
      entries.push(
        createPnpmProcess({
          label: processSpec.id,
          packageName: processSpec.package,
          script: processSpec.script,
          env,
        }),
      );
    }
  }

  return entries;
}

async function waitForSurfaceHealth(profileId, env) {
  for (const surfaceId of listHealthSurfaces(profileId)) {
    const url = resolveSurfaceHttpUrl(env, surfaceId);
    if (!url) {
      continue;
    }
    const ready = await probeHttpHealthy(url, {
      path: HEALTH_PATH,
      timeoutMs: HEALTH_TIMEOUT_MS,
    });
    if (!ready) {
      throw new Error(`timed out waiting for ${surfaceId} health at ${url}${HEALTH_PATH}`);
    }
    console.log(`[sdkwork-kernel] healthy ${surfaceId} (${url}${HEALTH_PATH})`);
  }
}

async function main() {
  const settings = parseArgs(process.argv.slice(2));
  if (settings.help) {
    printHelp();
    process.exit(0);
  }

  const profileId = resolveDevProfileId(settings.deploymentProfile, settings.environment)
    || DEFAULT_DEV_PROFILE_ID;
  const profileEnv = loadProfile(profileId);
  const runtimeEnv = mergeRuntimeEnv(process.env, profileEnv, bridgeLegacyServiceEnv(profileEnv), IAM_APPLICATION_BOOTSTRAP_ENV, {
    SDKWORK_KERNEL_PROFILE_ID: profileId,
  });
  const processes = buildProcessEntries(profileId, runtimeEnv);

  if (settings.dryRun) {
    console.log(`[sdkwork-kernel] profile=${profileId}`);
    for (const entry of processes) {
      console.log(`[${entry.label}] ${entry.command} ${entry.args.join(' ')}`);
    }
    process.exit(0);
  }

  const children = [];
  let shuttingDown = false;

  function shutdown(exceptChild) {
    if (shuttingDown) {
      return;
    }
    shuttingDown = true;
    for (const child of children) {
      if (child !== exceptChild && child.exitCode == null && child.signalCode == null) {
        terminateProcessTree(child);
      }
    }
  }

  function attachProcessLifecycle(entry, child) {
    child.on('error', (error) => {
      process.stderr.write(
        `[${entry.label}] ${error instanceof Error ? error.message : String(error)}\n`,
      );
      shutdown(child);
      process.exitCode = 1;
    });
    child.on('exit', (code, signal) => {
      if (shuttingDown) {
        return;
      }
      shutdown(child);
      if (code && code !== 0) {
        process.stderr.write(`[${entry.label}] exited with code ${code}\n`);
        process.exitCode = code;
        return;
      }
      if (signal) {
        process.stderr.write(`[${entry.label}] exited with signal ${signal}\n`);
        process.exitCode = 1;
      }
    });
  }

  for (const entry of processes) {
    const child = spawnProcessEntry(entry);
    children.push(child);
    attachProcessLifecycle(entry, child);
  }

  try {
    await waitForSurfaceHealth(profileId, runtimeEnv);
  } catch (error) {
    shutdown();
    throw error;
  }

  console.log(`[sdkwork-kernel] dev stack ready (profile=${profileId})`);
  const stop = () => shutdown();
  process.once('SIGINT', stop);
  process.once('SIGTERM', stop);
}

main().catch((error) => {
  console.error(`[sdkwork-kernel] ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
