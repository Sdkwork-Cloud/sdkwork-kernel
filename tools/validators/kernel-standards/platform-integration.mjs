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

const REQUIRED_ROUTE_CRATES = [
  'crates/sdkwork-router-agent-http-shared',
  'crates/sdkwork-router-agent-open-api',
  'crates/sdkwork-router-agent-app-api',
  'crates/sdkwork-router-agent-backend-api'
];

const ROUTE_HTTP_SHARED_DEPENDENCIES = [
  'sdkwork-web-axum',
  'sdkwork-web-core',
  'sdkwork-iam-web-adapter'
];

const AGENT_BUSINESS_POSTGRES_SYNC_DEPENDENCIES = [
  'sdkwork-database-config',
  'sdkwork-database-sqlx'
];

function cargoDeclaresDependency(cargoToml, dependencyName) {
  const dependencyPattern = new RegExp(`^${dependencyName}(?:\\.workspace)?\\s*=`, 'm');
  return dependencyPattern.test(cargoToml);
}

function cargoFeatureIncludesDependency(cargoToml, featureName, dependencyName) {
  const featureBlock = cargoToml.match(new RegExp(`^${featureName}\\s*=\\s*\\[([^\\]]*)\\]`, 'ms'));
  if (!featureBlock) {
    return false;
  }
  return featureBlock[1].includes(`dep:${dependencyName}`);
}

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
    'Phase 3',
    'Phase 4'
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

  for (const routeCrate of REQUIRED_ROUTE_CRATES) {
    const crateCargo = path.join(kernelRoot, routeCrate, 'Cargo.toml');
    const componentSpec = path.join(kernelRoot, routeCrate, 'specs', 'component.spec.json');
    if (!fs.existsSync(crateCargo)) {
      errors.push(`${routeCrate}/Cargo.toml must exist for Phase 1 route boundary extraction`);
    }
    if (!fs.existsSync(componentSpec)) {
      errors.push(`${routeCrate}/specs/component.spec.json must exist for Phase 1 route boundary extraction`);
    }
  }

  const httpSharedCargo = readFileIfExists(path.join(kernelRoot, REQUIRED_ROUTE_CRATES[0], 'Cargo.toml'));
  if (httpSharedCargo) {
    for (const dependency of ROUTE_HTTP_SHARED_DEPENDENCIES) {
      if (!cargoDeclaresDependency(httpSharedCargo, dependency)) {
        errors.push(
          `${REQUIRED_ROUTE_CRATES[0]}/Cargo.toml must depend on ${dependency} for sdkwork-web-framework integration`
        );
      }
    }
    const webBootstrap = readFileIfExists(
      path.join(kernelRoot, REQUIRED_ROUTE_CRATES[0], 'src', 'web_bootstrap.rs')
    );
    if (webBootstrap && !webBootstrap.includes('build_served_combined_router')) {
      errors.push(
        `${REQUIRED_ROUTE_CRATES[0]}/src/web_bootstrap.rs must expose build_served_combined_router for served HTTP surfaces`
      );
    }
    if (webBootstrap && webBootstrap.includes('SDKWORK_AGENT_WEB_FRAMEWORK_ENABLED')) {
      errors.push(
        `${REQUIRED_ROUTE_CRATES[0]}/src/web_bootstrap.rs must not retain SDKWORK_AGENT_WEB_FRAMEWORK_ENABLED opt-in; served routers always use web-framework`
      );
    }
    if (webBootstrap && !webBootstrap.includes('/agent/v3/api')) {
      errors.push(
        `${REQUIRED_ROUTE_CRATES[0]}/src/web_bootstrap.rs must register /agent/v3/api in the web request context profile`
      );
    }
  }

  for (const surfaceCrate of REQUIRED_ROUTE_CRATES.slice(1)) {
    const libRs = readFileIfExists(path.join(kernelRoot, surfaceCrate, 'src', 'lib.rs'));
    if (libRs && !libRs.includes('wrap_router_with_web_framework_from_env')) {
      errors.push(`${surfaceCrate}/src/lib.rs must wrap served routers with sdkwork-web-framework`);
    }
    if (
      libRs
      && (libRs.includes('build_open_router().with_state')
        || libRs.includes('build_backend_router().with_state'))
    ) {
      errors.push(
        `${surfaceCrate}/src/lib.rs build_served_router must use raw route builders to avoid duplicate gateway middleware`
      );
    }
  }

  const agentBusinessCargo = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-business', 'Cargo.toml'));
  if (agentBusinessCargo) {
    for (const dependency of AGENT_BUSINESS_POSTGRES_SYNC_DEPENDENCIES) {
      if (!cargoDeclaresDependency(agentBusinessCargo, dependency)) {
        errors.push(
          `sdkwork-agent-business/Cargo.toml must declare ${dependency} for postgres-sync database alignment`
        );
      }
      if (!cargoFeatureIncludesDependency(agentBusinessCargo, 'postgres-sync', dependency)) {
        errors.push(
          `sdkwork-agent-business postgres-sync feature must include dep:${dependency}`
        );
      }
    }
  }

  const persistenceRs = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-business', 'src', 'persistence.rs')
  );
  if (persistenceRs && !persistenceRs.includes('BlockingPostgresPool')) {
    errors.push(
      'sdkwork-agent-business/src/persistence.rs must use BlockingPostgresPool from sdkwork-database-sqlx for postgres-sync'
    );
  }

  const postgresSyncPool = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-business', 'src', 'postgres_sync_pool.rs')
  );
  if (!postgresSyncPool || !postgresSyncPool.includes('create_pool_from_config')) {
    errors.push(
      'sdkwork-agent-business/src/postgres_sync_pool.rs must bootstrap pools through sdkwork-database-sqlx'
    );
  }

  const agentDatabaseCargo = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-database', 'Cargo.toml'));
  if (agentDatabaseCargo && agentDatabaseCargo.includes('postgres = [')) {
    errors.push(
      'sdkwork-agent-database must not ship a broken postgres feature stub; use sdkwork-database for PostgreSQL pools'
    );
  }

  const workflowConfigPath = path.join(kernelRoot, 'sdkwork.workflow.json');
  if (!fs.existsSync(workflowConfigPath)) {
    errors.push('sdkwork.workflow.json must exist for Phase 4 packaging entrypoint');
  } else {
    let workflowConfig;
    try {
      workflowConfig = JSON.parse(fs.readFileSync(workflowConfigPath, 'utf8'));
    } catch (error) {
      errors.push(`invalid json: sdkwork.workflow.json: ${error.message}`);
      workflowConfig = null;
    }
    if (workflowConfig) {
      if (workflowConfig.app?.id !== 'sdkwork-kernel') {
        errors.push('sdkwork.workflow.json app.id must be sdkwork-kernel');
      }
      if (!Array.isArray(workflowConfig.targets) || workflowConfig.targets.length === 0) {
        errors.push('sdkwork.workflow.json must declare at least one package target');
      }
      const buildSteps = workflowConfig.lifecycle?.build;
      if (!Array.isArray(buildSteps) || !buildSteps.some((step) => String(step.run || '').includes('verify-kernel-audit-remediation.mjs'))) {
        errors.push(
          'sdkwork.workflow.json lifecycle.build must run scripts/verify-kernel-audit-remediation.mjs'
        );
      }
    }
  }

  const packageWorkflowPath = path.join(kernelRoot, '.github', 'workflows', 'package.yml');
  if (!fs.existsSync(packageWorkflowPath)) {
    errors.push('.github/workflows/package.yml must exist for Phase 4 packaging entrypoint');
  } else {
    const packageWorkflow = readFileIfExists(packageWorkflowPath);
    if (packageWorkflow && !packageWorkflow.includes('sdkwork-github-workflow/.github/workflows/sdkwork-package.yml')) {
      errors.push('.github/workflows/package.yml must call sdkwork-github-workflow reusable packaging workflow');
    }
    if (packageWorkflow && !packageWorkflow.includes('config_path: sdkwork.workflow.json')) {
      errors.push('.github/workflows/package.yml must pass config_path: sdkwork.workflow.json');
    }
  }
}
