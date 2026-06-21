#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const REQUIRED_ROOT_SCRIPTS = ['dev', 'build', 'test', 'check', 'verify', 'clean'];
const REQUIRED_CAPABILITY_SCRIPTS = ['topology:validate', 'api:check', 'sdk:check'];
const RETIRED_COMMAND_PATTERNS = [
  [/--hosting\b/u, '--hosting'],
  [/\bself-hosted\b/u, 'self-hosted'],
  [/\bcloud-hosted\b/u, 'cloud-hosted'],
];
const PACKAGE_SCAN_ROOTS = ['sdkwork-kernel-ui'];
const DOC_SCAN_ROOTS = ['README.md', 'AGENTS.md', 'docs', 'configs', 'deployments', 'specs'];
const IGNORED_DOC_PARTS = ['docs/superpowers', 'docs/quality', 'docs/review'];

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), 'utf8'));
}

function fail(message, details = []) {
  console.error(`kernel pnpm script standard failed: ${message}`);
  for (const detail of details) {
    console.error(`- ${detail}`);
  }
  process.exit(1);
}

function collectPackageJsonFiles(relativeRoot) {
  const absoluteRoot = path.join(repoRoot, relativeRoot);
  if (!fs.existsSync(absoluteRoot)) {
    return [];
  }

  const files = [];
  function walk(current) {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name === 'generated') {
          continue;
        }
        walk(entryPath);
        continue;
      }
      if (entry.name === 'package.json') {
        files.push(path.relative(repoRoot, entryPath));
      }
    }
  }

  walk(absoluteRoot);
  return files;
}

function collectDocFiles(relativeRoot) {
  const absoluteRoot = path.join(repoRoot, relativeRoot);
  if (!fs.existsSync(absoluteRoot)) {
    return [];
  }
  const stat = fs.statSync(absoluteRoot);
  if (stat.isFile()) {
    return [relativeRoot];
  }

  const files = [];
  function walk(current, relative) {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const entryRelative = path.join(relative, entry.name).replaceAll('\\', '/');
      if (entry.isDirectory()) {
        if (IGNORED_DOC_PARTS.some((part) => entryRelative.startsWith(part))) {
          continue;
        }
        walk(path.join(current, entry.name), entryRelative);
        continue;
      }
      if (/\.(?:md|mdx)$/iu.test(entry.name)) {
        files.push(entryRelative);
      }
    }
  }

  walk(absoluteRoot, relativeRoot.replaceAll('\\', '/'));
  return files;
}

const issues = [];
const scripts = readJson('package.json').scripts ?? {};

for (const scriptName of REQUIRED_ROOT_SCRIPTS) {
  if (!scripts[scriptName]) {
    issues.push(`missing required root script "${scriptName}"`);
  }
}

for (const scriptName of REQUIRED_CAPABILITY_SCRIPTS) {
  if (!scripts[scriptName]) {
    issues.push(`missing capability script "${scriptName}"`);
  }
}

for (const [scriptName, commandText] of Object.entries(scripts)) {
  if (scriptName.startsWith('kernel:')) {
    issues.push(`retired application-code-prefixed script "${scriptName}"`);
  }
  for (const [pattern, token] of RETIRED_COMMAND_PATTERNS) {
    if (pattern.test(String(commandText))) {
      issues.push(`script "${scriptName}" uses retired deployment token "${token}"`);
    }
  }
}

if (!String(scripts.dev ?? '').includes('scripts/sdkwork-command.mjs')) {
  issues.push('scripts.dev must dispatch through scripts/sdkwork-command.mjs');
}

for (const relativePath of ['scripts/sdkwork-command.mjs', 'scripts/clean-workspace.mjs']) {
  if (!fs.existsSync(path.join(repoRoot, relativePath))) {
    issues.push(`missing ${relativePath}`);
  }
}

for (const scanRoot of PACKAGE_SCAN_ROOTS) {
  for (const packagePath of collectPackageJsonFiles(scanRoot)) {
    const packageScripts = readJson(packagePath).scripts ?? {};
    for (const scriptName of Object.keys(packageScripts)) {
      if (scriptName.startsWith('kernel:')) {
        issues.push(`${packagePath}#${scriptName}: application-code-prefixed script is forbidden`);
      }
    }
  }
}

for (const scanRoot of DOC_SCAN_ROOTS) {
  for (const docPath of collectDocFiles(scanRoot)) {
    const text = fs.readFileSync(path.join(repoRoot, docPath), 'utf8');
    if (/\bpnpm\s+kernel:/u.test(text)) {
      issues.push(`${docPath} documents retired pnpm kernel:* command`);
    }
    if (/\b--hosting\b/u.test(text)) {
      issues.push(`${docPath} documents retired --hosting flag`);
    }
  }
}

if (issues.length > 0) {
  fail('kernel PNPM script surface is not compliant', issues);
}

console.log('kernel pnpm script standard ok');
