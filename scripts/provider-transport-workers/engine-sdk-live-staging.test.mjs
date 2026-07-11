import assert from 'node:assert/strict';
import process from 'node:process';

import {
  frameworkRequiresPackageResolution,
  missingCredentialRequirements,
  runStagingLiveSdkGate,
} from './engine-sdk-live-staging.mjs';

const stagingModule = await import('./engine-sdk-live-staging.mjs');

assert.equal(
  typeof stagingModule.supportedStagingFrameworks,
  'function',
  'staging live gate should expose its framework coverage as a testable contract',
);
assert.deepEqual(
  stagingModule.supportedStagingFrameworks(),
  ['codex', 'claude', 'gemini', 'opencode', 'openclaw'],
);
assert.equal(
  stagingModule.supportedStagingFrameworks().includes('hermes'),
  false,
  'Hermes requires a separate Python/TUI gateway staging proof',
);

assert.deepEqual(
  missingCredentialRequirements('gemini', { GEMINI_API_KEY: 'live' }),
  [],
);
assert.deepEqual(
  missingCredentialRequirements('gemini', { GOOGLE_API_KEY: 'live' }),
  [],
);
assert.deepEqual(
  missingCredentialRequirements('gemini', {}),
  ['GEMINI_API_KEY or GOOGLE_API_KEY'],
);
assert.deepEqual(
  missingCredentialRequirements('openclaw', { OPENCLAW_GATEWAY_TOKEN: 'live' }),
  ['OPENCLAW_GATEWAY_URL'],
);
assert.deepEqual(
  missingCredentialRequirements('openclaw', { OPENCLAW_GATEWAY_URL: 'http://127.0.0.1:43190' }),
  [],
  'OpenClaw gateway token is optional; staging preflight must not reject an unauthenticated private gateway',
);
assert.equal(
  frameworkRequiresPackageResolution('codex'),
  true,
  'Codex staging proof must use an importable official SDK package',
);
assert.equal(
  frameworkRequiresPackageResolution('openclaw'),
  false,
  'OpenClaw staging proof uses the gateway HTTP authority and must not require a local npm package import',
);

delete process.env.SDKWORK_KERNEL_STAGING_LIVE_SDK;
const skipped = await runStagingLiveSdkGate({ framework: 'codex' });
assert.equal(skipped.status, 'skipped');
assert.equal(skipped.reason, 'flag-disabled');

process.env.SDKWORK_KERNEL_STAGING_LIVE_SDK = '1';
process.env.SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS = '0';
delete process.env.OPENAI_API_KEY;
delete process.env.ANTHROPIC_API_KEY;

const missingCreds = await runStagingLiveSdkGate({ framework: 'codex' });
assert.equal(missingCreds.status, 'skipped');
assert.equal(missingCreds.reason, 'missing-credentials');

console.log('engine-sdk-live-staging contract passed.');
