import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const UI_AUTH_PROVIDER_PATH = path.join(
  root,
  'sdkwork-kernel-ui/packages/sdkwork-kernel-ui-services/src/service/kernel-ui-auth.provider.ts'
);
const UI_CLIENT_PATH = path.join(
  root,
  'sdkwork-kernel-ui/packages/sdkwork-kernel-ui-services/src/service/kernel-ui.real.ts'
);
const SERVER_APP_PATH = path.join(root, 'sdkwork-agent-server/src/app.rs');
const SERVER_RUNTIME_ROUTES_PATH = path.join(root, 'sdkwork-agent-server/src/runtime_routes.rs');
const SERVER_INTERNAL_RUNTIME_PATH = path.join(
  root,
  'sdkwork-agent-server/src/api/internal_runtime.rs'
);
const INTERNAL_ROUTER_PATH = path.join(
  root,
  'crates/sdkwork-router-agent-internal-api/src/lib.rs'
);
const SERVER_SURFACE_PATH = path.join(
  root,
  'sdkwork-agent-server/specs/AGENT_SERVER_HTTP_SURFACE.md'
);
const SERVER_HTTP_CONTRACTS_PATH = path.join(
  root,
  'sdkwork-agent-server/tests/http_internal_runtime_contracts.rs'
);
const INTERNAL_OPENAPI_PATH = path.join(
  root,
  'apis/internal-api/intelligence/sdkwork-agent-internal-api.openapi.yaml'
);
const INTERNAL_AUTHORITY_INDEX_PATH = path.join(root, 'apis/internal-api/authority-index.json');
const CLIENT_SSE_PATH = path.join(root, 'sdkwork-agent-client/src/sse.rs');
const INTERNAL_SDK_PATHS_PATH = path.join(
  root,
  'sdks/sdkwork-agent-internal-sdk/sdkwork-agent-internal-sdk-typescript/generated/server-openapi/src/api/paths.ts'
);

const INTERNAL_RUNTIME_PREFIX = '/internal/v3/api/intelligence/runtime';

const REQUIRED_INTERNAL_RUNTIME_ROUTES = [
  `${INTERNAL_RUNTIME_PREFIX}/snapshot`,
  `${INTERNAL_RUNTIME_PREFIX}/permissions/{permission_request_id}`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/close`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/messages`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/tasks`,
  `${INTERNAL_RUNTIME_PREFIX}/tasks/{task_id}`,
  `${INTERNAL_RUNTIME_PREFIX}/tasks/{task_id}/cancel`,
  `${INTERNAL_RUNTIME_PREFIX}/models`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/model/invoke`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/tools`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/tools/{tool_name}/execute`,
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/events/stream`
];

function assertServerExposesRoute(serverSource, route) {
  if (serverSource.includes(route)) {
    return;
  }

  const relative = route.startsWith(INTERNAL_RUNTIME_PREFIX)
    ? route.slice(INTERNAL_RUNTIME_PREFIX.length)
    : null;

  if (!relative) {
    assert.fail(`agent-server missing route: ${route}`);
  }

  const routeNeedle = relative.startsWith('/') ? relative : `/${relative}`;

  if (routeNeedle.includes('/events/stream')) {
    assert.ok(
      serverSource.includes('events/stream') && serverSource.includes('stream_session_events'),
      `agent-server missing SSE stream route wiring for ${route}`
    );
    return;
  }

  const quotedRoute = routeNeedle.replace(/\{[^}]+\}/g, '{session_id}');
  assert.match(
    serverSource,
    new RegExp(
      `\\.route\\(\\s*["']${quotedRoute.replace(/\{session_id\}/g, '\\{[^}]+\\}').replace(/\//g, '\\/')}["']`
    ),
    `agent-server missing nested route: ${route}`
  );
}

