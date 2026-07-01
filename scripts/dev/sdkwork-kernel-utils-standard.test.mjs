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

const agentDatabaseCargo = read('sdkwork-agent-database/Cargo.toml');
assert.match(
  agentDatabaseCargo,
  /sdkwork-utils-rust/u,
  'sdkwork-agent-database must declare sdkwork-utils-rust for postgres-sync alignment'
);

const workflow = JSON.parse(read('sdkwork.workflow.json'));
const dependencyIds = new Set((workflow.dependencies || []).map((dependency) => dependency.id));
assert(dependencyIds.has('sdkwork-utils'), 'sdkwork.workflow.json must declare sdkwork-utils sibling checkout');

const postgresPoolSource = read('sdkwork-agent-database/src/postgres_pool.rs');
assert.match(
  postgresPoolSource,
  /create_pool_from_config/u,
  'postgres_pool.rs must bootstrap pools through sdkwork-database-sqlx'
);

const uiWorkspace = read('pnpm-workspace.yaml');
assert.match(
  uiWorkspace,
  /sdkwork-utils\/packages\/sdkwork-utils-typescript/u,
  'pnpm-workspace.yaml must include sdkwork-utils-typescript sibling package'
);

const uiPackage = JSON.parse(read('sdkwork-kernel-ui/package.json'));
assert(
  uiPackage.dependencies?.['@sdkwork/utils'],
  'sdkwork-kernel-ui must depend on @sdkwork/utils for shared utility standardization'
);

const sessionPanel = read('sdkwork-kernel-ui/src/KernelUiSessionPanel.tsx');
assert.match(
  sessionPanel,
  /@sdkwork\/utils/u,
  'KernelUiSessionPanel must consume @sdkwork/utils helpers'
);

const packageWorkflow = read('.github/workflows/package.yml');
assert.match(packageWorkflow, /SDKWORK_UTILS_REF/u, 'package workflow must expose SDKWORK_UTILS_REF');

process.stdout.write('sdkwork-kernel utils standard passed\n');
