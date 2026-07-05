#!/usr/bin/env node

import { existsSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const removablePaths = ['target'];

for (const relativePath of removablePaths) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!existsSync(absolutePath)) {
    continue;
  }
  rmSync(absolutePath, { recursive: true, force: true });
  process.stdout.write(`removed ${relativePath}\n`);
}

process.stdout.write('sdkwork-kernel clean complete\n');
