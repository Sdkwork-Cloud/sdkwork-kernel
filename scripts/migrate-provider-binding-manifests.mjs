import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');
const catalogRoot = path.join(root, 'bindings', 'agent-providers');
const scriptPath = fileURLToPath(import.meta.url);

const integrationByAgent = {
  codex: [
    { mode: 'official_sdk', package: '@openai/codex-sdk' },
    { mode: 'rust_crate', crate: 'codex-core' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  'claude-code': [
    { mode: 'official_sdk', package: '@anthropic-ai/claude-agent-sdk' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  'gemini-cli': [
    { mode: 'source_tree', path: 'external/gemini-cli/packages/sdk' },
    { mode: 'npm_package', package: '@google/gemini-cli' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  opencode: [
    { mode: 'official_sdk', package: '@opencode-ai/sdk' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  'mimo-code': [
    { mode: 'source_tree', path: 'external/mimo-code/packages/sdk/js' },
    { mode: 'official_sdk', package: '@mimo-ai/sdk' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  openclaw: [
    { mode: 'official_sdk', package: 'openclaw' },
    { mode: 'http_openapi', transport: 'openclaw-gateway-open-api' },
  ],
  hermes: [
    { mode: 'python_module', module: 'run_agent' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
};

const providerLocalCapabilities = new Set([
  'sdk.session.lifecycle',
  'sdk.session.history',
]);

const runtimeOperationsByCapability = new Map([
  [
    'sdk.session.control',
    ['ping', 'session_interrupt', 'session_compact', 'session_fork'],
  ],
  ['sdk.model.chat', ['ping', 'model_chat', 'model_chat_stream']],
  ['sdk.model.stream', ['ping', 'model_chat_stream']],
  ['sdk.tool.invoke', ['ping', 'tool_invoke']],
  ['sdk.skill.invoke', ['ping', 'skill_invoke']],
]);

const unsupportedRustRuntimeOperations = new Set([
  'session_create',
  'session_interrupt',
  'session_compact',
  'session_fork',
  'skill_invoke',
]);

function defaultExecutionScopeForCapability(capabilityId) {
  if (providerLocalCapabilities.has(capabilityId)) {
    return 'provider_local';
  }
  return 'transport_runtime';
}

function defaultRuntimeOperationsForCapability(capabilityId) {
  if (providerLocalCapabilities.has(capabilityId)) {
    return ['ping'];
  }
  return runtimeOperationsByCapability.get(capabilityId) ?? ['ping'];
}

function uniqueOperations(operations) {
  const seen = new Set();
  const unique = [];
  for (const operation of operations) {
    if (seen.has(operation)) {
      continue;
    }
    seen.add(operation);
    unique.push(operation);
  }
  return unique;
}

function normalizeRuntimeOperations(capability, backend) {
  const declaredOperations = Array.isArray(backend.runtime_operations)
    ? backend.runtime_operations
    : defaultRuntimeOperationsForCapability(capability.capability_id);
  const seededOperations = declaredOperations.includes('ping')
    ? declaredOperations
    : ['ping', ...declaredOperations];

  if (capability.execution_scope === 'provider_local') {
    return ['ping'];
  }

  const supportedOperations = backend.kind === 'rust_native'
    ? seededOperations.filter((operation) => !unsupportedRustRuntimeOperations.has(operation))
    : seededOperations;

  return uniqueOperations(supportedOperations.length > 0 ? supportedOperations : ['ping']);
}

export function migrateManifest(agent, manifest) {
  const migrated = JSON.parse(JSON.stringify(manifest));
  migrated.manifest_type = 'agent_provider_binding';
  if (migrated.binding_id?.startsWith('binding.agent-sdk.')) {
    migrated.binding_id = migrated.binding_id.replace(
      'binding.agent-sdk.',
      'binding.'
    );
  }
  if (integrationByAgent[agent]) {
    migrated.integration_sources = integrationByAgent[agent];
  }
  if (migrated.description?.includes('SDK binding')) {
    migrated.description = migrated.description.replace(
      'External agent SDK binding',
      'External agent provider binding'
    );
  }

  for (const capability of migrated.capabilities ?? []) {
    capability.execution_scope = capability.execution_scope ??
      defaultExecutionScopeForCapability(capability.capability_id);
    for (const backend of capability.backends ?? []) {
      backend.runtime_operations = normalizeRuntimeOperations(capability, backend);
    }
  }

  return migrated;
}

export function runProviderBindingManifestMigration() {
  for (const entry of fs.readdirSync(catalogRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const agent = entry.name;
    const manifestPath = path.join(catalogRoot, agent, 'provider-binding.manifest.json');
    if (!fs.existsSync(manifestPath)) continue;
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    const migrated = migrateManifest(agent, manifest);
    fs.writeFileSync(manifestPath, `${JSON.stringify(migrated, null, 2)}\n`);
    console.log(`updated ${agent}`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  runProviderBindingManifestMigration();
}
