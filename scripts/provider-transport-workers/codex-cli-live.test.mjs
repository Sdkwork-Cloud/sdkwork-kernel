import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  buildCodexCliArgs,
  invokeCodexCliModelChat,
  parseCodexCliJsonl,
  probeCodexCli,
} from './codex-cli-live.mjs';

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-codex-cli-live-'));
const workingDirectory = path.join(tempRoot, 'workspace');
const fixturePath = path.join(tempRoot, 'fake-codex-fixture.mjs');
const capturePath = path.join(tempRoot, 'capture.json');
fs.mkdirSync(workingDirectory, { recursive: true });
fs.writeFileSync(
  fixturePath,
  `import fs from 'node:fs';

let prompt = '';
for await (const chunk of process.stdin) {
  prompt += chunk;
}
const capturePath = process.env.SDKWORK_CODEX_TEST_CAPTURE;
if (capturePath) {
  fs.writeFileSync(capturePath, JSON.stringify({
    args: process.argv.slice(2),
    cwd: process.cwd(),
    prompt,
  }), 'utf8');
}

const emit = () => {
  const args = process.argv.slice(2);
  const resumeIndex = args.indexOf('resume');
  const resumedSessionId = resumeIndex >= 0 ? args[resumeIndex + 1] : null;
  const providerSessionId = prompt === 'mismatch-session'
    ? 'thread-provider-mismatch'
    : resumedSessionId || 'thread-fixture-123';
  if (prompt !== 'without-provider-session') {
    process.stdout.write(JSON.stringify({ type: 'thread.started', thread_id: providerSessionId }) + '\\n');
  }
  const complete = () => {
    process.stdout.write(JSON.stringify({
      type: 'item.completed',
      item: { type: 'agent_message', text: 'answer:' + prompt },
    }) + '\\n');
    process.stdout.write(JSON.stringify({ type: 'turn.completed' }) + '\\n');
  };
  if (prompt === 'live activity') {
    setTimeout(complete, 250);
  } else {
    complete();
  }
};

if (prompt === 'timeout') {
  setTimeout(emit, 2_000);
} else {
  emit();
}
`,
  'utf8',
);

const wrapperPath = createCodexWrapper(tempRoot, fixturePath);
const cliEnvironment = {
  ...process.env,
  SDKWORK_CODEX_CLI_BIN: wrapperPath,
  SDKWORK_CODEX_TEST_CAPTURE: capturePath,
};

const probe = probeCodexCli(cliEnvironment);
assert.equal(probe.available, true, 'configured fake Codex CLI should be available');
assert.equal(path.resolve(probe.executable), path.resolve(wrapperPath));

const operation = {
  operation: 'model_chat',
  model_request_id: 'req-cli-1',
  messages: ['legacy prompt'],
  wire_messages: [{ role: 'user', content: [{ type: 'text', text: 'structured prompt' }] }],
  model_id: 'codex-test-model',
  session_id: 'thread-existing-456',
  working_directory: workingDirectory,
  timeout_ms: 5_000,
  execution_options: {
    approval_policy: 'onrequest',
    sandbox_mode: 'workspace_write',
    full_auto: false,
    skip_git_repo_check: true,
    ephemeral: true,
    max_output_bytes: 4_096,
    temperature: 0.2,
    top_p: 0.8,
    max_tokens: 1_024,
  },
};

const args = buildCodexCliArgs(operation);
assert.deepEqual(args.slice(0, 2), ['exec', '--json']);
assertIncludesPair(args, '--model', 'codex-test-model');
assertIncludesPair(args, '--sandbox', 'workspace-write');
assertIncludesPair(args, '--config', 'approval_policy="on-request"');
assertIncludesPair(args, '--cd', path.resolve(workingDirectory));
assertIncludesPair(args, 'resume', 'thread-existing-456');
assert.ok(args.includes('--skip-git-repo-check'));
assert.ok(args.includes('--ephemeral'));
assert.equal(args.at(-1), '-');
assert.equal(operation.execution_options.temperature, 0.2, 'unsupported CLI options stay in protocol');
assert.equal(operation.execution_options.top_p, 0.8, 'unsupported CLI options stay in protocol');
assert.equal(operation.execution_options.max_tokens, 1_024, 'unsupported CLI options stay in protocol');

const result = await invokeCodexCliModelChat(operation, {
  env: cliEnvironment,
  packageName: '@openai/codex-sdk',
  prompt: 'structured prompt',
});
assert.equal(result.ok, true);
assert.equal(result.mode, 'sdk_cli');
assert.equal(result.provider_session_id, 'thread-existing-456');
assert.deepEqual(result.messages, ['answer:structured prompt']);

const capture = JSON.parse(fs.readFileSync(capturePath, 'utf8'));
assert.equal(path.resolve(capture.cwd), path.resolve(workingDirectory));
assert.equal(capture.prompt, 'structured prompt');
assertIncludesPair(capture.args, '--model', 'codex-test-model');
assertIncludesPair(capture.args, 'resume', 'thread-existing-456');
assertIncludesPair(capture.args, '--sandbox', 'workspace-write');
assertIncludesPair(capture.args, '--config', 'approval_policy="on-request"');

