#!/usr/bin/env node
/**
 * Bootstrap component.spec.json and README.md for agent runtime workspace crates.
 * Writes UTF-8 without BOM.
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const kernelRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const crates = [
  {
    name: 'sdkwork-agent-api-bridge',
    capability: 'agent-api-bridge',
    displayName: 'SDKWork Agent API Bridge',
    description:
      'Bridge layer connecting AgentKernel runtime contracts to AgentBusiness HTTP APIs.',
    publicExports: [
      'AgentRuntimeBridge',
      'ContextBridge',
      'EventBridge',
      'ModelBridge',
      'SessionBridge',
      'ToolBridge'
    ],
    events: []
  },
  {
    name: 'sdkwork-agent-client',
    capability: 'agent-client',
    displayName: 'SDKWork Agent Client',
    description:
      'Typed HTTP, SSE, and WebSocket clients plus bridge plugin registry for external agent runtimes.',
    publicExports: [
      'AgentClient',
      'AgentClientMode',
      'AgentBridgePlugin',
      'AgentBridgePluginRegistry',
      'AgentBridgeProvider',
      'ChatClient',
      'SseChatClient',
      'WebSocketChatClient'
    ],
    events: []
  },
  {
    name: 'sdkwork-agent-database',
    capability: 'agent-database',
    displayName: 'SDKWork Agent Database',
    description:
      'Repository traits and SQLite, PostgreSQL, and in-memory adapters for agent session persistence.',
    publicExports: [
      'AgentDatabase',
      'SessionRepository',
      'MessageRepository',
      'TaskRepository',
      'EventRepository',
      'SchemaManager',
      'InMemoryDatabase'
    ],
    events: []
  },
  {
    name: 'sdkwork-agent-server',
    capability: 'agent-server',
    displayName: 'SDKWork Agent Server',
    description:
      'Runnable Axum server binary with configuration, preflight checks, health endpoints, and chat APIs.',
    publicExports: ['sdkwork-agent-server'],
    runtimeEntrypoints: ['src/main.rs', 'Cargo.toml'],
    events: []
  },
  {
    name: 'sdkwork-agent-session',
    capability: 'agent-session',
    displayName: 'SDKWork Agent Session',
    description:
      'Unified session and conversation managers over database repository traits.',
    publicExports: ['UnifiedSessionManager', 'ConversationManager', 'SessionConfig'],
    events: []
  },
];

function canonicalSpecs() {
  return [
    {
      file: 'AGENT_KERNEL_SPEC.md',
      path: '../specs/AGENT_KERNEL_SPEC.md',
      purpose: 'Agent runtime, provider SPI, event, policy, and manifest authority.'
    },
    {
      file: 'AGENT_RUNTIME_SPEC.md',
      path: '../specs/AGENT_RUNTIME_SPEC.md',
      purpose: 'Runtime lifecycle, execution, and host integration rules.'
    },
    {
      file: 'COMPONENT_SPEC.md',
      path: '../../sdkwork-specs/COMPONENT_SPEC.md',
      purpose: 'Component-local contract and discovery rules.'
    },
    {
      file: 'CODE_STYLE_SPEC.md',
      path: '../../sdkwork-specs/CODE_STYLE_SPEC.md',
      purpose: 'Authored source structure and generated code boundaries.'
    },
    {
      file: 'NAMING_SPEC.md',
      path: '../../sdkwork-specs/NAMING_SPEC.md',
      purpose: 'Canonical SDKWork naming rules.'
    },
    {
      file: 'RUST_CODE_SPEC.md',
      path: '../../sdkwork-specs/RUST_CODE_SPEC.md',
      purpose: 'Rust crate and module rules.'
    },
    {
      file: 'TEST_SPEC.md',
      path: '../../sdkwork-specs/TEST_SPEC.md',
      purpose: 'Verification and contract testing expectations.'
    }
  ];
}

function buildSpec(crate) {
  return {
    schemaVersion: 1,
    kind: 'sdkwork.component.spec',
    component: {
      name: crate.name,
      displayName: crate.displayName,
      version: '0.1.0',
      type: 'rust-crate',
      root: `sdkwork-kernel/${crate.name}`,
      domain: 'intelligence',
      capability: crate.capability,
      surfaceNotRequiredReason:
        'This repository-internal Rust runtime crate exposes library or binary integration surfaces only and has no app, open-api, or backend-admin HTTP contract of its own.',
      languages: ['rust'],
      generated: false,
      manifests: ['Cargo.toml']
    },
    canonicalSpecs: canonicalSpecs(),
    contracts: {
      publicExports: crate.publicExports,
      runtimeEntrypoints: crate.runtimeEntrypoints ?? [],
      routeManifest: null,
      sdkDependencies: [],
      dependencyApiExports: [],
      dependencyApiSurfaces: [],
      sdkClients: [],
      events: crate.events,
      configKeys: []
    },
    verification: {
      commands: [`cargo test --manifest-path ${crate.name}/Cargo.toml`]
    }
  };
}

function buildReadme(crate) {
  return `# ${crate.displayName}

Domain: \`intelligence\`
Capability: \`${crate.capability}\`
Package type: Rust runtime crate

${crate.description}

## Verification

\`\`\`bash
cargo test --manifest-path ${crate.name}/Cargo.toml
\`\`\`

## Canonical Specifications

- Component spec: [\`specs/component.spec.json\`](specs/component.spec.json)
- Agent kernel spec: [\`../specs/AGENT_KERNEL_SPEC.md\`](../specs/AGENT_KERNEL_SPEC.md)
- Agent runtime spec: [\`../specs/AGENT_RUNTIME_SPEC.md\`](../specs/AGENT_RUNTIME_SPEC.md)
`;
}

for (const crate of crates) {
  const crateRoot = path.join(kernelRoot, crate.name);
  const specsDir = path.join(crateRoot, 'specs');
  fs.mkdirSync(specsDir, { recursive: true });

  const specPath = path.join(specsDir, 'component.spec.json');
  const readmePath = path.join(crateRoot, 'README.md');

  fs.writeFileSync(specPath, `${JSON.stringify(buildSpec(crate), null, 2)}\n`, 'utf8');
  if (!fs.existsSync(readmePath)) {
    fs.writeFileSync(readmePath, buildReadme(crate), 'utf8');
  }
  console.log(`wrote ${path.relative(kernelRoot, specPath)}`);
}
