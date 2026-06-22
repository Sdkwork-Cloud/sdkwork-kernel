import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const UI_CLIENT_PATH = path.join(
  root,
  'sdkwork-kernel-ui/packages/sdkwork-kernel-ui-services/src/service/kernel-ui.real.ts'
);
const SERVER_APP_PATH = path.join(root, 'sdkwork-agent-server/src/app.rs');
const SERVER_RUNTIME_ROUTES_PATH = path.join(root, 'sdkwork-agent-server/src/runtime_routes.rs');
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
  'sdkwork-agent-server/tests/http_kernel_contracts.rs'
);

const INTERNAL_RUNTIME_PREFIX = '/internal/v3/api/intelligence/runtime';

const REQUIRED_KERNEL_ROUTES = [
  '/api/kernel/snapshot',
  `${INTERNAL_RUNTIME_PREFIX}/snapshot`,
  '/api/kernel/permissions/{permission_request_id}',
  `${INTERNAL_RUNTIME_PREFIX}/permissions/{permission_request_id}`,
  '/api/kernel/sessions',
  `${INTERNAL_RUNTIME_PREFIX}/sessions`,
  '/api/kernel/sessions/{session_id}',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}`,
  '/api/kernel/sessions/{session_id}/close',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/close`,
  '/api/kernel/sessions/{session_id}/messages',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/messages`,
  '/api/kernel/sessions/{session_id}/tasks',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/tasks`,
  '/api/kernel/tasks/{task_id}',
  `${INTERNAL_RUNTIME_PREFIX}/tasks/{task_id}`,
  '/api/kernel/tasks/{task_id}/cancel',
  `${INTERNAL_RUNTIME_PREFIX}/tasks/{task_id}/cancel`,
  '/api/kernel/models',
  `${INTERNAL_RUNTIME_PREFIX}/models`,
  '/api/kernel/sessions/{session_id}/model/invoke',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/model/invoke`,
  '/api/kernel/sessions/{session_id}/tools',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/tools`,
  '/api/kernel/sessions/{session_id}/tools/{tool_name}/execute',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/tools/{tool_name}/execute`,
  '/api/kernel/sessions/{session_id}/events/stream',
  `${INTERNAL_RUNTIME_PREFIX}/sessions/{session_id}/events/stream`
];

function assertServerExposesRoute(serverSource, route) {
  if (serverSource.includes(route)) {
    return;
  }

  const legacyPrefix = '/api/kernel';
  const internalPrefix = '/internal/v3/api/intelligence/runtime';
  const relative = route.startsWith(internalPrefix)
    ? route.slice(internalPrefix.length)
    : route.startsWith(legacyPrefix)
      ? route.slice(legacyPrefix.length)
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
  assert.equal(fs.existsSync(INTERNAL_ROUTER_PATH), true);

  const uiSource = fs.readFileSync(UI_CLIENT_PATH, 'utf8');
  const serverSource = fs.readFileSync(SERVER_APP_PATH, 'utf8');
  const runtimeRoutesSource = fs.readFileSync(SERVER_RUNTIME_ROUTES_PATH, 'utf8');
  const internalRouterSource = fs.readFileSync(INTERNAL_ROUTER_PATH, 'utf8');

  assert.ok(runtimeRoutesSource.includes('INTERNAL_RUNTIME_MOUNT_PREFIX'));
  assert.ok(runtimeRoutesSource.includes('LEGACY_KERNEL_MOUNT_PREFIX'));
  assert.match(internalRouterSource, /internal_route_manifest/);
  assert.match(internalRouterSource, /build_kernel_runtime_routes/);

  assert.ok(
    serverSource.includes('LEGACY_KERNEL_MOUNT_PREFIX') && serverSource.includes('build_kernel_runtime_routes')
  );
  assert.ok(
    serverSource.includes('INTERNAL_RUNTIME_MOUNT_PREFIX')
      && serverSource.includes('build_kernel_runtime_routes')
  );

  const routeSource = `${serverSource}\n${runtimeRoutesSource}`;

  for (const route of REQUIRED_KERNEL_ROUTES) {
    assertServerExposesRoute(routeSource, route);
  }

  assert.match(uiSource, /@sdkwork\/agent-internal-sdk/);
  assert.match(uiSource, /createClient\(/);
  assert.match(uiSource, /intelligence\.runtime\.snapshot\.load/);
  assert.match(uiSource, /intelligence\.runtime\.sessions\.events\.stream/);
  assert.match(uiSource, /response\.items \?\? \[\]/);
  assert.doesNotMatch(uiSource, /\/api\/kernel\//, 'kernel UI must not call legacy /api/kernel paths directly');
  assert.doesNotMatch(uiSource, /new EventSource\(/, 'SSE must not use browser EventSource without auth headers');
});

test('agent-server HTTP surface spec documents kernel ingress security and readiness probes', () => {
  assert.equal(fs.existsSync(SERVER_SURFACE_PATH), true);
  const source = fs.readFileSync(SERVER_SURFACE_PATH, 'utf8');
  const contracts = fs.readFileSync(SERVER_HTTP_CONTRACTS_PATH, 'utf8');
  assert.match(source, /\/internal\/v3\/api\/intelligence\/runtime/);
  assert.match(source, /\/api\/kernel/);
  assert.match(source, /ingress/i);
  assert.match(source, /ready|health/i);
  assert.match(contracts, /ingress_token_auth_rejects_missing_credentials/);
  assert.match(contracts, /kernel_session_create_and_read_roundtrip/);
  assert.match(contracts, /internal_runtime_session_roundtrip_uses_items_list_envelope/);
});

test('kernel UI session config uses string tenantId aligned with server metadata', () => {
  const typesPath = path.join(root, 'sdkwork-kernel-ui/packages/sdkwork-kernel-ui-types/src/index.ts');
  const typesSource = fs.readFileSync(typesPath, 'utf8');
  assert.match(typesSource, /tenantId\??: string/);
});