const resultWithoutProviderSession = await invokeCodexCliModelChat(operation, {
  env: cliEnvironment,
  packageName: '@openai/codex-sdk',
  prompt: 'without-provider-session',
});
assert.equal(
  resultWithoutProviderSession.provider_session_id,
  null,
  'the CLI adapter must not treat a requested resume id as provider-verified terminal metadata',
);

assertIncludesPair(
  buildCodexCliArgs({
    ...operation,
    execution_options: { sandbox_mode: 'danger-full-access' },
  }),
  '--sandbox',
  'danger-full-access',
);
assert.throws(
  () => buildCodexCliArgs({ ...operation, working_directory: path.join(tempRoot, 'missing') }),
  /codex_cli_invalid_working_directory/,
);

const parsed = parseCodexCliJsonl(
  [
    JSON.stringify({ type: 'thread.started', thread_id: 'thread-parser' }),
    JSON.stringify({
      type: 'item.completed',
      item: { type: 'agent_message', text: 'parsed answer' },
    }),
    JSON.stringify({ type: 'turn.completed' }),
  ].join('\n'),
);
assert.deepEqual(parsed.messages, ['parsed answer']);
assert.equal(parsed.provider_session_id, 'thread-parser');

await assert.rejects(
  invokeCodexCliModelChat(
    {
      operation: 'model_chat',
      model_request_id: 'req-timeout',
      messages: ['timeout'],
      working_directory: workingDirectory,
      timeout_ms: 50,
    },
    { env: cliEnvironment, prompt: 'timeout' },
  ),
  /codex_cli_timeout/,
);

await assert.rejects(
  invokeCodexCliModelChat(
    {
      operation: 'model_chat',
      model_request_id: 'req-limit',
      messages: ['large'],
      working_directory: workingDirectory,
      execution_options: { max_output_bytes: 4 },
    },
    { env: cliEnvironment, prompt: 'large' },
  ),
  /codex_cli_output_limit_exceeded/,
);

const productionEnvironment = {
  SDKWORK_KERNEL_PROFILE_ID: 'cloud.production',
  SDKWORK_KERNEL_ENVIRONMENT: 'production',
  SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS: '',
  SDKWORK_AGENT_SDK_WORKSPACE_ROOT: path.resolve('.'),
  SDKWORK_AGENT_SDK_PACKAGE_PATHS: '',
  SDKWORK_CODEX_CLI_BIN: wrapperPath,
  SDKWORK_CODEX_TEST_CAPTURE: capturePath,
};
const pingResponse = await invokeWorker('@openai/codex-sdk', null, productionEnvironment, 'sdkwork/ping');
assert.equal(pingResponse.result.package_resolved, false);
assert.equal(pingResponse.result.cli_available, true);
assert.equal(pingResponse.result.runtime_available, true);
assert.equal(pingResponse.result.runtime_mode, 'sdk_cli');

const workerResponse = await invokeWorker('@openai/codex-sdk', operation, productionEnvironment);
assert.equal(workerResponse.result.ok, true, 'production should use a real Codex CLI transport');
assert.equal(workerResponse.result.mode, 'sdk_cli');
assert.equal(workerResponse.result.provider_session_id, 'thread-existing-456');

const liveActivityFrames = await invokeWorkerFrames(
  '@openai/codex-sdk',
  {
    ...operation,
    model_request_id: 'req-cli-live-activity',
    session_id: undefined,
    messages: ['live activity'],
    wire_messages: undefined,
  },
  productionEnvironment,
);
assertLiveActivityTiming(liveActivityFrames, 'thread-fixture-123', 'Codex');

const mismatchedActivityFrames = await invokeWorkerFrames(
  '@openai/codex-sdk',
  {
    ...operation,
    model_request_id: 'req-cli-mismatch',
    messages: ['mismatch-session'],
    wire_messages: undefined,
  },
  productionEnvironment,
);
assert.equal(mismatchedActivityFrames.at(-1).frame.result.payload?.ok, false);
assert.equal(
  mismatchedActivityFrames.some((entry) => entry.frame.result?.event === 'session.activity'),
  false,
  'a mismatched requested/native Codex identity must not emit activity',
);

const missingCliEnvironment = {
  ...productionEnvironment,
  SDKWORK_CODEX_CLI_BIN: path.join(tempRoot, 'missing-codex'),
};
const unavailableResponse = await invokeWorker(
  '@openai/codex-sdk',
  { operation: 'model_chat', model_request_id: 'req-unavailable', messages: ['hello'] },
  missingCliEnvironment,
);
assert.equal(unavailableResponse.result.ok, false);
assert.equal(unavailableResponse.result.mode, 'sdk_live_failed');

