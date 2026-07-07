#!/usr/bin/env node
/**
 * Opt-in staging live SDK invoke gate (REQ-2026-0001 / G-13).
 *
 * Default merge pipeline stays credential-free via engine-sdk-live.test.mjs.
 * Run this script from workflow_dispatch staging CI or locally with:
 *   SDKWORK_KERNEL_STAGING_LIVE_SDK=1 node scripts/provider-transport-workers/engine-sdk-live-staging.mjs
 */

import assert from 'node:assert/strict';
import process from 'node:process';

import {
  invokeModelChatLive,
  mockProviderInvocationAllowed,
  resolvePackageSpecifier,
} from './engine-sdk-live.mjs';

const STAGING_FLAG_ENV = 'SDKWORK_KERNEL_STAGING_LIVE_SDK';
const REQUIRE_CREDENTIALS_ENV = 'SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS';

const FRAMEWORKS = {
  codex: {
    packageName: '@openai/codex-sdk',
    credentialRequirements: [['OPENAI_API_KEY']],
  },
  claude: {
    packageName: '@anthropic-ai/claude-agent-sdk',
    credentialRequirements: [['ANTHROPIC_API_KEY']],
  },
  gemini: {
    packageName: '@google/gemini-cli-sdk',
    credentialRequirements: [['GEMINI_API_KEY', 'GOOGLE_API_KEY']],
  },
  opencode: {
    packageName: '@opencode-ai/sdk',
    credentialRequirements: [['OPENCODE_API_KEY', 'OPENAI_API_KEY']],
  },
  openclaw: {
    packageName: 'openclaw',
    credentialRequirements: [['OPENCLAW_GATEWAY_TOKEN'], ['OPENCLAW_GATEWAY_URL']],
  },
};

function parseFrameworkArg(argv) {
  const index = argv.indexOf('--framework');
  if (index === -1) {
    return 'codex';
  }
  return argv[index + 1] ?? 'codex';
}

function stagingEnabled() {
  const value = process.env[STAGING_FLAG_ENV]?.trim().toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function credentialsRequired() {
  const value = process.env[REQUIRE_CREDENTIALS_ENV]?.trim().toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

export function missingCredentialRequirements(framework, env = process.env) {
  const config = FRAMEWORKS[framework];
  if (!config) {
    throw new Error(`unknown framework: ${framework}`);
  }
  return config.credentialRequirements
    .filter((group) => !group.some((name) => env[name]?.trim()))
    .map((group) => group.join(' or '));
}

async function runFrameworkLiveInvoke(framework) {
  const config = FRAMEWORKS[framework];
  assert.ok(config, `unsupported framework: ${framework}`);
  assert.ok(
    resolvePackageSpecifier(config.packageName),
    `${config.packageName} should resolve in staging workspace`,
  );

  const result = await invokeModelChatLive(config.packageName, {
    model_request_id: `staging-${framework}`,
    messages: ['Reply with exactly: SDKWORK_STAGING_OK'],
    wire_messages: [
      {
        role: 'user',
        content: [{ type: 'text', text: 'Reply with exactly: SDKWORK_STAGING_OK' }],
      },
    ],
  });

  assert.equal(result.ok, true, `${framework} live invoke should succeed`);
  assert.equal(result.mode, 'sdk_live', `${framework} should use sdk_live mode`);
  assert.ok(Array.isArray(result.messages), `${framework} should return messages`);
  assert.ok(result.messages.join('\n').length > 0, `${framework} should return non-empty output`);
}

export async function runStagingLiveSdkGate(options = {}) {
  const framework = options.framework ?? parseFrameworkArg(process.argv.slice(2));
  const frameworks =
    framework === 'all' ? Object.keys(FRAMEWORKS) : [framework];

  if (!stagingEnabled()) {
    console.log(
      `[skip] ${STAGING_FLAG_ENV} is not enabled; staging live SDK gate skipped intentionally`,
    );
    return { status: 'skipped', reason: 'flag-disabled' };
  }

  process.env.SDKWORK_KERNEL_PROFILE_ID = 'cloud.split-services.production';
  process.env.SDKWORK_KERNEL_ENVIRONMENT = 'production';
  delete process.env.SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS;

  assert.equal(
    mockProviderInvocationAllowed(),
    false,
    'staging live gate must run under production mock fail-closed profile',
  );

  const blocked = [];
  for (const entry of frameworks) {
    const missing = missingCredentialRequirements(entry);
    if (missing.length > 0) {
      blocked.push({ framework: entry, missing });
    }
  }

  if (blocked.length > 0) {
    const message = blocked
      .map(({ framework: entry, missing }) => `${entry}: ${missing.join(', ')}`)
      .join('; ');
    if (credentialsRequired()) {
      throw new Error(`staging credentials missing for ${message}`);
    }
    console.log(`[skip] staging credentials missing (${message})`);
    return { status: 'skipped', reason: 'missing-credentials', blocked };
  }

  for (const entry of frameworks) {
    await runFrameworkLiveInvoke(entry);
    console.log(`[pass] staging live SDK invoke: ${entry}`);
  }

  return { status: 'passed', frameworks };
}

if (process.argv[1]?.replace(/\\/g, '/').endsWith('engine-sdk-live-staging.mjs')) {
  runStagingLiveSdkGate()
    .then((report) => {
      console.log(`engine-sdk-live-staging finished: ${report.status}`);
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
