import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';

function invokeWorker(operation, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['scripts/provider-transport-workers/generic-ts-sdk-worker.mjs', '--package', '@sdkwork/missing-sdk'],
      {
        env: {
          ...process.env,
          SDKWORK_KERNEL_PROFILE_ID: 'cloud.production',
          SDKWORK_KERNEL_ENVIRONMENT: 'production',
          SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS: '',
          ...env,
        },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );

    const stderr = [];
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));

    const rl = readline.createInterface({ input: child.stdout });
    rl.once('line', (line) => {
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
        method: 'sdkwork/capability.invoke',
        params: { operation },
      })}\n`,
    );
  });
}

for (const operation of [
  { operation: 'session_create', agent_id: 'agent-1' },
  { operation: 'tool_invoke', tool_id: 'tool-1' },
  { operation: 'skill_invoke', skill_id: 'skill-1' },
  { operation: 'unknown_operation' },
]) {
  const response = await invokeWorker(operation);
  assert.equal(response.result.ok, false, `${operation.operation} must fail closed`);
  if (operation.operation === 'session_create') {
    assert.equal(response.result.mode, 'sdk_live_failed');
    assert.match(response.result.error, /mock fallback is disabled/);
  } else {
    assert.equal(response.result.mode, 'unsupported_operation');
    assert.match(response.result.error, /not implemented by the official provider SDK adapter/);
  }
}

const devResponse = await invokeWorker(
  { operation: 'tool_invoke', tool_id: 'tool-1' },
  {
    SDKWORK_KERNEL_PROFILE_ID: 'standalone.development',
    SDKWORK_KERNEL_ENVIRONMENT: 'development',
  },
);
assert.equal(devResponse.result.ok, false, 'unsupported tool calls never use synthetic fallback');
assert.equal(devResponse.result.mode, 'unsupported_operation');

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-generic-ts-stream-'));
const codexSdkMirror = path.join(tempRoot, 'codex-sdk');
const completionMarkerPath = path.join(tempRoot, 'codex-stream-completed');
const streamReleasePath = path.join(tempRoot, 'codex-stream-release');
fs.mkdirSync(codexSdkMirror, { recursive: true });
fs.writeFileSync(
  path.join(codexSdkMirror, 'package.json'),
  JSON.stringify({
    type: 'module',
    name: '@openai/codex-sdk',
    version: '0.0.0-test',
    exports: { '.': './index.js' },
  }),
  'utf8',
);
fs.writeFileSync(
  path.join(codexSdkMirror, 'index.js'),
  `import fs from 'node:fs';

class FakeThread {
  constructor(id = null) {
    this.id = id;
  }

  async run(prompt) {
    if (!this.id) {
      this.id = 'thread-sdk-live-invoke';
    }
    return {
      finalResponse: 'invoke:' + prompt,
      items: [{ type: 'agent_message', text: 'invoke:' + prompt }],
    };
  }

  async runStreamed(prompt) {
    const thread = this;
    const completionMarkerPath = process.env.SDKWORK_CODEX_STREAM_COMPLETION_MARKER;
    const streamReleasePath = process.env.SDKWORK_CODEX_STREAM_RELEASE_PATH;
    return {
      events: (async function* () {
        thread.id = 'thread-sdk-live-streamed';
        yield { type: 'thread.started', thread_id: thread.id };
        yield {
          type: 'item.updated',
          item: { id: 'message-1', type: 'agent_message', text: 'first' },
        };
        if (prompt === 'provider fatal stream error') {
          yield { type: 'error', message: 'fake Codex stream failed' };
          return;
        }
        while (streamReleasePath && !fs.existsSync(streamReleasePath)) {
          await new Promise((resolve) => setTimeout(resolve, 5));
        }
        if (completionMarkerPath) {
          fs.writeFileSync(completionMarkerPath, 'complete', 'utf8');
        }
        yield {
          type: 'item.completed',
          item: { id: 'message-1', type: 'agent_message', text: 'first second' },
        };
        yield { type: 'turn.completed', usage: {} };
      })(),
    };
  }
}

