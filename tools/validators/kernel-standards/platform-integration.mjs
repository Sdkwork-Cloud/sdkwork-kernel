import fs from 'node:fs';
import path from 'node:path';

const AUTHORITY_INDEX_PATH = path.join('apis', 'agent-business', 'authority-index.json');

const PLATFORM_ADR_PATH = path.join(
  'docs',
  'architecture',
  'decisions',
  'ADR-20260618-platform-framework-adoption.md'
);

const REQUIRED_WORKSPACE_DEPENDENCIES = [
  'sdkwork-web-core',
  'sdkwork-web-axum',
  'sdkwork-web-contract',
  'sdkwork-web-bootstrap',
  'sdkwork-database-config',
  'sdkwork-database-sqlx',
  'sdkwork-iam-web-adapter'
];

const REQUIRED_AUTHORITY_SURFACES = ['open-api', 'app-api', 'backend-api'];

/**
 * Phase 0 platform alignment: authority index, ADR evidence, and declared workspace
 * dependencies on sdkwork-web-framework and sdkwork-database.
 */
export function validatePlatformIntegration({ kernelRoot, errors, ensureFile, readFileIfExists }) {
  ensureFile(PLATFORM_ADR_PATH);
  ensureFile(AUTHORITY_INDEX_PATH);

  const adrText = readFileIfExists(path.join(kernelRoot, PLATFORM_ADR_PATH));
  for (const requiredText of [
    'Status: accepted',
    'WEB_FRAMEWORK_SPEC.md',
    'DATABASE_SPEC.md',
    'sdkwork-web-framework',
    'sdkwork-database',
    'sdkwork-discovery',
    'Phase 0',
    'Phase 1',
    'Phase 2',
    'Phase 3'
  ]) {
    if (!adrText.includes(requiredText)) {
      errors.push(
        `${PLATFORM_ADR_PATH} must document platform adoption (${requiredText})`
      );
    }
  }

  const authorityIndexPath = path.join(kernelRoot, AUTHORITY_INDEX_PATH);
  let authorityIndex;
  try {
    authorityIndex = JSON.parse(fs.readFileSync(authorityIndexPath, 'utf8'));
  } catch (error) {
    errors.push(`invalid json: ${AUTHORITY_INDEX_PATH}: ${error.message}`);
    return;
  }

  if (!Array.isArray(authorityIndex.authorities) || authorityIndex.authorities.length === 0) {
    errors.push(`${AUTHORITY_INDEX_PATH} must declare at least one API authority`);
    return;
  }

  const surfaces = new Set();
  for (const authority of authorityIndex.authorities) {
    if (!authority.surface || !authority.relativePath) {
      errors.push(`${AUTHORITY_INDEX_PATH} authority entries require surface and relativePath`);
      continue;
    }
    surfaces.add(authority.surface);

    const authorityFile = path.resolve(path.dirname(authorityIndexPath), authority.relativePath);
    if (!fs.existsSync(authorityFile)) {
      errors.push(
        `${AUTHORITY_INDEX_PATH} authority ${authority.surface} points to missing file: ${authority.relativePath}`
      );
    }
  }

  for (const surface of REQUIRED_AUTHORITY_SURFACES) {
    if (!surfaces.has(surface)) {
      errors.push(`${AUTHORITY_INDEX_PATH} must index ${surface} authority`);
    }
  }

  const workspaceCargoPath = path.join(kernelRoot, 'Cargo.toml');
  const workspaceCargo = readFileIfExists(workspaceCargoPath);
  if (!workspaceCargo) {
    errors.push('missing workspace Cargo.toml for platform dependency declaration');
    return;
  }

  for (const dependency of REQUIRED_WORKSPACE_DEPENDENCIES) {
    if (!workspaceCargo.includes(`${dependency} =`)) {
      errors.push(
        `Cargo.toml [workspace.dependencies] must declare ${dependency} for platform framework/database adoption`
      );
    }
  }

  const apisReadmePath = path.join(kernelRoot, 'apis', 'README.md');
  const apisReadme = readFileIfExists(apisReadmePath);
  if (!apisReadme.includes('agent-business/authority-index.json')) {
    errors.push('apis/README.md must reference apis/agent-business/authority-index.json');
  }
}
