#!/usr/bin/env node

import fs from 'node:fs';
import process from 'node:process';
import {
  checksumPathFor,
  payloadFilesFromOutputGlobs,
  releaseDirFor,
  resolveLifecycleTarget,
  sha256File,
} from './kernel-release-targets.mjs';

const kernelRoot = process.cwd();
let targetContext;
try {
  targetContext = resolveLifecycleTarget({
    kernelRoot,
    commandName: 'generate-kernel-checksums.mjs',
  });
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const { packageId, target, version } = targetContext;
const outputDir = releaseDirFor(kernelRoot, packageId);
const payloads = payloadFilesFromOutputGlobs(kernelRoot, target.outputGlobs);
if (payloads.errors.length > 0 || payloads.files.length === 0) {
  for (const error of payloads.errors) {
    console.error(`CHECKSUM: ${error}`);
  }
  if (payloads.files.length === 0) {
    console.error(`CHECKSUM: no payload files found for ${packageId}`);
  }
  process.exit(1);
}

fs.mkdirSync(outputDir, { recursive: true });
const checksumFile = checksumPathFor(kernelRoot, packageId, version);
const lines = [];
for (const filePath of payloads.files) {
  lines.push(`${await sha256File(filePath)}  ${filePath.split(/[\\/]/u).at(-1)}`);
}
fs.writeFileSync(checksumFile, `${lines.join('\n')}\n`, 'utf8');
console.log(`Checksum written: ${checksumFile}`);
