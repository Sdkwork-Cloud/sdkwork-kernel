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
  'crates/sdkwork-router-agent-backend-api',
  'crates/sdkwork-router-agent-internal-api'
];

const WEB_FRAMEWORK_ROUTE_CRATES = [
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

  for (const surfaceCrate of WEB_FRAMEWORK_ROUTE_CRATES) {
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

  const internalAuthorityIndex = path.join(kernelRoot, 'apis', 'internal-api', 'authority-index.json');
  if (!fs.existsSync(internalAuthorityIndex)) {
    errors.push('apis/internal-api/authority-index.json must exist for internal-api surface indexing');
  }

  const internalRouterLib = readFileIfExists(
    path.join(kernelRoot, 'crates/sdkwork-router-agent-internal-api', 'src', 'lib.rs')
  );
  if (internalRouterLib && !internalRouterLib.includes('internal_route_manifest')) {
    errors.push(
      'crates/sdkwork-router-agent-internal-api/src/lib.rs must export internal_route_manifest for internal-api route boundary'
    );
  }
  if (internalRouterLib && !internalRouterLib.includes('build_internal_runtime_routes')) {
    errors.push(
      'crates/sdkwork-router-agent-internal-api/src/lib.rs must re-export build_internal_runtime_routes'
    );
  }

  const internalRuntimeHandler = path.join(
    kernelRoot,
    'sdkwork-agent-server',
    'src',
    'api',
    'internal_runtime.rs'
  );
  if (!fs.existsSync(internalRuntimeHandler)) {
    errors.push(
      'sdkwork-agent-server/src/api/internal_runtime.rs must exist for internal-api runtime handlers'
    );
  }
  const legacyKernelHandler = path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'api', 'kernel.rs');
  if (fs.existsSync(legacyKernelHandler)) {
    errors.push('sdkwork-agent-server/src/api/kernel.rs is retired; use internal_runtime.rs');
  }

  const runtimeRoutes = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'runtime_routes.rs')
  );
  if (runtimeRoutes) {
    if (!runtimeRoutes.includes('build_internal_runtime_routes')) {
      errors.push('sdkwork-agent-server/src/runtime_routes.rs must export build_internal_runtime_routes');
    }
    if (!runtimeRoutes.includes('INTERNAL_RUNTIME_MOUNT_PREFIX')) {
      errors.push('sdkwork-agent-server/src/runtime_routes.rs must declare INTERNAL_RUNTIME_MOUNT_PREFIX');
    }
    if (
      runtimeRoutes.includes('LEGACY_KERNEL_MOUNT_PREFIX')
      || runtimeRoutes.includes('build_kernel_runtime_routes')
    ) {
      errors.push('sdkwork-agent-server/src/runtime_routes.rs must not retain retired kernel mount helpers');
    }
  }

  const serverApp = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'app.rs'));
  if (
    serverApp
    && (serverApp.includes('/api/sessions')
      || serverApp.includes('/api/chat')
      || serverApp.includes('/api/kernel'))
  ) {
    errors.push('sdkwork-agent-server/src/app.rs must not mount retired legacy HTTP prefixes');
  }
  if (serverApp && !serverApp.includes('"/metrics"')) {
    errors.push('sdkwork-agent-server/src/app.rs must expose GET /metrics for production observability');
  }

  const serverMetrics = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'metrics.rs'));
  if (!serverMetrics || !serverMetrics.includes('sdkwork_kernel_http_requests_total')) {
    errors.push(
      'sdkwork-agent-server/src/metrics.rs must expose sdkwork_kernel_http_requests_total per OBSERVABILITY_SPEC.md'
    );
  }
  if (serverMetrics && !serverMetrics.includes('sdkwork_kernel_runtime_persistence_backend_info')) {
    errors.push(
      'sdkwork-agent-server/src/metrics.rs must expose runtime persistence backend gauge for multi-replica ops'
    );
  }
  if (serverMetrics && !serverMetrics.includes('sdkwork_kernel_rate_limit_backend_info')) {
    errors.push(
      'sdkwork-agent-server/src/metrics.rs must expose rate-limit backend gauge for distributed limiter ops'
    );
  }

  const serverPersistence = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'persistence.rs')
  );
  if (!serverPersistence || !serverPersistence.includes('PersistenceBackend')) {
    errors.push(
      'sdkwork-agent-server/src/persistence.rs must define PersistenceBackend for sqlite/postgres runtime sessions'
    );
  }

  const serverRateLimit = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'rate_limit.rs')
  );
  if (!serverRateLimit || !serverRateLimit.includes('uses_redis')) {
    errors.push(
      'sdkwork-agent-server/src/rate_limit.rs must expose uses_redis for distributed rate limiting'
    );
  }
  if (serverRateLimit && !serverRateLimit.includes('tenant_overrides')) {
    errors.push(
      'sdkwork-agent-server/src/rate_limit.rs must apply per-tenant rate limit overrides for commercial tenancy'
    );
  }

  const serverIngressJwt = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'ingress_jwt.rs')
  );
  if (!serverIngressJwt || !serverIngressJwt.includes('validate_ingress_jwt')) {
    errors.push(
      'sdkwork-agent-server/src/ingress_jwt.rs must validate enterprise ingress JWT credentials'
    );
  }
  if (serverIngressJwt && !serverIngressJwt.includes('load_jwks_file')) {
    errors.push(
      'sdkwork-agent-server/src/ingress_jwt.rs must support local JWKS file material for RS256 enterprise ingress'
    );
  }
  if (serverIngressJwt && !serverIngressJwt.includes('fetch_jwks_url')) {
    errors.push(
      'sdkwork-agent-server/src/ingress_jwt.rs must fetch remote JWKS URL material at startup for enterprise OIDC ingress'
    );
  }

  const serverUsageMeter = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'usage_meter.rs')
  );
  if (!serverUsageMeter || !serverUsageMeter.includes('usage_meter')) {
    errors.push(
      'sdkwork-agent-server/src/usage_meter.rs must emit structured commercial usage facts'
    );
  }

  const serverConfig = readFileIfExists(path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'config.rs'));
  if (
    serverConfig &&
    (!serverConfig.includes('runtime_database_engine') ||
      !serverConfig.includes('requires_distributed_rate_limit'))
  ) {
    errors.push(
      'sdkwork-agent-server/src/config.rs must declare runtime_database_engine and requires_distributed_rate_limit'
    );
  }

  const serverMiddleware = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'middleware.rs')
  );
  if (serverMiddleware && !serverMiddleware.includes('route_template')) {
    errors.push(
      'sdkwork-agent-server/src/middleware.rs must log route templates instead of raw paths per OBSERVABILITY_SPEC.md'
    );
  }

  const serverHttpSurface = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-server', 'src', 'http_surface.rs')
  );
  if (serverHttpSurface && !serverHttpSurface.includes('fn route_template')) {
    errors.push('sdkwork-agent-server/src/http_surface.rs must define route_template for observability labels');
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
  const agentDatabasePostgresPool = readFileIfExists(
    path.join(kernelRoot, 'sdkwork-agent-database', 'src', 'postgres_pool.rs')
  );
  if (
    agentDatabaseCargo?.includes('postgres-sync') &&
    (!agentDatabasePostgresPool || !agentDatabasePostgresPool.includes('create_pool_from_config'))
  ) {
    errors.push(
      'sdkwork-agent-database/src/postgres_pool.rs must bootstrap pools through sdkwork-database-sqlx when postgres-sync is enabled'
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
