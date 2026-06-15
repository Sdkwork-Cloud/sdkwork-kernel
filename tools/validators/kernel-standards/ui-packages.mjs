import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

const requiredUiPackages = [
  'sdkwork-kernel-ui-types',
  'sdkwork-kernel-ui-core',
  'sdkwork-kernel-ui-services',
  'sdkwork-kernel-ui-commons',
  'sdkwork-kernel-ui-agent',
  'sdkwork-kernel-ui-code',
  'sdkwork-kernel-ui-workspace',
  'sdkwork-kernel-ui-terminal',
  'sdkwork-kernel-ui-telemetry',
  'sdkwork-kernel-ui-permissions'
];

export function validateKernelUiPackages({ kernelRoot, errors, ensureFile, readJson }) {
  const kernelUiArchitectureCheck = spawnSync(
    process.execPath,
    [path.join(kernelRoot, 'sdkwork-kernel-ui', 'scripts', 'check-kernel-ui-architecture.mjs')],
    {
      cwd: kernelRoot,
      encoding: 'utf8'
    }
  );
  if (kernelUiArchitectureCheck.status !== 0) {
    errors.push(
      `kernel UI architecture check failed:\n${kernelUiArchitectureCheck.stdout}${kernelUiArchitectureCheck.stderr}`
    );
  }

  const uiRoot = path.join(kernelRoot, 'sdkwork-kernel-ui');
  ensureFile(path.join('sdkwork-kernel-ui', 'package.json'));
  ensureFile(path.join('sdkwork-kernel-ui', 'pnpm-workspace.yaml'));
  ensureFile(path.join('sdkwork-kernel-ui', 'README.md'));

  for (const packageDir of requiredUiPackages) {
    const packageRoot = path.join(uiRoot, 'packages', packageDir);
    const packageJsonPath = path.join(packageRoot, 'package.json');
    const srcDir = path.join(packageRoot, 'src');

    if (!fs.existsSync(packageJsonPath)) {
      errors.push(`missing kernel UI package manifest: ${packageDir}`);
      continue;
    }

    const packageJson = readJson(path.relative(kernelRoot, packageJsonPath));
    if (!packageJson) {
      continue;
    }

    const expectedName = `@sdkwork/${packageDir.replace('sdkwork-', '')}`;
    if (packageJson.name !== expectedName) {
      errors.push(`${packageDir} package name must be ${expectedName}`);
    }

    if (!fs.existsSync(path.join(srcDir, 'index.ts')) && !fs.existsSync(path.join(srcDir, 'index.tsx'))) {
      errors.push(`${packageDir} must expose src/index.ts or src/index.tsx`);
    }
  }
}
