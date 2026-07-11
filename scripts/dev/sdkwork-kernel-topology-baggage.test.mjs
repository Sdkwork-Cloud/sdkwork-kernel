#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

const scanRoots = [
  'sdkwork-agent-server',
  'sdkwork-agent-kernel',
  'sdkwork-code-kernel',
  'sdkwork-kernel-plugins',
  'scripts',
  'configs',
  'deployments',
  'docs',
  'specs',
  'tests',
  'tools',
  'README.md',
  'AGENTS.md',
  'package.json',
];

const skipPathFragments = [
  '/target/',
  '/node_modules/',
  '/generated/',
  '/external/',
  'sdkwork-kernel-topology-baggage.test.mjs',
  'kernel_topology_alignment.test.mjs',
  'kernel_deployment_release.test.mjs',
  'docs/topology-standard.md',
  'configs/topology/',
  'docs/quality/',
  'docs/archive/',
  'docs/archive/superpowers/',
  'docs/superpowers/',
  'sdkwork-kernel-audit/',
];

const allowlistPathFragments = [
  'specs/topology.spec.json',
  'scripts/sdkwork-command.mjs',
  'scripts/kernel-dev.mjs',
  'docs/architecture/tech/TECH-sdkwork-standards-alignment-20260612.md',
  'docs/architecture/decisions/ADR-20260612-sdkwork-kernel-root-dictionary.md',
  'docs/architecture/decisions/ADR-20260626-agents-application-layer-separation.md',
  'docs/architecture/decisions/ADR-20260618-platform-framework-adoption.md',
  'docs/architecture/decisions/ADR-20260612-agent-implementation-type.md',
  'docs/architecture/tech/TECH-2026-06-12-agent-implementation-type.md',
  'docs/architecture/tech/TECH-2026-06-04-rig-agent-provider-deployments.md',
  'docs/architecture/tech/TECH-2026-06-04-rig-complete-plugin-design.md',
  'docs/architecture/tech/TECH-2026-06-04-rig-complete-plugin.md',
  'docs/architecture/tech/TECH-2026-06-12-sdkwork-specs-structure-hardening-design.md',
  'tools/validators/kernel-standards/platform-integration.mjs',
  'tools/validators/kernel-standards/workspace-evidence.mjs',
  'tests/kernel_workspace_structure.test.mjs',
  'scripts/dev/sdkwork-kernel-utils-standard.test.mjs',
];

const bannedPatterns = [
  { id: 'topology v1 env key', pattern: /SDKWORK_KERNEL_TOPOLOGY/u },
  { id: 'client topology v1 env key', pattern: /VITE_KERNEL_TOPOLOGY/u },
  { id: 'retired hosting env key', pattern: /SDKWORK_KERNEL_HOSTING/u },
  { id: 'retired split bind env key', pattern: /SDKWORK_BIND_ADDRESS/u },
  { id: 'retired mock fallback env key', pattern: /SDKWORK_KERNEL_ALLOW_MOCK_FALLBACK/u },
  { id: 'topology CLI flag', pattern: /--topology\b/u },
  { id: 'public process layout CLI flag', pattern: /--service-layout\b/u },
  { id: 'retired split-services profile segment', pattern: /\bsplit-services\b/u },
  { id: 'retired unified-process profile segment', pattern: /\bunified-process\b/u },
  {
    id: 'hardcoded application ingress url',
    pattern: /http:\/\/127\.0\.0\.1:18280/u,
  },
  {
    id: 'removed in-repo kernel ui workspace',
    pattern: /sdkwork-kernel-ui/u,
  },
  {
    id: 'removed in-repo managed agent business crate',
    pattern: /sdkwork-agent-business/u,
  },
];

const surfaceUrlKeys = [
  'SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL',
  'VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL',
  'VITE_SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL',
];

function slash(value) {
  return String(value).replaceAll('\\', '/');
}

function shouldSkip(relativePath) {
  const normalized = slash(relativePath);
  return skipPathFragments.some((fragment) => normalized.includes(fragment));
}

function isAllowlisted(relativePath) {
  const normalized = slash(relativePath);
  return allowlistPathFragments.some((fragment) => normalized.endsWith(fragment));
}

function isTextCandidate(relativePath) {
  return /\.(?:md|mjs|json|yml|yaml|toml|rs|ts|tsx|env)$/u.test(relativePath);
}

function collectFiles(relativeRoot) {
  const absoluteRoot = path.join(repoRoot, relativeRoot);
  if (!fs.existsSync(absoluteRoot)) {
    return [];
  }
  const stat = fs.statSync(absoluteRoot);
  if (stat.isFile()) {
    return [relativeRoot];
  }
  const files = [];
  for (const entry of fs.readdirSync(absoluteRoot, { withFileTypes: true })) {
    const relativePath = path.join(relativeRoot, entry.name);
    if (shouldSkip(relativePath)) {
      continue;
    }
    if (entry.isDirectory()) {
      files.push(...collectFiles(relativePath));
      continue;
    }
    if (isTextCandidate(relativePath)) {
      files.push(relativePath);
    }
  }
  return files;
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

const files = scanRoots.flatMap((root) => collectFiles(root));

for (const { id, pattern } of bannedPatterns) {
  const hits = [];
  for (const relativePath of files) {
    if (isAllowlisted(relativePath)) {
      continue;
    }
    const text = readText(relativePath);
    if (pattern.test(text)) {
      hits.push(relativePath);
    }
  }
  assert.equal(
    hits.length,
    0,
    `topology baggage (${id}) found in active paths: ${hits.join(', ')}`,
  );
}

const spec = JSON.parse(readText('specs/topology.spec.json'));
assert.equal(spec.schemaVersion, 4);
assert.equal(spec.archetype, 'realtime-application-platform');
assert.equal(spec.defaults.developmentProfileId, 'standalone.development');
assert.equal(spec.defaults.productionProfileId, 'cloud.production');
assert.equal(spec.vocabulary?.serviceLayout, undefined);
for (const profileId of Object.keys(spec.profileFiles ?? {})) {
  assert.equal(profileId.split('.').length, 2, `${profileId} must be deploymentProfile.environment`);
}

const profileDir = path.join(repoRoot, 'configs/topology');
const profileFiles = fs.readdirSync(profileDir).filter((name) => name.endsWith('.env'));
assert.ok(profileFiles.length >= 4, 'topology profile env files required');

const packageJson = JSON.parse(readText('package.json'));
assert.match(
  JSON.stringify(packageJson.dependencies ?? {}),
  /"@sdkwork\/app-topology"/u,
  'package.json must depend on @sdkwork/app-topology',
);
assert.match(
  JSON.stringify(packageJson.scripts ?? {}),
  /"dev":/u,
  'package.json must expose standard dev script',
);

for (const profileFile of profileFiles) {
  const profileText = readText(path.join('configs/topology', profileFile));
  for (const key of surfaceUrlKeys) {
    const match = profileText.match(new RegExp(`^${key}=(.+)$`, 'mu'));
    if (!match) {
      continue;
    }
    const value = match[1].trim();
    assert.match(
      value,
      /^https?:\/\/[^/]+(?::\d+)?$/u,
      `${profileFile} ${key} must be an ingress origin without API path prefix; got ${value}`,
    );
  }
}

console.log('[sdkwork-kernel-topology-baggage] ok');
