#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

const rootCargo = read('Cargo.toml');
assert.match(
  rootCargo,
  /sdkwork-utils-rust\s*=\s*\{[^}]*sdkwork-utils/u,
  'Cargo.toml must declare sdkwork-utils-rust workspace dependency'
);

const agentBusinessCargo = read('sdkwork-agent-business/Cargo.toml');
assert.match(
  agentBusinessCargo,
  /sdkwork-utils-rust\.workspace\s*=\s*true/u,
  'sdkwork-agent-business must consume sdkwork-utils-rust from the workspace'
);

const workflow = JSON.parse(read('sdkwork.workflow.json'));
const dependencyIds = new Set((workflow.dependencies || []).map((dependency) => dependency.id));
assert(dependencyIds.has('sdkwork-utils'), 'sdkwork.workflow.json must declare sdkwork-utils sibling checkout');

const validationSource = read('sdkwork-agent-business/src/validation.rs');
assert.match(
  validationSource,
  /sdkwork_utils_rust/u,
  'validation.rs must consume sdkwork-utils-rust instead of ad hoc blank checks'
);

const uiWorkspace = read('sdkwork-kernel-ui/pnpm-workspace.yaml');
assert.match(
  uiWorkspace,
  /sdkwork-utils\/packages\/sdkwork-utils-typescript/u,
  'sdkwork-kernel-ui/pnpm-workspace.yaml must include sdkwork-utils-typescript sibling package'
);

const uiPackage = JSON.parse(read('sdkwork-kernel-ui/package.json'));
assert(
  uiPackage.dependencies?.['@sdkwork/utils-typescript'],
  'sdkwork-kernel-ui must depend on @sdkwork/utils-typescript for shared utility standardization'
);

const sessionPanel = read('sdkwork-kernel-ui/src/KernelUiSessionPanel.tsx');
assert.match(
  sessionPanel,
  /@sdkwork\/utils-typescript/u,
  'KernelUiSessionPanel must consume @sdkwork/utils-typescript helpers'
);

const packageWorkflow = read('.github/workflows/package.yml');
assert.match(packageWorkflow, /SDKWORK_UTILS_REF/u, 'package workflow must expose SDKWORK_UTILS_REF');

process.stdout.write('sdkwork-kernel utils standard passed\n');