test('kernel UI client uses internal SDK and agent-server exposes runtime routes', () => {
  assert.equal(fs.existsSync(UI_CLIENT_PATH), true);
  assert.equal(fs.existsSync(SERVER_APP_PATH), true);
  assert.equal(fs.existsSync(SERVER_HTTP_CONTRACTS_PATH), true);

  assert.equal(fs.existsSync(SERVER_RUNTIME_ROUTES_PATH), true);
  assert.equal(fs.existsSync(SERVER_INTERNAL_RUNTIME_PATH), true);
  assert.equal(fs.existsSync(INTERNAL_ROUTER_PATH), true);

  const uiSource = fs.readFileSync(UI_CLIENT_PATH, 'utf8');
  const authProviderSource = fs.readFileSync(UI_AUTH_PROVIDER_PATH, 'utf8');
  const serverSource = fs.readFileSync(SERVER_APP_PATH, 'utf8');
  const runtimeRoutesSource = fs.readFileSync(SERVER_RUNTIME_ROUTES_PATH, 'utf8');
  const internalRuntimeSource = fs.readFileSync(SERVER_INTERNAL_RUNTIME_PATH, 'utf8');
  const internalRouterSource = fs.readFileSync(INTERNAL_ROUTER_PATH, 'utf8');

  assert.match(internalRuntimeSource, /struct InternalRuntimeApiState/);
  assert.doesNotMatch(internalRuntimeSource, /struct KernelApiState/);
  assert.doesNotMatch(runtimeRoutesSource, /api::kernel/);

  assert.ok(runtimeRoutesSource.includes('INTERNAL_RUNTIME_MOUNT_PREFIX'));
  assert.doesNotMatch(runtimeRoutesSource, /LEGACY_KERNEL_MOUNT_PREFIX/);
  assert.doesNotMatch(serverSource, /LEGACY_KERNEL_MOUNT_PREFIX/);
  assert.doesNotMatch(serverSource, /\/api\/kernel/);
  assert.doesNotMatch(serverSource, /\/api\/sessions/);
  assert.doesNotMatch(serverSource, /\/api\/chat/);
  assert.match(internalRouterSource, /internal_route_manifest/);
  assert.match(internalRouterSource, /build_internal_runtime_routes/);
  assert.doesNotMatch(internalRouterSource, /build_legacy_router/);

  assert.ok(
    serverSource.includes('INTERNAL_RUNTIME_MOUNT_PREFIX')
      && serverSource.includes('build_internal_runtime_routes')
  );

  const routeSource = `${serverSource}\n${runtimeRoutesSource}`;

  for (const route of REQUIRED_INTERNAL_RUNTIME_ROUTES) {
    assertServerExposesRoute(routeSource, route);
  }

  assert.match(uiSource, /@sdkwork\/agent-internal-sdk/);
  assert.match(uiSource, /createClient\(/);
  assert.match(uiSource, /buildKernelUiAuthHeaders/);
  assert.match(authProviderSource, /x-sdkwork-tenant-id/);
  assert.match(authProviderSource, /x-sdkwork-identity-mac/);
  assert.match(uiSource, /intelligence\.runtime\.snapshot\.load/);
  assert.match(uiSource, /intelligence\.runtime\.sessions\.events\.stream/);
  assert.match(uiSource, /response\.items \?\? \[\]/);
  assert.doesNotMatch(uiSource, /\/api\/kernel\//, 'kernel UI must not call retired /api/kernel paths');
  assert.doesNotMatch(uiSource, /new EventSource\(/, 'SSE must not use browser EventSource without auth headers');
});

function readMountPrefixFromRust(source) {
  const match = source.match(
    /pub const INTERNAL_RUNTIME_MOUNT_PREFIX: &str = "([^"]+)"/
  );
  assert.ok(match, 'Rust source must declare INTERNAL_RUNTIME_MOUNT_PREFIX');
  return match[1];
}

function readRuntimeMountPrefixFromOpenApi(source) {
  const paths = [...source.matchAll(/^  (\/internal\/v3\/api\/[^\n:]+):/gm)].map(
    (entry) => entry[1]
  );
  assert.ok(paths.length > 0, 'internal-api OpenAPI must declare runtime paths');
  for (const pathValue of paths) {
    assert.ok(
      pathValue.startsWith(`${INTERNAL_RUNTIME_PREFIX}/`) || pathValue === INTERNAL_RUNTIME_PREFIX,
      `OpenAPI path must mount under runtime prefix: ${pathValue}`
    );
  }
  return INTERNAL_RUNTIME_PREFIX;
}

test('internal-api runtime mount prefix stays aligned across authority, server, client, and SDK', () => {
  const openapiSource = fs.readFileSync(INTERNAL_OPENAPI_PATH, 'utf8');
  const authorityIndex = JSON.parse(fs.readFileSync(INTERNAL_AUTHORITY_INDEX_PATH, 'utf8'));
  const runtimeRoutesSource = fs.readFileSync(SERVER_RUNTIME_ROUTES_PATH, 'utf8');
  const clientSseSource = fs.readFileSync(CLIENT_SSE_PATH, 'utf8');
  const sdkPathsSource = fs.readFileSync(INTERNAL_SDK_PATHS_PATH, 'utf8');

  const openapiPrefix = readRuntimeMountPrefixFromOpenApi(openapiSource);
  const serverPrefix = readMountPrefixFromRust(runtimeRoutesSource);
  const clientPrefix = readMountPrefixFromRust(clientSseSource);

  const sdkApiPrefixMatch = sdkPathsSource.match(
    /export const CUSTOM_API_PREFIX = '([^']+)'/
  );
  assert.ok(sdkApiPrefixMatch, 'internal SDK must export CUSTOM_API_PREFIX');
  const sdkRuntimePrefix = `${sdkApiPrefixMatch[1]}/intelligence/runtime`;

  const authorityPrefix = `${authorityIndex.surfaces[0].apiPrefix}/intelligence/runtime`;

  assert.equal(openapiPrefix, INTERNAL_RUNTIME_PREFIX);
  assert.equal(serverPrefix, INTERNAL_RUNTIME_PREFIX);
  assert.equal(clientPrefix, INTERNAL_RUNTIME_PREFIX);
  assert.equal(sdkRuntimePrefix, INTERNAL_RUNTIME_PREFIX);
  assert.equal(authorityPrefix, INTERNAL_RUNTIME_PREFIX);
  assert.equal(authorityIndex.surfaces[0].surface, 'internal-api');
  assert.equal(authorityIndex.surfaces[0].ingress, 'application.public-ingress');
});

