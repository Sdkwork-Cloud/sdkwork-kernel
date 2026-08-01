import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  buildClaudeCliArgs,
  buildGeminiCliArgs,
  buildOpenCodeCliArgs,
  invokeProviderCliModelChat,
  parseClaudeStreamJson,
  parseGeminiStreamJson,
  parseOpenCodeJson,
  probeProviderCli,
} from './provider-cli-live.mjs';
import { terminateProcessTree } from './codex-cli-live.mjs';

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-provider-cli-live-'));
const workingDirectory = path.join(tempRoot, 'workspace');
const fixture = path.join(tempRoot, 'provider-fixture.mjs');
const LIVE_TEST_TIMEOUT_MS = 30_000;
fs.mkdirSync(workingDirectory, { recursive: true });

try {
  fs.writeFileSync(
    fixture,
    `import fs from 'node:fs';
let input = '';
for await (const chunk of process.stdin) input += chunk;
const kind = process.env.SDKWORK_PROVIDER_CLI_TEST_KIND;
const args = process.argv.slice(2);
const sessionFlag = kind === 'opencode' ? '--session' : kind === 'gemini' ? '--resume' : '--resume';
const sessionIndex = args.indexOf(sessionFlag);
const existingSession = sessionIndex >= 0 ? args[sessionIndex + 1] : null;
const sessionId = process.env.SDKWORK_PROVIDER_CLI_TEST_SESSION_ID || existingSession || kind + '-session-123';
const omitSessionId = process.env.SDKWORK_PROVIDER_CLI_TEST_OMIT_SESSION_ID === '1';
const positional = input;
const delayMs = Number(process.env.SDKWORK_PROVIDER_CLI_TEST_DELAY_MS || 0);
const terminalOnly = process.env.SDKWORK_PROVIDER_CLI_TEST_TERMINAL_ONLY === '1';
if (kind === 'claude') {
  if (terminalOnly) {
    process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', ...(omitSessionId ? {} : { session_id: sessionId }), result: 'claude:' + positional }) + '\\n');
  } else {
    process.stdout.write(JSON.stringify({ type: 'system', subtype: 'init', ...(omitSessionId ? {} : { session_id: sessionId }) }) + '\\n');
    const complete = () => {
      process.stdout.write(JSON.stringify({ type: 'assistant', message: { content: [{ type: 'text', text: 'claude:' + positional }] } }) + '\\n');
      process.stdout.write(JSON.stringify({ type: 'result', subtype: 'success', ...(omitSessionId ? {} : { session_id: sessionId }) }) + '\\n');
    };
    delayMs > 0 ? setTimeout(complete, delayMs) : complete();
  }
} else if (kind === 'gemini') {
  process.stdout.write(JSON.stringify({ type: 'init', ...(omitSessionId ? {} : { session_id: sessionId }) }) + '\\n');
  const complete = () => process.stdout.write(JSON.stringify({ type: 'content', value: 'gemini:' + positional }) + '\\n');
  delayMs > 0 ? setTimeout(complete, delayMs) : complete();
} else {
  process.stdout.write(JSON.stringify({ type: 'step_start', ...(omitSessionId ? {} : { sessionID: sessionId }) }) + '\\n');
  const complete = () => process.stdout.write(JSON.stringify({ type: 'text', ...(omitSessionId ? {} : { sessionID: sessionId }), part: { text: 'opencode:' + positional } }) + '\\n');
  delayMs > 0 ? setTimeout(complete, delayMs) : complete();
}
`,
    'utf8',
  );

  const operation = {
    operation: 'model_chat',
    model_request_id: 'req-provider-cli',
    messages: ['implement the change'],
    model_id: 'provider/model-test',
    session_id: 'session-canonical-cli',
    provider_session_id: 'existing-session-456',
    working_directory: workingDirectory,
    timeout_ms: LIVE_TEST_TIMEOUT_MS,
    execution_options: {
      approval_policy: 'on-failure',
      sandbox_mode: 'workspace-write',
      ephemeral: false,
      require_live_provider: true,
      max_output_bytes: 4_096,
    },
  };

  const claudeArgs = buildClaudeCliArgs(operation);
  assertIncludesPair(claudeArgs, '--model', 'provider/model-test');
  assertIncludesPair(claudeArgs, '--resume', 'existing-session-456');
  assertIncludesPair(claudeArgs, '--permission-mode', 'acceptEdits');
  assertIncludesPair(claudeArgs, '--add-dir', path.resolve(workingDirectory));

  const geminiArgs = buildGeminiCliArgs(operation);
  assertIncludesPair(geminiArgs, '-o', 'stream-json');
  assertIncludesPair(geminiArgs, '--approval-mode', 'auto_edit');
  assertIncludesPair(geminiArgs, '--resume', 'existing-session-456');
  assert.equal(geminiArgs.includes('--sandbox'), true);

  const opencodeArgs = buildOpenCodeCliArgs(operation);
  assert.deepEqual(opencodeArgs.slice(0, 4), ['run', '--format', 'json', '--pure']);
  assertIncludesPair(opencodeArgs, '--dir', path.resolve(workingDirectory));
  assertIncludesPair(opencodeArgs, '--session', 'existing-session-456');
  assert.equal(opencodeArgs.includes('implement the change'), false);

  const canonicalOnlyOperation = { ...operation };
  delete canonicalOnlyOperation.provider_session_id;
  assert.equal(buildClaudeCliArgs(canonicalOnlyOperation).includes('--resume'), false);
  assert.equal(buildGeminiCliArgs(canonicalOnlyOperation).includes('--resume'), false);
  assert.equal(buildOpenCodeCliArgs(canonicalOnlyOperation).includes('--session'), false);

  assert.throws(
    () =>
      buildClaudeCliArgs({
        ...operation,
        execution_options: { approval_policy: 'never' },
      }),
    /dangerous permission bypass is prohibited/,
  );
  assert.throws(
    () =>
      buildGeminiCliArgs({
        ...operation,
        execution_options: { approval_policy: 'yolo' },
      }),
    /dangerous permission bypass is prohibited/,
  );
  assert.throws(
    () =>
      buildOpenCodeCliArgs({
        ...operation,
        execution_options: { sandbox_mode: 'danger-full-access' },
      }),
    /dangerous permission bypass is prohibited/,
  );
  assert.throws(
    () =>
      buildClaudeCliArgs({
        ...operation,
        execution_options: { sandbox_mode: 'bogus', approval_policy: 'on-failure' },
      }),
    /claude_cli_unsupported_sandbox_mode/,
  );
  assert.throws(
    () =>
      buildGeminiCliArgs({
        ...operation,
        execution_options: { sandbox_mode: 'workspace-write', approval_policy: 'bogus' },
      }),
    /gemini_cli_unsupported_approval_policy/,
  );
  assert.throws(
    () =>
      buildGeminiCliArgs({
        ...operation,
        execution_options: {
          sandbox_mode: 'workspace-write',
          approval_policy: 'on-failure',
          ephemeral: true,
        },
      }),
    /gemini_cli_ephemeral_unsupported/,
  );
  assert.throws(
    () =>
      buildOpenCodeCliArgs({
        ...operation,
        execution_options: { sandbox_mode: 'read-only', approval_policy: 'plan' },
      }),
    /opencode_cli_unsupported_policy/,
  );

  assert.equal(
    parseClaudeStreamJson(
      [
        JSON.stringify({ type: 'system', session_id: 'claude-parser' }),
        JSON.stringify({ type: 'assistant', message: { content: [{ text: 'claude text' }] } }),
      ].join('\n'),
    ).provider_session_id,
    'claude-parser',
  );
  assert.deepEqual(
    parseGeminiStreamJson(JSON.stringify({ type: 'content', value: 'gemini text' })).messages,
    ['gemini text'],
  );
  const geminiWarningResult = parseGeminiStreamJson(
    [
      JSON.stringify({ type: 'error', severity: 'warning', message: 'retrying' }),
      JSON.stringify({ type: 'content', value: 'gemini recovered' }),
      JSON.stringify({ type: 'result', status: 'success' }),
    ].join('\n'),
  );
  assert.equal(geminiWarningResult.error, null);
  assert.equal(geminiWarningResult.finish_reason, 'stop');
  assert.deepEqual(geminiWarningResult.messages, ['gemini recovered']);
  const geminiFatalResult = parseGeminiStreamJson(
    JSON.stringify({
      type: 'result',
      status: 'error',
      error: { message: 'credential rejected' },
    }),
  );
  assert.equal(geminiFatalResult.error, 'credential rejected');
  assert.deepEqual(
    parseOpenCodeJson(
      JSON.stringify({ type: 'text', sessionID: 'opencode-parser', part: { text: 'open text' } }),
    ).messages,
    ['open text'],
  );

  for (const provider of [
    {
      packageName: '@anthropic-ai/claude-agent-sdk',
      environmentKey: 'SDKWORK_CLAUDE_CLI_BIN',
      kind: 'claude',
    },
    {
      packageName: '@google/gemini-cli-sdk',
      environmentKey: 'SDKWORK_GEMINI_CLI_BIN',
      kind: 'gemini',
    },
    {
      packageName: '@opencode-ai/sdk',
      environmentKey: 'SDKWORK_OPENCODE_CLI_BIN',
      kind: 'opencode',
    },
  ]) {
    const wrapper = createCliWrapper(tempRoot, fixture, provider.kind);
    const environment = {
      ...process.env,
      [provider.environmentKey]: wrapper,
      SDKWORK_PROVIDER_CLI_TEST_KIND: provider.kind,
    };
    assert.equal(probeProviderCli(provider.packageName, environment).available, true);
    const result = await invokeProviderCliModelChat(provider.packageName, operation, {
      env: environment,
      prompt: 'implement the change',
    });
    assert.equal(result.ok, true);
    assert.equal(result.mode, 'sdk_cli');
    assert.equal(result.provider_session_id, 'existing-session-456');
    assert.equal(result.messages[0], `${provider.kind}:implement the change`);

    const createdResult = await invokeProviderCliModelChat(
      provider.packageName,
      canonicalOnlyOperation,
      {
        env: environment,
        prompt: 'create provider session',
      },
    );
    assert.equal(createdResult.provider_session_id, `${provider.kind}-session-123`);

    await assert.rejects(
      invokeProviderCliModelChat(provider.packageName, operation, {
        env: {
          ...environment,
          SDKWORK_PROVIDER_CLI_TEST_SESSION_ID: `${provider.kind}-session-different`,
        },
        prompt: 'reject provider session mismatch',
      }),
      /resumed a different provider session/,
    );

    const identitylessResult = await invokeProviderCliModelChat(
      provider.packageName,
      canonicalOnlyOperation,
      {
        env: {
          ...environment,
          SDKWORK_PROVIDER_CLI_TEST_OMIT_SESSION_ID: '1',
        },
        prompt: 'do not synthesize provider identity',
      },
    );
    assert.equal(
      identitylessResult.provider_session_id,
      null,
      'a provider Session identity must never be synthesized from canonical session_id.',
    );

    const workerEnvironment = {
      ...environment,
      SDKWORK_KERNEL_PROFILE_ID: 'standalone.production',
      SDKWORK_KERNEL_ENVIRONMENT: 'production',
      SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS: '0',
    };
    const ping = await invokeWorker(provider.packageName, null, workerEnvironment, 'sdkwork/ping');
    assert.equal(ping.result.runtime_available, true);
    assert.equal(ping.result.runtime_mode, 'sdk_cli');
    const workerResult = await invokeWorker(provider.packageName, operation, workerEnvironment);
    assert.equal(workerResult.result.ok, true);
    assert.equal(workerResult.result.mode, 'sdk_cli');
    assert.equal(workerResult.result.provider_session_id, 'existing-session-456');

    const liveActivityFrames = await invokeWorkerFrames(provider.packageName, operation, {
      ...workerEnvironment,
      SDKWORK_PROVIDER_CLI_TEST_DELAY_MS: '250',
    });
    assertLiveActivityTiming(liveActivityFrames, 'existing-session-456', provider.kind);
    if (provider.kind === 'claude') {
      const terminalOnlyFrames = await invokeWorkerFrames(provider.packageName, operation, {
        ...workerEnvironment,
        SDKWORK_PROVIDER_CLI_TEST_TERMINAL_ONLY: '1',
      });
      assert.equal(
        terminalOnlyFrames.some((entry) => entry.frame.result?.event === 'session.activity'),
        false,
        'terminal-only provider identity must remain Unsupported instead of flashing Working',
      );
      assert.equal(terminalOnlyFrames.at(-1).frame.result.payload?.ok, true);
    }
  }

  const multilinePrompt = `line one\n${'x'.repeat(12 * 1024)}`;
  const opencodeWrapper = createCliWrapper(tempRoot, fixture, 'opencode-multiline');
  const multilineResult = await invokeProviderCliModelChat(
    '@opencode-ai/sdk',
    {
      ...operation,
      execution_options: {
        ...operation.execution_options,
        max_output_bytes: 32 * 1024,
      },
    },
    {
      env: {
        ...process.env,
        SDKWORK_OPENCODE_CLI_BIN: opencodeWrapper,
        SDKWORK_PROVIDER_CLI_TEST_KIND: 'opencode',
      },
      prompt: multilinePrompt,
    },
  );
  assert.equal(multilineResult.messages[0], `opencode:${multilinePrompt}`);

  console.log('Provider CLI live contract passed.');
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}

