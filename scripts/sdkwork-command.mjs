#!/usr/bin/env node
/**
 * SDKWork Kernel standard command dispatcher.
 *
 * Maps PNPM_SCRIPT_SPEC.md public commands to kernel implementation scripts.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');

const ALLOWED_DEPLOYMENT_PROFILES = new Set(['standalone', 'cloud']);
const ALLOWED_ENVIRONMENTS = new Set(['development', 'test', 'staging', 'production']);
const ALLOWED_RUNTIME_TARGETS = new Set([
  'browser',
  'desktop',
  'server',
  'container',
  'tablet-ipados',
  'tablet-android',
  'capacitor-ios',
  'capacitor-android',
  'flutter-ios',
  'flutter-android',
  'android-native',
  'ios-native',
  'harmony-native',
  'mini-program',
  'test-runner',
]);

const RETIRED_VALUES = new Set(['self-hosted', 'cloud-hosted', 'hosting', 'web', 'mobile', 'native', 'docker']);
const RETIRED_FLAGS = new Set(['service-layout', 'database']);

function parseArgs(argv) {
  const args = { command: null, flags: {} };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const value = argv[index + 1] && !argv[index + 1].startsWith('--') ? argv[++index] : true;
      args.flags[key] = value;
    } else if (!args.command) {
      args.command = arg;
    }
  }
  return args;
}

function validateAxisValues(flags) {
  const {
    'deployment-profile': deploymentProfile,
    environment,
    'runtime-target': runtimeTarget,
  } = flags;

  for (const flagName of Object.keys(flags)) {
    if (RETIRED_FLAGS.has(flagName)) {
      if (flagName === 'database') {
        console.error('[sdkwork-kernel] Retired flag --database. Server runtime always uses the SDKWORK_DATABASE_* PostgreSQL profile.');
        process.exit(1);
      }
      console.error(`[sdkwork-kernel] Retired flag --${flagName}. Use --deployment-profile and --environment.`);
      process.exit(1);
    }
  }
  if (deploymentProfile && !ALLOWED_DEPLOYMENT_PROFILES.has(deploymentProfile)) {
    console.error(`[sdkwork-kernel] Invalid deployment-profile: ${deploymentProfile}`);
    process.exit(1);
  }
  if (environment && !ALLOWED_ENVIRONMENTS.has(environment)) {
    console.error(`[sdkwork-kernel] Invalid environment: ${environment}`);
    process.exit(1);
  }
  if (runtimeTarget && !ALLOWED_RUNTIME_TARGETS.has(runtimeTarget)) {
    console.error(`[sdkwork-kernel] Invalid runtime-target: ${runtimeTarget}`);
    process.exit(1);
  }

  for (const [key, value] of Object.entries(flags)) {
    if (typeof value === 'string' && RETIRED_VALUES.has(value)) {
      console.error(`[sdkwork-kernel] Retired value '${value}' for --${key}. Use standard axis values only.`);
      process.exit(1);
    }
  }
}

function runStep(command, args, cwd = repoRoot) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if ((result.status ?? 1) !== 0) {
    process.exit(result.status ?? 1);
  }
}

function runNodeScript(relativePath, scriptArgs = []) {
  runStep(process.execPath, [path.join(repoRoot, relativePath), ...scriptArgs]);
}

function printUsage() {
  console.error(`Usage: node scripts/sdkwork-command.mjs <command> [flags]

Commands:
  dev      Start topology-aware kernel dev stack (agent server)
  build    Build Rust workspace and agent internal SDK
  test     Run default repository test subset
  check    Run static standards and policy checks
  verify   Run merge-ready verification aggregate
  clean    Remove reproducible build artifacts

Dev flags:
  --runtime-target <server|browser|...>   Default: server
  --deployment-profile <standalone|cloud>            Default: standalone
  --environment <development|test|staging|production> Default: development
  --dry-run
`);
}

function dispatch({ command, flags }) {
  if (!command) {
    printUsage();
    process.exit(1);
  }

  validateAxisValues(flags);

  const runtimeTarget = flags['runtime-target'] || 'server';
  const deploymentProfile = flags['deployment-profile'] || 'standalone';
  const environment = flags.environment || 'development';

  switch (command) {
    case 'dev': {
      if (runtimeTarget !== 'server') {
        console.error('[sdkwork-kernel] Only --runtime-target server is supported for kernel dev today.');
        process.exit(1);
      }
      const devArgs = [
        '--deployment-profile',
        deploymentProfile,
        '--environment',
        environment,
      ];
      if (flags['dry-run']) {
        devArgs.push('--dry-run');
      }
      runNodeScript('scripts/kernel-dev.mjs', devArgs);
      break;
    }
    case 'build': {
      runStep('cargo', ['build', '--workspace']);
      runStep('pnpm', [
        '--dir',
        'sdks/sdkwork-agent-internal-sdk/sdkwork-agent-internal-sdk-typescript',
        'verify',
      ]);
      break;
    }
    case 'test': {
      runStep('pnpm', ['topology:validate']);
      runStep(process.execPath, ['--test', 'tests/kernel_workspace_structure.test.mjs']);
      runStep(process.execPath, ['--test', 'tests/kernel_topology_alignment.test.mjs']);
      runStep(process.execPath, ['--test', 'scripts/dev/sdkwork-kernel-topology-baggage.test.mjs']);
      runStep(process.execPath, ['--test', 'scripts/dev/sdkwork-kernel-utils-standard.test.mjs']);
      break;
    }
    case 'check': {
      runNodeScript('scripts/check-kernel-standards.mjs');
      runNodeScript('scripts/check-agent-sdk-workspace.mjs');
      runStep('pnpm', ['check:pnpm-script-standard']);
      runStep('pnpm', ['topology:validate']);
      break;
    }
    case 'verify': {
      runNodeScript('scripts/verify-kernel-audit-remediation.mjs');
      break;
    }
    case 'clean': {
      runNodeScript('scripts/clean-workspace.mjs');
      break;
    }
    default: {
      console.error(`[sdkwork-kernel] Unknown standard command: ${command}`);
      printUsage();
      process.exit(1);
    }
  }
}

const parsed = parseArgs(process.argv.slice(2));
if (parsed.command === '--help' || parsed.command === '-h' || process.argv.includes('--help') || process.argv.includes('-h')) {
  printUsage();
  process.exit(0);
}

dispatch(parsed);
