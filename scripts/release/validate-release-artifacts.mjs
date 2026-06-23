#!/usr/bin/env node

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
const suffix = process.platform === 'win32' ? '.exe' : '';
const binaryPath = path.join(kernelRoot, 'target', 'release', `${packageId}${suffix}`);
const releaseDir = path.join(kernelRoot, 'dist', 'release', packageId);
const sbomPath = path.join(releaseDir, `${packageId}-${version}.cyclonedx.json`);
const checksumPath = path.join(releaseDir, `${packageId}-${version}.sha256`);

const errors = [];

if (!fs.existsSync(binaryPath)) {
  errors.push(`missing release binary: ${binaryPath}`);
}
if (!fs.existsSync(sbomPath)) {
  errors.push(`missing SBOM: ${sbomPath}`);
}
if (!fs.existsSync(checksumPath)) {
  errors.push(`missing checksum file: ${checksumPath}`);
}

const manifest = JSON.parse(fs.readFileSync(path.join(kernelRoot, 'sdkwork.app.config.json'), 'utf8'));
if (manifest.security?.sbomRequired && !fs.existsSync(sbomPath)) {
  errors.push('sdkwork.app.config.json requires SBOM evidence');
}
if (manifest.security?.checksumRequired && !fs.existsSync(checksumPath)) {
  errors.push('sdkwork.app.config.json requires checksum evidence');
}

if (errors.length > 0) {
  for (const error of errors) {
    console.error(`RELEASE VALIDATION: ${error}`);
  }
  process.exit(1);
}

console.log('Release artifact validation passed.');
