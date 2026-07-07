import assert from 'node:assert/strict';
import process from 'node:process';

import {
  missingCredentialRequirements,
  runStagingLiveSdkGate,
} from './engine-sdk-live-staging.mjs';

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
