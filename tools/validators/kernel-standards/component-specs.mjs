import fs from 'node:fs';
import path from 'node:path';

export function listComponentSpecFiles(scanPath) {
  if (!fs.existsSync(scanPath)) {
    return [];
  }

  const stat = fs.statSync(scanPath);
  if (stat.isFile()) {
    return path.basename(scanPath) === 'component.spec.json' ? [scanPath] : [];
  }

  return fs.readdirSync(scanPath, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = path.join(scanPath, entry.name);
    if (entry.isDirectory()) {
      if (
        entry.name === '.git' ||
        entry.name === 'node_modules' ||
        entry.name === 'dist' ||
        entry.name === 'generated' ||
        entry.name === 'target' ||
        entry.name === 'external'
      ) {
        return [];
      }
      return listComponentSpecFiles(entryPath);
    }

    return entry.name === 'component.spec.json' ? [entryPath] : [];
  });
}

export function validateComponentCanonicalSpecPaths(manifestPath, { kernelRoot, errors }) {
  const manifest = readJsonFile(manifestPath, kernelRoot, errors);
  if (!manifest) {
    return;
  }

  const componentRoot = resolveComponentRoot(manifestPath, manifest, kernelRoot);
  for (const canonicalSpec of manifest.canonicalSpecs ?? []) {
    if (!canonicalSpec.path) {
      errors.push(
        `${path.relative(kernelRoot, manifestPath)} canonical spec ${canonicalSpec.file ?? '<unknown>'} must declare a path`
      );
      continue;
    }

    const resolvedPath = path.resolve(componentRoot, canonicalSpec.path);
    if (!fs.existsSync(resolvedPath) || !fs.statSync(resolvedPath).isFile()) {
      errors.push(
        `${path.relative(kernelRoot, manifestPath)} canonical spec ${canonicalSpec.file ?? canonicalSpec.path} must resolve to ${path.relative(kernelRoot, resolvedPath)}`
      );
    }
  }
}

export function validateComponentSpecMetadata(manifestPath, { kernelRoot, errors }) {
  const manifest = readJsonFile(manifestPath, kernelRoot, errors);
  if (!manifest) {
    return;
  }

  const relativePath = path.relative(kernelRoot, manifestPath);
  const component = manifest.component ?? {};
  const contracts = manifest.contracts ?? {};
  const canonicalSpecFiles = new Set((manifest.canonicalSpecs ?? []).map((spec) => spec.file));
  const relativeParts = relativePath.split(path.sep);
  const isSdkArea = component.type === 'sdk-family' || relativeParts[0] === 'sdks';
  const ownsAuthoredSource =
    component.generated !== true &&
    ['rust-crate', 'node-package', 'react-package', 'web-backend-service', 'rust-route-crate'].includes(
      component.type
    );

  if (!Array.isArray(contracts.sdkDependencies)) {
    errors.push(`${relativePath} must declare contracts.sdkDependencies explicitly`);
  }
  if (!Array.isArray(contracts.dependencyApiExports)) {
    errors.push(`${relativePath} must declare contracts.dependencyApiExports explicitly`);
  }
  if (!Array.isArray(contracts.dependencyApiSurfaces)) {
    errors.push(`${relativePath} must declare contracts.dependencyApiSurfaces explicitly`);
  }

  if (isSdkArea) {
    const expectedSurface = expectedSdkComponentSurface(relativePath, manifest);
    if (component.surface !== expectedSurface) {
      errors.push(`${relativePath} component.surface must be ${expectedSurface}`);
    }
  }

  if (!isSdkArea && !component.surface) {
    validateSurfaceNotRequiredReason(relativePath, component, errors);
  }

  if (component.type === 'sdk-family') {
    if (!(component.manifests ?? []).includes('sdk-manifest.json')) {
      errors.push(`${relativePath} SDK family component.manifests must include sdk-manifest.json`);
    }
    if ((component.manifests ?? []).includes('.sdkwork-assembly.json')) {
      errors.push(`${relativePath} SDK family component.manifests must not reference retired .sdkwork-assembly.json`);
    }
    if ((contracts.runtimeEntrypoints ?? []).includes('.sdkwork-assembly.json')) {
      errors.push(`${relativePath} SDK family runtimeEntrypoints must not reference retired .sdkwork-assembly.json`);
    }
    if ((contracts.configKeys ?? []).includes('.sdkwork-assembly.json')) {
      errors.push(`${relativePath} SDK family configKeys must not reference retired .sdkwork-assembly.json`);
    }
    if (!Object.hasOwn(contracts, 'routeManifest')) {
      errors.push(`${relativePath} must declare contracts.routeManifest explicitly`);
    } else if (contracts.routeManifest !== null && typeof contracts.routeManifest !== 'string') {
      errors.push(`${relativePath} contracts.routeManifest must be null or a route manifest path`);
    }

    for (const specFile of [
      'SDK_SPEC.md',
      'SDK_WORKSPACE_GENERATION_SPEC.md',
      'API_SPEC.md',
      'TEST_SPEC.md',
      'DOCUMENTATION_SPEC.md'
    ]) {
      if (!canonicalSpecFiles.has(specFile)) {
        errors.push(`${relativePath} must cite ${specFile} in canonicalSpecs`);
      }
    }
  }

  if (ownsAuthoredSource) {
    for (const specFile of ['CODE_STYLE_SPEC.md', 'NAMING_SPEC.md']) {
      if (!canonicalSpecFiles.has(specFile)) {
        errors.push(`${relativePath} must cite ${specFile} in canonicalSpecs`);
      }
    }
  }
}

