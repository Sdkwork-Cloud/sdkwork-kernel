import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const catalogRoot = path.join(root, 'bindings', 'agent-providers');

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
    { mode: 'official_sdk', package: '@google/gemini-cli' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  opencode: [
    { mode: 'official_sdk', package: '@opencode-ai/sdk' },
    { mode: 'http_openapi', transport: 'openclaw-gateway-open-api' },
    { mode: 'ipc_protocol', transport: 'jsonrpc_stdio' },
  ],
  'mimo-code': [
    { mode: 'source_tree', path: 'external/mimo-code' },
    { mode: 'official_sdk', package: '@mimo-ai/sdk' },
    { mode: 'http_openapi', transport: 'opencode-gateway-open-api' },
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

for (const entry of fs.readdirSync(catalogRoot, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const agent = entry.name;
  const manifestPath = path.join(catalogRoot, agent, 'provider-binding.manifest.json');
  if (!fs.existsSync(manifestPath)) continue;
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  manifest.manifest_type = 'agent_provider_binding';
  if (manifest.binding_id?.startsWith('binding.agent-sdk.')) {
    manifest.binding_id = manifest.binding_id.replace(
      'binding.agent-sdk.',
      'binding.agent-provider.'
    );
  }
  if (integrationByAgent[agent]) {
    manifest.integration_sources = integrationByAgent[agent];
  }
  if (manifest.description?.includes('SDK binding')) {
    manifest.description = manifest.description.replace(
      'External agent SDK binding',
      'External agent provider binding'
    );
  }
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`updated ${agent}`);
}
