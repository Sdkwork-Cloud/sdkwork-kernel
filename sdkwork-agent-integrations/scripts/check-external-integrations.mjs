import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(import.meta.dirname, '..', '..');
const testPath = path.join(
  root,
  'sdkwork-agent-integrations',
  'tests',
  'external_integration_structure.test.mjs'
);

const result = spawnSync(process.execPath, ['--test', testPath], {
  cwd: root,
  stdio: 'inherit'
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

console.log('External agent integration structure check passed.');