function validateSurfaceNotRequiredReason(relativePath, component, errors) {
  if (typeof component.surfaceNotRequiredReason !== 'string') {
    errors.push(`${relativePath} must explain why component.surface is not required`);
    return;
  }
  const reason = component.surfaceNotRequiredReason.trim();
  if (reason.length < 48) {
    errors.push(`${relativePath} component.surfaceNotRequiredReason must be specific`);
  }
  if (/\b(?:todo|tbd|n\/a)\b/i.test(reason)) {
    errors.push(`${relativePath} component.surfaceNotRequiredReason must not be a placeholder`);
  }
}

export function expectedSdkComponentSurface(relativePath, manifest) {
  const declaredSurface = manifest.sdk?.sdkSurface ?? manifest.sdk?.sdkType;
  if (declaredSurface === 'internal') {
    return 'internal-api';
  }
  if (declaredSurface === 'open' || declaredSurface === 'custom') {
    return 'open-api';
  }
  if (declaredSurface === 'app') {
    return 'app-api';
  }
  if (declaredSurface === 'backend') {
    return 'backend-admin';
  }

  const componentName = manifest.component?.name ?? '';
  const normalizedPath = relativePath.replaceAll('\\', '/');
  const surfaceSource = `${componentName} ${normalizedPath}`;
  if (surfaceSource.includes('-internal-sdk')) {
    return 'internal-api';
  }
  if (surfaceSource.includes('-backend-sdk')) {
    return 'backend-admin';
  }
  if (surfaceSource.includes('-app-sdk')) {
    return 'app-api';
  }

  return 'open-api';
}

function resolveComponentRoot(manifestPath, manifest, kernelRoot) {
  const componentRoot = manifest.component?.root;
  if (!componentRoot) {
    return path.dirname(path.dirname(manifestPath));
  }

  const normalizedRoot = componentRoot.replaceAll('\\', '/');
  const repositoryName = path.basename(kernelRoot);
  const candidates = [
    path.resolve(kernelRoot, normalizedRoot),
    normalizedRoot.startsWith(`${repositoryName}/`)
      ? path.resolve(path.dirname(kernelRoot), normalizedRoot)
      : null,
    path.dirname(path.dirname(manifestPath))
  ].filter(Boolean);

  return candidates.find((candidate) => fs.existsSync(candidate)) ?? candidates[0];
}

function readJsonFile(filePath, kernelRoot, errors) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    errors.push(`invalid json: ${path.relative(kernelRoot, filePath)}: ${error.message}`);
    return null;
  }
}
