import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const CODEX_PROVIDER_ROOT = path.join(
  'agent-providers',
  'crates',
  'sdkwork-agent-provider-codex'
);

const PRIVATE_STORAGE_PATTERNS = [
  [/state_\d+\.sqlite/iu, 'versioned Codex state SQLite file'],
  [/\bPRAGMA\b/iu, 'SQLite PRAGMA'],
  [/\bFROM\s+(?:threads|messages|turns|items)\b/iu, 'Codex private table query'],
  [/\brollout_path\b/iu, 'Codex rollout path'],
  [/\bread_codex_rollout\b/iu, 'Codex rollout reader'],
  [/\brusqlite\b/iu, 'direct SQLite dependency']
];

export function validateCodexSourceIntegration({ kernelRoot, errors, readFileIfExists }) {
  const rootCargo = readFileIfExists(path.join(kernelRoot, 'Cargo.toml'));
  const providerCargoPath = path.join(kernelRoot, CODEX_PROVIDER_ROOT, 'Cargo.toml');
  const providerCargo = readFileIfExists(providerCargoPath);
  const appServerSource = readFileIfExists(
    path.join(kernelRoot, CODEX_PROVIDER_ROOT, 'src', 'app_server.rs')
  );

  requireText(
    rootCargo,
    'codex-app-server-client = { path = "external/codex/codex-rs/app-server-client" }',
    'workspace must source codex-app-server-client from the pinned external Codex tree',
    errors
  );
  requireText(
    rootCargo,
    'codex-app-server-protocol = { path = "external/codex/codex-rs/app-server-protocol" }',
    'workspace must source codex-app-server-protocol from the pinned external Codex tree',
    errors
  );
  requireText(
    providerCargo,
    'codex-app-server-client.workspace = true',
    'Codex provider must consume the official app-server client facade',
    errors
  );
  requireText(
    providerCargo,
    'codex-app-server-protocol.workspace = true',
    'Codex provider must consume official typed app-server protocol models',
    errors
  );
  requireText(
    appServerSource,
    'session_source: SessionSource::AppServer.into()',
    'Codex in-process startup must use the typed AppServer session source',
    errors
  );

  for (const filePath of listProviderProductionFiles(kernelRoot)) {
    const content = readFileIfExists(filePath);
    for (const [pattern, label] of PRIVATE_STORAGE_PATTERNS) {
      if (pattern.test(content)) {
        errors.push(
          `${path.relative(kernelRoot, filePath)}: ${label} is forbidden; use the official typed Codex app-server facade`
        );
      }
    }
  }

  validateExternalDependencyOwnership(kernelRoot, errors, readFileIfExists);
  validatePinnedCleanSubmodule(kernelRoot, errors);
}

function listProviderProductionFiles(kernelRoot) {
  const providerRoot = path.join(kernelRoot, CODEX_PROVIDER_ROOT);
  const files = [path.join(providerRoot, 'Cargo.toml')];
  const sourceRoot = path.join(providerRoot, 'src');
  if (!fs.existsSync(sourceRoot)) {
    return files;
  }

  const pending = [sourceRoot];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(entryPath);
      } else if (entry.isFile() && entry.name.endsWith('.rs')) {
        files.push(entryPath);
      }
    }
  }
  return files;
}

function validateExternalDependencyOwnership(kernelRoot, errors, readFileIfExists) {
  for (const cargoPath of listCargoManifests(kernelRoot)) {
    const relativePath = path.relative(kernelRoot, cargoPath).replaceAll('\\', '/');
    if (relativePath === 'Cargo.toml') {
      continue;
    }
    if (/external\/codex/iu.test(readFileIfExists(cargoPath))) {
      errors.push(
        `${relativePath}: external Codex paths must be declared once at workspace root and consumed only by the L3 provider`
      );
    }
  }
}

function listCargoManifests(kernelRoot) {
  const manifests = [];
  const pending = [kernelRoot];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.name === '.git' || entry.name === 'external' || entry.name === 'node_modules' || entry.name === 'target') {
        continue;
      }
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(entryPath);
      } else if (entry.isFile() && entry.name === 'Cargo.toml') {
        manifests.push(entryPath);
      }
    }
  }
  return manifests;
}

function validatePinnedCleanSubmodule(kernelRoot, errors) {
  const gitlink = spawnSync('git', ['ls-files', '--stage', 'external/codex'], {
    cwd: kernelRoot,
    encoding: 'utf8'
  });
  if (gitlink.status !== 0 || !gitlink.stdout.startsWith('160000 ')) {
    errors.push('external/codex must be a pinned Git submodule (mode 160000)');
    return;
  }

  const externalRoot = path.join(kernelRoot, 'external', 'codex');
  if (!fs.existsSync(externalRoot)) {
    errors.push('external/codex submodule must be initialized for Codex source integration');
    return;
  }
  const status = spawnSync('git', ['status', '--short'], {
    cwd: externalRoot,
    encoding: 'utf8'
  });
  if (status.status !== 0) {
    errors.push(`unable to inspect external/codex cleanliness: ${status.stderr.trim()}`);
    return;
  }
  const dirty = status.stdout
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  const unexpected = dirty.filter((line) => !ALLOWED_CODEX_WORKAROUND_PATHS.has(line));
  if (unexpected.length > 0) {
    errors.push(`external/codex must remain read-only and clean: ${unexpected.join(', ')}`);
  }
}

/// Documented Windows toolchain workaround allowed inside the pinned Codex
/// submodule: pinning `tracing` to =0.1.41 / `tracing-subscriber` to 0.3.18
/// avoids nondeterministic rustc const-eval ICEs on Windows (shared with the
/// cloudrouter workspace). Any other modification fails the cleanliness gate.
const ALLOWED_CODEX_WORKAROUND_PATHS = new Set(['M codex-rs/Cargo.toml']);

function requireText(content, expected, message, errors) {
  if (!content.includes(expected)) {
    errors.push(message);
  }
}
