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
const providerWorkerRelativePaths = [
  'generic-ts-sdk-worker.mjs',
  'engine-sdk-live.mjs',
  'codex-cli-live.mjs',
  'provider-cli-live.mjs',
];

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

function assertPathWithin(parentPath, childPath) {
  const relativePath = path.relative(path.resolve(parentPath), path.resolve(childPath));
  if (!relativePath || relativePath.startsWith('..') || path.isAbsolute(relativePath)) {
    throw new Error(`release staging path must stay within ${parentPath}: ${childPath}`);
  }
}

function copyRequiredFile(sourcePath, targetPath, label) {
  if (!fs.existsSync(sourcePath) || !fs.statSync(sourcePath).isFile()) {
    throw new Error(`Missing ${label}: ${sourcePath}`);
  }
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.copyFileSync(sourcePath, targetPath);
}

function stageReleasePayload(stagingDir) {
  assertPathWithin(outputDir, stagingDir);
  fs.rmSync(stagingDir, { recursive: true, force: true });
  fs.mkdirSync(stagingDir, { recursive: true });

  const stagedBinary = path.join(stagingDir, path.basename(binaryPath));
  copyRequiredFile(binaryPath, stagedBinary, 'release binary');

  const providerRuntimeDir = path.join(stagingDir, 'provider-runtime');
  const workerTargetDir = path.join(providerRuntimeDir, 'workers');
  for (const workerRelativePath of providerWorkerRelativePaths) {
    copyRequiredFile(
      path.join(kernelRoot, 'scripts', 'provider-transport-workers', workerRelativePath),
      path.join(workerTargetDir, workerRelativePath),
      `provider worker ${workerRelativePath}`,
    );
  }

  const nodeSource = process.execPath;
  const targetIsWindows = target.platform === 'windows';
  if (targetIsWindows !== (path.extname(nodeSource).toLowerCase() === '.exe')) {
    throw new Error(
      `Node runtime ${nodeSource} does not match release target platform ${target.platform}; package on the target runner.`,
    );
  }
  const nodeTarget = targetIsWindows
    ? path.join(providerRuntimeDir, 'node', 'node.exe')
    : path.join(providerRuntimeDir, 'node', 'bin', 'node');
  copyRequiredFile(nodeSource, nodeTarget, 'Node runtime');

  if (!targetIsWindows) {
    fs.chmodSync(stagedBinary, 0o755);
    fs.chmodSync(nodeTarget, 0o755);
  }

  fs.writeFileSync(
    path.join(providerRuntimeDir, 'manifest.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        nodeVersion: process.version,
        workers: providerWorkerRelativePaths.map((worker) => `workers/${worker}`),
      },
      null,
      2,
    )}\n`,
    'utf8',
  );
}

function createZipArchive(stagingDir) {
  const command = [
    'Compress-Archive',
    '-Path',
    quotePowerShellLiteral(path.join(stagingDir, '*')),
    '-DestinationPath',
    quotePowerShellLiteral(artifactPath),
    '-Force',
  ].join(' ');
  runChecked('powershell', ['-NoProfile', '-Command', command]);
}

function createTarGzArchive(stagingDir) {
  runChecked('tar', [
    '-czf',
    artifactPath,
    '-C',
    stagingDir,
    '.',
  ]);
}

const stagingDir = path.join(outputDir, `.package-staging-${packageId}`);
let packagingError = null;
try {
  stageReleasePayload(stagingDir);
  if (format === 'zip') {
    createZipArchive(stagingDir);
  } else if (format === 'tar.gz') {
    createTarGzArchive(stagingDir);
  } else {
    throw new Error(`unsupported kernel package format: ${format}`);
  }
} catch (error) {
  console.error(error.message);
  packagingError = error;
} finally {
  assertPathWithin(outputDir, stagingDir);
  fs.rmSync(stagingDir, { recursive: true, force: true });
}

if (packagingError) {
  process.exit(1);
}

console.log(`Release package written: ${artifactPath}`);