function createCliWrapper(directory, fixturePath, kind) {
  if (process.platform === 'win32') {
    const wrapper = path.join(directory, `${kind}.cmd`);
    fs.writeFileSync(wrapper, `@echo off\r\n"${process.execPath}" "${fixturePath}" %*\r\n`, 'utf8');
    return wrapper;
  }
  const wrapper = path.join(directory, kind);
  const quote = (value) => `'${value.replaceAll("'", "'\\''")}'`;
  fs.writeFileSync(wrapper, `#!/bin/sh\nexec ${quote(process.execPath)} ${quote(fixturePath)} "$@"\n`, 'utf8');
  fs.chmodSync(wrapper, 0o755);
  return wrapper;
}

function assertIncludesPair(values, key, expectedValue) {
  const index = values.indexOf(key);
  assert.notEqual(index, -1, `${key} must be present`);
  assert.equal(values[index + 1], expectedValue);
}

function invokeWorker(packageName, operation, environment, method = 'sdkwork/capability.invoke') {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['scripts/provider-transport-workers/generic-ts-sdk-worker.mjs', '--package', packageName],
      {
        cwd: path.resolve('.'),
        env: environment,
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    let stdout = '';
    const stderr = [];
    child.stdout.on('data', (chunk) => {
      stdout += String(chunk);
      const newline = stdout.indexOf('\n');
      if (newline < 0) return;
      terminateProcessTree(child);
      try {
        resolve(JSON.parse(stdout.slice(0, newline)));
      } catch (error) {
        reject(error);
      }
    });
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
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
        env: environment,
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );
    const frames = [];
    const stderr = [];
    let stdout = '';
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
              terminateProcessTree(child);
              resolve(frames);
              return;
            }
          } catch (error) {
            terminateProcessTree(child);
            reject(error);
            return;
          }
        }
        newline = stdout.indexOf('\n');
      }
    });
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
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
    ['started', 'working', 'working', 'working', 'idle', 'terminal'],
  );
  assert.equal(activity[0].frame.result.provider_session_id, providerSessionId);
  assert.ok(
    done.observedAt - activity[0].observedAt >= 150,
    `${provider} Working must arrive while the CLI process is still running`,
  );
}
