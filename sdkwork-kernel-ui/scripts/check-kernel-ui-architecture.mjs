import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = resolveKernelUiRoot();

const packagesRoot = path.join(root, 'packages');
const expectedPackages = [
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

const layeredPackageRules = {
  'sdkwork-kernel-ui-agent': ['components', 'service', 'hooks', 'types'],
  'sdkwork-kernel-ui-code': ['components', 'service', 'hooks', 'types'],
  'sdkwork-kernel-ui-workspace': ['components', 'service', 'hooks', 'types'],
  'sdkwork-kernel-ui-terminal': ['components', 'service', 'hooks', 'types'],
  'sdkwork-kernel-ui-telemetry': ['components', 'service', 'hooks', 'types'],
  'sdkwork-kernel-ui-permissions': ['components', 'service', 'hooks', 'types'],
  'sdkwork-kernel-ui-commons': ['components', 'types'],
  'sdkwork-kernel-ui-core': ['runtime', 'types'],
  'sdkwork-kernel-ui-services': ['service']
};

const errors = [];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function resolveKernelUiRoot() {
  const candidates = [
    process.cwd(),
    path.join(process.cwd(), 'sdkwork-kernel-ui'),
    path.resolve(scriptDir, '..')
  ];
  const visited = new Set();

  for (const candidate of candidates) {
    const resolved = path.resolve(candidate);
    if (visited.has(resolved)) {
      continue;
    }
    visited.add(resolved);

    if (
      fs.existsSync(path.join(resolved, 'package.json')) &&
      fs.existsSync(path.join(resolved, 'pnpm-workspace.yaml')) &&
      fs.existsSync(path.join(resolved, 'packages'))
    ) {
      return resolved;
    }
  }

  console.error('Unable to resolve sdkwork-kernel-ui root. Run from sdkwork-kernel or sdkwork-kernel-ui.');
  process.exit(1);
}

for (const packageDir of expectedPackages) {
  const packageRoot = path.join(packagesRoot, packageDir);
  const packageJsonPath = path.join(packageRoot, 'package.json');
  const srcDir = path.join(packageRoot, 'src');

  if (!fs.existsSync(packageJsonPath)) {
    errors.push(`missing package.json for ${packageDir}`);
    continue;
  }

  if (!fs.existsSync(srcDir)) {
    errors.push(`missing src directory for ${packageDir}`);
  }

  const packageJson = readJson(packageJsonPath);
  const expectedName = `@sdkwork/${packageDir.replace('sdkwork-', '').replace('kernel-ui-', 'kernel-ui-')}`;

  if (packageJson.name !== expectedName) {
    errors.push(`${packageDir} package name must be ${expectedName}`);
  }

  const indexTs = path.join(srcDir, 'index.ts');
  const indexTsx = path.join(srcDir, 'index.tsx');
  if (!fs.existsSync(indexTs) && !fs.existsSync(indexTsx)) {
    errors.push(`${packageDir} must export through src/index.ts or src/index.tsx`);
  }

  for (const requiredDir of layeredPackageRules[packageDir] ?? []) {
    const requiredPath = path.join(srcDir, requiredDir);
    if (!fs.existsSync(requiredPath) || !fs.statSync(requiredPath).isDirectory()) {
      errors.push(`${packageDir} must have src/${requiredDir}/ for kernel UI layering`);
    }
  }

  for (const field of ['dependencies', 'devDependencies']) {
    for (const [dependencyName, version] of Object.entries(packageJson[field] ?? {})) {
      if (dependencyName.startsWith('@sdkwork/kernel-ui-') && version !== 'workspace:*') {
        errors.push(`${packageDir} internal dependency ${dependencyName} must use workspace:*`);
      }
    }
  }
}

for (const filePath of listSourceFiles(path.join(root, 'src')).concat(listSourceFiles(packagesRoot))) {
  const contents = fs.readFileSync(filePath, 'utf8');
  const deepImport = contents.match(/from\s+['"](@sdkwork\/kernel-ui-[^'"]+\/[^'"]+)['"]/);
  if (deepImport) {
    errors.push(`${path.relative(root, filePath)} uses forbidden deep import: ${deepImport[1]}`);
  }
}

const rootPackage = readJson(path.join(root, 'package.json'));
for (const [dependencyName, version] of Object.entries(rootPackage.dependencies ?? {})) {
  if (dependencyName.startsWith('@sdkwork/kernel-ui-') && version !== 'workspace:*') {
    errors.push(`root internal dependency ${dependencyName} must use workspace:*`);
  }
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log(`Kernel UI architecture check passed for ${expectedPackages.length} packages.`);

function listSourceFiles(dir) {
  if (!fs.existsSync(dir)) {
    return [];
  }

  const entries = fs.readdirSync(dir, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const entryPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist') {
        return [];
      }
      return listSourceFiles(entryPath);
    }

    return /\.(ts|tsx)$/.test(entry.name) ? [entryPath] : [];
  });
}