test('agent-client remote HTTP uses canonical ingress auth and signed identity headers', () => {
  const clientSseSource = fs.readFileSync(CLIENT_SSE_PATH, 'utf8');
  const clientIngressSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-client/src/ingress_auth.rs'),
    'utf8'
  );
  const clientContractSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-client/tests/ingress_auth_contract.rs'),
    'utf8'
  );

  assert.match(clientSseSource, /ingress_auth::apply_ingress_auth/);
  assert.match(clientIngressSource, /Authorization/);
  assert.match(clientIngressSource, /x-api-key/);
  assert.match(clientIngressSource, /x-sdkwork-tenant-id/);
  assert.match(clientIngressSource, /x-sdkwork-identity-mac/);
  assert.match(clientIngressSource, /compute_identity_mac/);
  assert.match(clientContractSource, /apply_ingress_auth_sets_canonical_headers/);
});

test('agent-server HTTP surface spec documents kernel ingress security and readiness probes', () => {
  assert.equal(fs.existsSync(SERVER_SURFACE_PATH), true);
  const source = fs.readFileSync(SERVER_SURFACE_PATH, 'utf8');
  const contracts = fs.readFileSync(SERVER_HTTP_CONTRACTS_PATH, 'utf8');
  assert.match(source, /\/internal\/v3\/api\/intelligence\/runtime/);
  assert.doesNotMatch(source, /Legacy kernel UI alias/);
  assert.match(source, /ingress/i);
  assert.match(source, /ready|health/i);
  assert.match(contracts, /metrics_endpoint_exposes_prometheus_families_without_auth/);
  assert.match(source, /GET \/metrics/);
  assert.match(source, /route template/i);

  const middlewareSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-server/src/middleware.rs'),
    'utf8'
  );
  const metricsSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-server/src/metrics.rs'),
    'utf8'
  );
  const httpSurfaceSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-server/src/http_surface.rs'),
    'utf8'
  );
  assert.match(middlewareSource, /route_template/);
  assert.match(httpSurfaceSource, /fn route_template/);
  assert.match(metricsSource, /sdkwork_kernel_http_requests_total/);
  assert.match(metricsSource, /sdkwork_kernel_health_status/);
  assert.match(metricsSource, /sdkwork_kernel_runtime_persistence_backend_info/);
  assert.match(metricsSource, /sdkwork_kernel_rate_limit_backend_info/);
  assert.match(metricsSource, /sdkwork_kernel_model_invocations_total/);
  assert.match(metricsSource, /sdkwork_kernel_model_tokens_total/);
  const ingressJwtSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-server/src/ingress_jwt.rs'),
    'utf8'
  );
  const usageMeterSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-server/src/usage_meter.rs'),
    'utf8'
  );
  assert.match(source, /JWT ingress/i);
  assert.match(source, /SDKWORK_KERNEL_INGRESS_JWT_RSA_PUBLIC_KEY_PEM/);
  assert.match(source, /SDKWORK_KERNEL_INGRESS_JWT_JWKS_URL/);
  assert.match(source, /SDKWORK_TENANT_RATE_LIMIT_OVERRIDES/);
  assert.match(ingressJwtSource, /validate_ingress_jwt/);
  assert.match(ingressJwtSource, /load_jwks_file/);
  assert.match(ingressJwtSource, /fetch_jwks_url/);
  const rateLimitSource = fs.readFileSync(
    path.join(root, 'sdkwork-agent-server/src/rate_limit.rs'),
    'utf8'
  );
  assert.match(rateLimitSource, /tenant_overrides/);
  const clientIngressAuth = fs.readFileSync(
    path.join(root, 'sdkwork-agent-client/src/ingress_auth.rs'),
    'utf8'
  );
  assert.match(clientIngressAuth, /ingress_profile/);
  assert.match(usageMeterSource, /usage_meter/);
  assert.match(contracts, /retired_kernel_alias_snapshot_returns_not_found/);
  assert.match(contracts, /retired_legacy_session_paths_return_not_found/);
  assert.match(contracts, /internal_runtime_session_api_enforces_token_scope/);
  assert.match(contracts, /internal_runtime_session_create_and_read_roundtrip/);
  assert.match(contracts, /internal_runtime_session_roundtrip_uses_items_list_envelope/);
  assert.match(contracts, /api::internal_runtime::InternalRuntimeApiState/);
});

test('kernel UI session config uses string tenantId aligned with server metadata', () => {
  const typesPath = path.join(root, 'sdkwork-kernel-ui/packages/sdkwork-kernel-ui-types/src/index.ts');
  const typesSource = fs.readFileSync(typesPath, 'utf8');
  assert.match(typesSource, /tenantId\??: string/);
});
