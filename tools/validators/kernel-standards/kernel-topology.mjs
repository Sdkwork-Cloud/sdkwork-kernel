import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

export function validateKernelTopology({ kernelRoot, errors, readJson }) {
  const requiredPaths = [
    'specs/topology.spec.json',
    'scripts/lib/kernel-topology.mjs',
    'scripts/kernel-dev.mjs',
    'docs/topology-standard.md',
    'package.json',
  ];

  for (const relativePath of requiredPaths) {
    const filePath = path.join(kernelRoot, relativePath);
    if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
      errors.push(`missing topology adoption file: ${relativePath}`);
    }
  }

  const packageJsonPath = path.join(kernelRoot, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  const packageJson = readJson('package.json');
  if (
    packageJson?.dependencies?.['@sdkwork/app-topology'] !== 'file:../sdkwork-app-topology' &&
    packageJson?.dependencies?.['@sdkwork/app-topology'] !== 'workspace:*'
  ) {
    errors.push(
      'package.json must depend on @sdkwork/app-topology via file:../sdkwork-app-topology or workspace:*'
    );
  }

  const spec = readJson('specs/topology.spec.json');
  if (!spec || spec.schemaVersion !== 4 || spec.kind !== 'sdkwork.app.topology') {
    errors.push('specs/topology.spec.json must declare schemaVersion 4 sdkwork.app.topology');
    return;
  }

  for (const profilePath of Object.values(spec.profileFiles ?? {})) {
    const resolvedPath = path.join(kernelRoot, profilePath);
    if (!fs.existsSync(resolvedPath) || !fs.statSync(resolvedPath).isFile()) {
      errors.push(`missing topology profile env: ${profilePath}`);
    }
  }

  const validateScript = path.resolve(kernelRoot, '..', 'sdkwork-app-topology', 'scripts', 'sdkwork-topology.mjs');
  if (!fs.existsSync(validateScript)) {
    errors.push('missing ../sdkwork-app-topology/scripts/sdkwork-topology.mjs');
    return;
  }

  const validation = spawnSync(
    process.execPath,
    [validateScript, 'validate', '--root', kernelRoot, '--spec', 'specs/topology.spec.json'],
    {
      cwd: kernelRoot,
      encoding: 'utf8',
    },
  );

  if (validation.status !== 0) {
    errors.push(
      `topology validate failed:\n${validation.stdout}${validation.stderr}`,
    );
  }
}