export class Codex {
  startThread() {
    return new FakeThread();
  }

  resumeThread(id) {
    return new FakeThread(id);
  }
}
`,
  'utf8',
);

try {
  const codexActivityFrames = await invokeWorkerFrames(
    '@openai/codex-sdk',
    {
      operation: 'model_chat',
      model_request_id: 'req-codex-live-activity',
      session_id: 'thread-sdk-existing',
      messages: ['invoke now'],
      timeout_ms: 5_000,
      execution_options: { require_live_provider: true },
    },
    {
      SDKWORK_AGENT_SDK_PACKAGE_PATHS: JSON.stringify({
        '@openai/codex-sdk': codexSdkMirror,
      }),
      SDKWORK_CODEX_CLI_BIN: path.join(tempRoot, 'missing-codex'),
    },
    undefined,
    true,
  );
  const activityPhases = codexActivityFrames
    .filter((frame) => frame.response.result?.event === 'session.activity')
    .map((frame) => frame.response.result.phase);
  assert.deepEqual(activityPhases, ['started', 'working', 'idle', 'terminal']);
  const invokeDone = codexActivityFrames.at(-1)?.response.result;
  assert.equal(invokeDone?.event, 'invoke.done');
  assert.equal(invokeDone?.payload?.provider_session_id, 'thread-sdk-existing');

  let completionMarkerExistedAtFirstChunk = null;
  const codexStreamFrames = await invokeWorkerFrames(
    '@openai/codex-sdk',
    {
      operation: 'model_chat_stream',
      model_request_id: 'req-codex-live-stream',
      messages: ['stream now'],
      timeout_ms: 5_000,
      execution_options: { require_live_provider: true },
    },
    {
      SDKWORK_AGENT_SDK_PACKAGE_PATHS: JSON.stringify({
        '@openai/codex-sdk': codexSdkMirror,
      }),
      SDKWORK_CODEX_STREAM_COMPLETION_MARKER: completionMarkerPath,
      SDKWORK_CODEX_STREAM_RELEASE_PATH: streamReleasePath,
    },
    (frame) => {
      if (
        completionMarkerExistedAtFirstChunk == null &&
        frame.response.result?.event === 'stream.chunk'
      ) {
        completionMarkerExistedAtFirstChunk = fs.existsSync(completionMarkerPath);
        fs.writeFileSync(streamReleasePath, 'release', 'utf8');
      }
    },
  );
  const codexChunks = codexStreamFrames.filter(
    (frame) => frame.response.result?.event === 'stream.chunk',
  );
  const codexEvents = codexStreamFrames.filter(
    (frame) => frame.response.result?.event === 'stream.event',
  );
  const codexDone = codexStreamFrames.find(
    (frame) => frame.response.result?.event === 'stream.done',
  );
  assert.equal(
    completionMarkerExistedAtFirstChunk,
    false,
    'Codex must write the first stream chunk before the provider stream completes',
  );
  assert.deepEqual(
    codexChunks.map((frame) => frame.response.result.content),
    ['first', ' second'],
  );
  assert.deepEqual(
    codexChunks.map((frame) => frame.response.result.sequence),
    [0, 1],
  );
  assert.deepEqual(
    codexEvents.map((frame) => frame.response.result.kernel_event.event_type),
    [
      'agent.session.started',
      'agent.message.updated',
      'agent.message.completed',
      'agent.turn.completed',
    ],
  );
  assert.ok(
    codexEvents.every(
      (frame) => frame.response.result.model_request_id === 'req-codex-live-stream',
    ),
  );
  assert.equal(
    codexEvents[1].response.result.kernel_event.payload.item.text,
    'first',
  );
  assert.ok(codexDone, 'Codex stream must terminate with stream.done');
  assert.equal(codexDone.response.result.model_request_id, 'req-codex-live-stream');
  assert.equal(codexDone.response.result.provider_session_id, 'thread-sdk-live-streamed');

  const failedCodexStreamFrames = await invokeWorkerFrames(
    '@openai/codex-sdk',
    {
      operation: 'model_chat_stream',
      model_request_id: 'req-codex-live-stream-error',
      messages: ['provider fatal stream error'],
      timeout_ms: 5_000,
      execution_options: { require_live_provider: true },
    },
    {
      SDKWORK_AGENT_SDK_PACKAGE_PATHS: JSON.stringify({
        '@openai/codex-sdk': codexSdkMirror,
      }),
    },
  );
  assert.ok(
    failedCodexStreamFrames.some((frame) => frame.response.result?.event === 'stream.chunk'),
    'a provider failure after a delta must preserve the already emitted delta',
  );
  assert.equal(
    failedCodexStreamFrames.some((frame) => frame.response.result?.event === 'stream.done'),
    false,
    'a provider failure must not be represented as a successful stream.done frame',
  );
  const failedCodexResult = failedCodexStreamFrames.at(-1)?.response.result;
  assert.equal(failedCodexResult?.ok, false);
  assert.match(failedCodexResult?.error ?? '', /fake Codex stream failed/);

  const stubStreamFrames = await invokeWorkerFrames(
    '@sdkwork/missing-sdk',
    {
      operation: 'model_chat_stream',
      model_request_id: 'req-stub-stream',
      messages: ['fallback stream'],
    },
    {
      SDKWORK_KERNEL_PROFILE_ID: 'standalone.development',
      SDKWORK_KERNEL_ENVIRONMENT: 'development',
      SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS: '1',
    },
  );
  const stubDone = stubStreamFrames.find(
    (frame) => frame.response.result?.event === 'stream.done',
  );
  assert.ok(stubDone, 'buffered fallback stream must still terminate');
  assert.equal(
    Object.hasOwn(stubDone.response.result, 'provider_session_id'),
    false,
    'providers without a verified provider session id must not receive a fabricated terminal id',
  );
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}

console.log('generic-ts-sdk-worker production fail-closed contract passed.');

function invokeWorkerFrames(packageName, operation, env = {}, onFrame, activityStream = false) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ['scripts/provider-transport-workers/generic-ts-sdk-worker.mjs', '--package', packageName],
      {
        env: {
          ...process.env,
          SDKWORK_KERNEL_PROFILE_ID: 'cloud.production',
          SDKWORK_KERNEL_ENVIRONMENT: 'production',
          SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS: '',
          ...env,
        },
        stdio: ['pipe', 'pipe', 'pipe'],
      },
    );

    const frames = [];
    const stderr = [];
    let settled = false;
    let timeout = null;
    const finish = (callback, value) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timeout);
      rl.close();
      child.kill();
      callback(value);
    };
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
    const rl = readline.createInterface({ input: child.stdout });
    rl.on('line', (line) => {
      let response;
      try {
        response = JSON.parse(line);
      } catch (error) {
        finish(reject, error);
        return;
      }
      const frame = { response, receivedAt: Date.now() };
      try {
        onFrame?.(frame);
      } catch (error) {
        finish(reject, error);
        return;
      }
      frames.push(frame);
      if (
        response.result?.event === 'stream.done' ||
        response.result?.event === 'invoke.done' ||
        response.result?.ok === false ||
        response.error
      ) {
        finish(resolve, frames);
      }
    });
    child.once('error', (error) => finish(reject, error));
    child.once('exit', (code) => {
      if (!settled && code !== 0 && code !== null) {
        finish(reject, new Error(`worker exited with ${code}: ${stderr.join('')}`));
      }
    });
    timeout = setTimeout(() => {
      finish(reject, new Error(`worker did not emit a terminal stream frame: ${stderr.join('')}`));
    }, 5_000);

    child.stdin.write(
      `${JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'sdkwork/capability.invoke',
        params: { operation, activity_stream: activityStream },
      })}\n`,
    );
  });
}