const developmentResponse = await invokeWorker(
  '@openai/codex-sdk',
  { operation: 'model_chat', model_request_id: 'req-development', messages: ['hello'] },
  {
    ...missingCliEnvironment,
    SDKWORK_KERNEL_PROFILE_ID: 'standalone.development',
    SDKWORK_KERNEL_ENVIRONMENT: 'development',
  },
);
assert.equal(developmentResponse.result.ok, true);
assert.equal(developmentResponse.result.mode, 'stub');

const requiredLiveResponse = await invokeWorker(
  '@openai/codex-sdk',
  {
    operation: 'model_chat',
    model_request_id: 'req-required-live',
    messages: ['hello'],
    execution_options: { require_live_provider: true },
  },
  {
    ...missingCliEnvironment,
    SDKWORK_KERNEL_PROFILE_ID: 'standalone.development',
    SDKWORK_KERNEL_ENVIRONMENT: 'development',
  },
);
assert.equal(requiredLiveResponse.result.ok, false);
assert.equal(requiredLiveResponse.result.mode, 'sdk_live_failed');

console.log('codex-cli-live contract passed.');

function assertIncludesPair(values, key, expectedValue) {
  const index = values.indexOf(key);
  assert.notEqual(index, -1, `${key} should be present`);
  assert.equal(values[index + 1], expectedValue);
}

function createCodexWrapper(root, fixture) {
  if (process.platform === 'win32') {
    const wrapper = path.join(root, 'codex.cmd');
    fs.writeFileSync(wrapper, `@echo off\r\n"${process.execPath}" "${fixture}" %*\r\n`, 'utf8');
    return wrapper;
  }

  const wrapper = path.join(root, 'codex');
  const quote = (value) => `'${value.replaceAll("'", "'\\''")}'`;
  fs.writeFileSync(
    wrapper,
    `#!/bin/sh\nexec ${quote(process.execPath)} ${quote(fixture)} "$@"\n`,
    'utf8',
  );
  fs.chmodSync(wrapper, 0o755);
  return wrapper;
}

function invokeWorker(packageName, operation, environment, method = 'sdkwork/capability.invoke') {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['scripts/provider-transport-workers/generic-ts-sdk-worker.mjs', '--package', packageName],
      {
        cwd: path.resolve('.'),
        env: { ...process.env, ...environment },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    const stderr = [];
    let stdout = '';
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
    child.stdout.on('data', (chunk) => {
      stdout += String(chunk);
      const newline = stdout.indexOf('\n');
      if (newline === -1) {
        return;
      }
      const line = stdout.slice(0, newline);
      child.kill();
      try {
        resolve(JSON.parse(line));
      } catch (error) {
        reject(error);
      }
    });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code !== 0 && code !== null) {
        reject(new Error(`worker exited with ${code}: ${stderr.join('')}`));
      }
    });

    child.stdin.write(
      `${JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method,
        params: operation ? { operation } : undefined,
      })}\n`,
    );
  });
}

function invokeWorkerFrames(packageName, operation, environment) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['scripts/provider-transport-workers/generic-ts-sdk-worker.mjs', '--package', packageName],
      {
        cwd: path.resolve('.'),
        env: { ...process.env, ...environment },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    const frames = [];
    const stderr = [];
    let stdout = '';
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
    child.stdout.on('data', (chunk) => {
      stdout += String(chunk);
      let newline = stdout.indexOf('\n');
      while (newline >= 0) {
        const line = stdout.slice(0, newline).trim();
        stdout = stdout.slice(newline + 1);
        if (line) {
          try {
            const frame = JSON.parse(line);
            frames.push({ frame, observedAt: Date.now() });
            if (frame.result?.event === 'invoke.done' || frame.result?.ok === false) {
              child.kill();
              resolve(frames);
              return;
            }
          } catch (error) {
            child.kill();
            reject(error);
            return;
          }
        }
        newline = stdout.indexOf('\n');
      }
    });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code !== 0 && code !== null) {
        reject(new Error(`worker exited with ${code}: ${stderr.join('')}`));
      }
    });
    child.stdin.write(
      `${JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'sdkwork/capability.invoke',
        params: { operation, activity_stream: true },
      })}\n`,
    );
  });
}

function assertLiveActivityTiming(frames, providerSessionId, provider) {
  const activity = frames.filter((entry) => entry.frame.result?.event === 'session.activity');
  const done = frames.find((entry) => entry.frame.result?.event === 'invoke.done');
  assert.ok(done, `${provider} invocation must emit invoke.done`);
  assert.deepEqual(
    activity.map((entry) => entry.frame.result.phase),
    ['started', 'working', 'working', 'idle', 'terminal'],
  );
  assert.equal(activity[0].frame.result.provider_session_id, providerSessionId);
  assert.ok(
    done.observedAt - activity[0].observedAt >= 150,
    `${provider} Working must arrive while the CLI process is still running`,
  );
}
