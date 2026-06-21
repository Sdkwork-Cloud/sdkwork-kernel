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
const SERVER_SURFACE_PATH = path.join(
  root,
  'sdkwork-agent-server/specs/AGENT_SERVER_HTTP_SURFACE.md'
);
const SERVER_HTTP_CONTRACTS_PATH = path.join(
  root,
  'sdkwork-agent-server/tests/http_kernel_contracts.rs'
);

const REQUIRED_KERNEL_ROUTES = [
  '/api/kernel/snapshot',
  '/api/kernel/permissions/{permission_request_id}',
  '/api/kernel/sessions',
  '/api/kernel/sessions/{session_id}',
  '/api/kernel/sessions/{session_id}/close',
  '/api/kernel/sessions/{session_id}/messages',
  '/api/kernel/sessions/{session_id}/tasks',
  '/api/kernel/tasks/{task_id}',
  '/api/kernel/tasks/{task_id}/cancel',
  '/api/kernel/models',
  '/api/kernel/sessions/{session_id}/model/invoke',
  '/api/kernel/sessions/{session_id}/tools',
  '/api/kernel/sessions/{session_id}/tools/{tool_name}/execute',
  '/api/kernel/sessions/{session_id}/events/stream'
];

function normalizeUiPath(rawPath) {
  return rawPath
    .replace(/\$\{encodeURIComponent\([^}]+\)\}/g, '{id}')
    .replace(/\{id\}/g, '{session_id}')
    .replace('/tools/{session_id}/execute', '/tools/{tool_name}/execute')
    .replace('/permissions/{session_id}', '/permissions/{permission_request_id}')
    .replace('/tasks/{session_id}/cancel', '/tasks/{task_id}/cancel')
    .replace('GET `/api/kernel/tasks/{session_id}`', 'GET `/api/kernel/tasks/{task_id}`');
}

function extractUiKernelPaths(source) {
  const matches = source.matchAll(/['"`]\/api\/kernel[^'"`]+['"`]/g);
  const paths = new Set();
  for (const match of matches) {
    const raw = match[0].slice(1, -1);
    const normalized = normalizeUiPath(raw)
      .replace('{session_id}', '{session_id}')
      .replace('{permissionRequestId}', '{permission_request_id}')
      .replace('{taskId}', '{task_id}')
      .replace('{toolName}', '{tool_name}');
    paths.add(normalized);
  }
  return paths;
}

test('kernel UI client and agent-server expose the same /api/kernel route surface', () => {
  assert.equal(fs.existsSync(UI_CLIENT_PATH), true);
  assert.equal(fs.existsSync(SERVER_APP_PATH), true);
  assert.equal(fs.existsSync(SERVER_HTTP_CONTRACTS_PATH), true);

  const uiSource = fs.readFileSync(UI_CLIENT_PATH, 'utf8');
  const serverSource = fs.readFileSync(SERVER_APP_PATH, 'utf8');

  for (const route of REQUIRED_KERNEL_ROUTES) {
    assert.match(
      serverSource,
      new RegExp(route.replace(/\{[^}]+\}/g, '\\{[^}]+\\}').replace(/\//g, '\\/')),
      `agent-server missing route: ${route}`
    );
  }

  assert.match(uiSource, /\/api\/kernel\/snapshot/);
  assert.match(uiSource, /\/api\/kernel\/sessions/);
  assert.match(uiSource, /\/api\/kernel\/sessions\/\$\{encodeURIComponent\(sessionId\)\}\/events\/stream/);
  assert.doesNotMatch(uiSource, /new EventSource\(/, 'SSE must use fetch so auth headers can be sent');
  assert.match(uiSource, /ReadableStream|getReader\(/);
});

test('agent-server HTTP surface spec documents kernel ingress security and readiness probes', () => {
  assert.equal(fs.existsSync(SERVER_SURFACE_PATH), true);
  const source = fs.readFileSync(SERVER_SURFACE_PATH, 'utf8');
  const contracts = fs.readFileSync(SERVER_HTTP_CONTRACTS_PATH, 'utf8');
  assert.match(source, /\/api\/kernel/);
  assert.match(source, /ingress/i);
  assert.match(source, /ready|health/i);
  assert.match(contracts, /ingress_token_auth_rejects_missing_credentials/);
  assert.match(contracts, /kernel_session_create_and_read_roundtrip/);
});

test('kernel UI session config uses string tenantId aligned with server metadata', () => {
  const typesPath = path.join(root, 'sdkwork-kernel-ui/packages/sdkwork-kernel-ui-types/src/index.ts');
  const typesSource = fs.readFileSync(typesPath, 'utf8');
  assert.match(typesSource, /tenantId\??: string/);
});
