#!/usr/bin/env node

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const kernelRoot = process.cwd();
const packageId = process.env.SDKWORK_PACKAGE_ID ?? 'sdkwork-agent-server';
const version = process.env.SDKWORK_RELEASE_VERSION ?? '0.1.0';
const outputDir = path.join(kernelRoot, 'dist', 'release', packageId);
const suffix = process.platform === 'win32' ? '.exe' : '';
const binaryPath = path.join(kernelRoot, 'target', 'release', `${packageId}${suffix}`);

if (!fs.existsSync(binaryPath)) {
  console.error(`Missing release binary: ${binaryPath}`);
  process.exit(1);
}

fs.mkdirSync(outputDir, { recursive: true });
const digest = createHash('sha256').update(fs.readFileSync(binaryPath)).digest('hex');
const checksumFile = path.join(outputDir, `${packageId}-${version}.sha256`);
const line = `${digest}  ${path.basename(binaryPath)}\n`;
fs.writeFileSync(checksumFile, line, 'utf8');
console.log(`Checksum written: ${checksumFile}`);
