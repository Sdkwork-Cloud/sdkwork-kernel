import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  buildHermesGatewaySpawnOptions,
  hermesGatewayStagingEnabled,
  runHermesGatewayStagingProof,
} from './hermes-gateway-staging.mjs';

assert.equal(hermesGatewayStagingEnabled({}), false);
assert.equal(hermesGatewayStagingEnabled({ SDKWORK_KERNEL_STAGING_HERMES_GATEWAY: '1' }), true);
assert.equal(
  hermesGatewayStagingEnabled({ SDKWORK_KERNEL_STAGING_HERMES_GATEWAY: 'true' }),
  true,
);

const skipped = await runHermesGatewayStagingProof({ env: {} });
assert.equal(skipped.status, 'skipped');
assert.equal(skipped.reason, 'flag-disabled');

const missingRoot = path.join(os.tmpdir(), 'sdkwork-missing-hermes-agent');
await assert.rejects(
  () =>
    runHermesGatewayStagingProof({
      env: { SDKWORK_KERNEL_STAGING_HERMES_GATEWAY: '1' },
      hermesRoot: missingRoot,
    }),
  /missing Hermes Agent gateway source/,
);

const fixtureRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-hermes-gateway-'));
fs.mkdirSync(path.join(fixtureRoot, 'tui_gateway'), { recursive: true });
fs.writeFileSync(path.join(fixtureRoot, 'tui_gateway', 'entry.py'), '', 'utf8');
const spawnOptions = buildHermesGatewaySpawnOptions({
  env: {},
  hermesRoot: fixtureRoot,
  hermesHome: path.join(fixtureRoot, '.hermes-home'),
});

assert.equal(spawnOptions.command, 'python');
assert.deepEqual(spawnOptions.args, ['-u', '-m', 'tui_gateway.entry']);
assert.equal(spawnOptions.options.cwd, fixtureRoot);
assert.match(spawnOptions.options.env.PYTHONPATH, /sdkwork-hermes-gateway-/);
assert.equal(spawnOptions.options.env.HERMES_HOME, path.join(fixtureRoot, '.hermes-home'));

console.log('hermes-gateway-staging contract passed.');
