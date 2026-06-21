import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const REQUIRED_ROOT_SCRIPTS = ['dev', 'build', 'test', 'check', 'verify', 'clean'];
const REQUIRED_CAPABILITY_SCRIPTS = ['topology:validate', 'api:check', 'sdk:check', 'check:pnpm-script-standard'];

/**
 * PNPM_SCRIPT_SPEC alignment for the kernel standards repository root.
 * Scans kernel-owned surfaces only; external/ reference trees are out of scope.
 */
export function validatePlatformPnpmScripts({ kernelRoot, errors }) {
  const packagePath = path.join(kernelRoot, 'package.json');
  if (!fs.existsSync(packagePath)) {
    errors.push('missing root package.json for PNPM script standard');
    return;
  }

  const manifest = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
  const scripts = manifest.scripts ?? {};

  for (const scriptName of REQUIRED_ROOT_SCRIPTS) {
    if (!scripts[scriptName]) {
      errors.push(`package.json scripts must expose required root command "${scriptName}"`);
    }
  }

  for (const scriptName of REQUIRED_CAPABILITY_SCRIPTS) {
    if (!scripts[scriptName]) {
      errors.push(`package.json scripts must expose capability command "${scriptName}"`);
    }
  }

  if (!String(scripts.dev ?? '').includes('sdkwork-command.mjs')) {
    errors.push('package.json scripts.dev must call scripts/sdkwork-command.mjs');
  }

  for (const relativePath of ['scripts/sdkwork-command.mjs', 'scripts/clean-workspace.mjs', 'scripts/check-pnpm-script-standard.mjs']) {
    if (!fs.existsSync(path.join(kernelRoot, relativePath))) {
      errors.push(`${relativePath} must exist for PNPM script standard`);
    }
  }

  const pnpmStandardCheck = spawnSync(
    process.execPath,
    [path.join(kernelRoot, 'scripts/check-pnpm-script-standard.mjs')],
    {
      cwd: kernelRoot,
      encoding: 'utf8',
    },
  );

  if (pnpmStandardCheck.status !== 0) {
    errors.push(
      `kernel pnpm script standard check failed:\n${pnpmStandardCheck.stdout}${pnpmStandardCheck.stderr}`,
    );
  }
}
