#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import {
  releaseArtifactPathFor,
  releaseBinaryPathFor,
  releaseDirFor,
  resolveLifecycleTarget,
} from './kernel-release-targets.mjs';

const kernelRoot = process.cwd();
let targetContext;
try {
  targetContext = resolveLifecycleTarget({
    kernelRoot,
    commandName: 'package-kernel-artifact.mjs',
  });
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const { packageId, target, workflow, version } = targetContext;
const format = target.formats?.[0];
const binaryPath = process.env.SDKWORK_KERNEL_RELEASE_BINARY?.trim()
  ? path.resolve(kernelRoot, process.env.SDKWORK_KERNEL_RELEASE_BINARY.trim())
  : releaseBinaryPathFor(kernelRoot, target.platform);

if (!fs.existsSync(binaryPath)) {
  console.error(`Missing release binary for ${packageId}: ${binaryPath}`);
  process.exit(1);
}

const outputDir = releaseDirFor(kernelRoot, packageId);
const artifactPath = releaseArtifactPathFor({
  kernelRoot,
  workflow,
  packageId,
  version,
  format,
});
fs.mkdirSync(outputDir, { recursive: true });
if (fs.existsSync(artifactPath)) {
  fs.rmSync(artifactPath, { force: true });
}

function runChecked(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? kernelRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.status !== 0) {
    const detail = `${result.stdout ?? ''}${result.stderr ?? ''}`.trim();
    throw new Error(`${command} ${args.join(' ')} failed${detail ? `: ${detail}` : ''}`);
  }
}

function quotePowerShellLiteral(value) {
  return `'${String(value).replace(/'/gu, "''")}'`;
}

function createZipArchive() {
  const command = [
    'Compress-Archive',
    '-LiteralPath',
    quotePowerShellLiteral(binaryPath),
    '-DestinationPath',
    quotePowerShellLiteral(artifactPath),
    '-Force',
  ].join(' ');
  runChecked('powershell', ['-NoProfile', '-Command', command]);
}

function createTarGzArchive() {
  runChecked('tar', [
    '-czf',
    artifactPath,
    '-C',
    path.dirname(binaryPath),
    path.basename(binaryPath),
  ]);
}

try {
  if (format === 'zip') {
    createZipArchive();
  } else if (format === 'tar.gz') {
    createTarGzArchive();
  } else {
    throw new Error(`unsupported kernel package format: ${format}`);
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

console.log(`Release package written: ${artifactPath}`);
