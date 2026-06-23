#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const kernelRoot = process.cwd();
const packageId = process.env.SDKWORK_PACKAGE_ID ?? 'sdkwork-agent-server';
const version =
  process.env.SDKWORK_RELEASE_VERSION ??
  JSON.parse(fs.readFileSync(path.join(kernelRoot, 'sdkwork.app.config.json'), 'utf8')).release
    ?.currentVersion ??
  '0.0.0';
const outputDir = path.join(kernelRoot, 'dist', 'release', packageId);
const outputPath = path.join(outputDir, `${packageId}-${version}.cyclonedx.json`);

function runCargoCyclonedx() {
  const attempts = [
    ['cyclonedx-cargo', ['cyclonedx', '--format', 'json', '-p', 'sdkwork-agent-server', '-o', outputPath]],
    ['cyclonedx', ['cyclonedx', '--format', 'json', '-p', 'sdkwork-agent-server', '-o', outputPath]],
  ];
  for (const [subcommand, args] of attempts) {
    const result = spawnSync('cargo', [subcommand, ...args], {
      cwd: kernelRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    if (result.status === 0 && fs.existsSync(outputPath)) {
      return true;
    }
  }
  return false;
}

function metadataFallback() {
  const result = spawnSync(
    'cargo',
    ['metadata', '--format-version', '1', '--manifest-path', 'sdkwork-agent-server/Cargo.toml'],
    { cwd: kernelRoot, encoding: 'utf8' },
  );
  if (result.status !== 0) {
    console.error(result.stderr || result.stdout);
    process.exit(1);
  }
  const metadata = JSON.parse(result.stdout);
  const components = [];
  for (const pkg of metadata.packages) {
    components.push({
      type: 'library',
      'bom-ref': `pkg:cargo/${pkg.name}@${pkg.version}`,
      name: pkg.name,
      version: pkg.version,
      purl: `pkg:cargo/${pkg.name}@${pkg.version}`,
    });
  }
  const bom = {
    bomFormat: 'CycloneDX',
    specVersion: '1.5',
    version: 1,
    metadata: {
      timestamp: new Date().toISOString(),
      component: {
        type: 'application',
        'bom-ref': `pkg:cargo/${packageId}@${version}`,
        name: packageId,
        version,
      },
      tools: [
        {
          vendor: 'SDKWork',
          name: 'generate-kernel-sbom.mjs',
          version: '1.0.0',
        },
      ],
    },
    components,
  };
  fs.mkdirSync(outputDir, { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(bom, null, 2)}\n`, 'utf8');
}

fs.mkdirSync(outputDir, { recursive: true });

if (runCargoCyclonedx()) {
  console.log(`SBOM written: ${outputPath}`);
} else {
  console.warn('cargo cyclonedx plugin unavailable; emitting metadata-derived CycloneDX SBOM.');
  metadataFallback();
  console.log(`SBOM written: ${outputPath}`);
}

const digest = createHash('sha256').update(fs.readFileSync(outputPath)).digest('hex');
fs.writeFileSync(`${outputPath}.sha256`, `${digest}  ${path.basename(outputPath)}\n`, 'utf8');
