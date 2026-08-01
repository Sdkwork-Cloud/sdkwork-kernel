import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';
import test from 'node:test';

import { terminateProcessTree } from './codex-cli-live.mjs';

test('routes Codex app-server approvals, resume, and interrupt through canonical Session identity', async (t) => {
  const fixture = createFixture(t);
  const worker = createWorker(t, fixture);
  const sessionId = 'session-canonical-1';

  worker.send(1, 'sdkwork/capability.invoke', {
    operation: {
      operation: 'model_chat_stream',
      model_request_id: 'model-request-1',
      session_id: sessionId,
      turn_id: 'canonical-turn-1',
      messages: ['first Turn'],
      timeout_ms: 5_000,
      execution_options: { require_live_provider: true },
    },
  });
  const optionFrame = await worker.waitFor((frame) =>
    frame.id === 1
      && frame.result?.event === 'stream.event'
      && frame.result.kernel_event?.payload?.providerEventType === 'item/tool/call',
  );
  const optionInteraction = optionFrame.result.kernel_event.payload.interaction;
  assert.equal(optionInteraction.sessionId, sessionId);
  assert.equal(optionInteraction.kind, 'option_picker');
  assert.equal(optionInteraction.correlation.providerToolName, 'request_option_picker');
  worker.send(20, 'sdkwork/serverRequest.respond', {
    model_request_id: 'model-request-1',
    session_id: sessionId,
    provider_session_id: 'provider-session-1',
    provider_turn_id: 'turn-1',
    request_id: 'option-picker-1',
    resolution: {
      action: 'submit',
      selectedOptions: ['Local'],
      freeformAnswer: null,
    },
  });
  const optionResponse = await worker.waitFor((frame) => frame.id === 20);
  assert.equal(optionResponse.result?.interaction_kind, 'option_picker');
  const approvalFrame = await worker.waitFor((frame) =>
    frame.id === 1
      && frame.result?.event === 'stream.event'
      && frame.result.kernel_event?.payload?.providerEventType
        === 'item/commandExecution/requestApproval',
  );
  const approvalEvent = approvalFrame.result.kernel_event;
  assert.equal(approvalEvent.session_id, sessionId);
  assert.equal(approvalEvent.step_id, 'canonical-turn-1');
  assert.equal(approvalEvent.payload.providerSessionId, 'provider-session-1');
  assert.equal(approvalEvent.payload.providerTurnId, 'turn-1');
  assert.equal(approvalEvent.payload.providerRequestId, 'approval-1');
  assert.equal(approvalEvent.payload.interaction.sessionId, sessionId);
  assert.equal(approvalEvent.payload.interaction.category, 'approval');
  assert.equal(approvalEvent.payload.interaction.kind, 'command_execution');
  assert.equal(approvalEvent.payload.interaction.request.command, 'node --test');
  assert.equal(
    approvalEvent.payload.interaction.correlation.providerRequestId,
    'approval-1',
  );
  assert.equal(Object.hasOwn(approvalEvent.payload, 'threadId'), false);
  assert.equal(
    JSON.stringify(approvalEvent.payload.rawProviderPayload).includes('threadId'),
    false,
  );

  worker.send(21, 'sdkwork/serverRequest.respond', {
    model_request_id: 'model-request-1',
    session_id: sessionId,
    turn_id: 'wrong-canonical-turn',
    provider_session_id: 'provider-session-1',
    provider_turn_id: 'turn-1',
    request_id: 'approval-1',
    result: { decision: 'accept' },
  });
  const rejectedCanonicalTurnAffinity = await worker.waitFor((frame) => frame.id === 21);
  assert.match(rejectedCanonicalTurnAffinity.error?.message ?? '', /Turn affinity changed/u);

  worker.send(2, 'sdkwork/serverRequest.respond', {
    model_request_id: 'model-request-1',
    session_id: sessionId,
    turn_id: 'canonical-turn-1',
    provider_session_id: 'wrong-provider-session',
    provider_turn_id: 'turn-1',
    request_id: 'approval-1',
    result: { decision: 'accept' },
  });
  const rejectedAffinity = await worker.waitFor((frame) => frame.id === 2);
  assert.match(rejectedAffinity.error?.message ?? '', /affinity changed/u);

  worker.send(3, 'sdkwork/serverRequest.respond', {
    model_request_id: 'model-request-1',
    session_id: sessionId,
    turn_id: 'canonical-turn-1',
    provider_session_id: 'provider-session-1',
    provider_turn_id: 'turn-1',
    request_id: 'approval-1',
    resolution: { action: 'accept' },
  });
  const approvalResponse = await worker.waitFor((frame) => frame.id === 3);
  assert.equal(approvalResponse.result?.ok, true);
  assert.equal(approvalResponse.result?.provider_session_id, 'provider-session-1');
  assert.equal(approvalResponse.result?.interaction_kind, 'command_execution');
  const firstDone = await worker.waitFor((frame) =>
    frame.id === 1 && frame.result?.event === 'stream.done',
  );
  assert.equal(firstDone.result.provider_session_id, 'provider-session-1');
  assert.equal(firstDone.result.finish_reason, 'stop');

  worker.send(4, 'sdkwork/capability.invoke', {
    operation: {
      operation: 'model_chat_stream',
      model_request_id: 'model-request-2',
      session_id: sessionId,
      turn_id: 'canonical-turn-2',
      provider_session_id: 'provider-session-1',
      messages: ['resumed Turn'],
      timeout_ms: 5_000,
      execution_options: { require_live_provider: true },
    },
  });
  const resumedDone = await worker.waitFor((frame) =>
    frame.id === 4 && frame.result?.event === 'stream.done',
  );
  assert.equal(resumedDone.result.provider_session_id, 'provider-session-1');
  assert.equal(resumedDone.result.finish_reason, 'stop');
  const resumedEvents = worker.frames
    .filter((frame) => frame.id === 4 && frame.result?.event === 'stream.event')
    .map((frame) => frame.result.kernel_event);
  assert.ok(resumedEvents.length > 0);
  assert.ok(resumedEvents.every((event) => event.step_id === 'canonical-turn-2'));
  assert.equal(
    resumedEvents.some((event) => event.payload?.providerTurnId === 'turn-1'),
    false,
  );
  const resumedText = worker.frames
    .filter((frame) => frame.id === 4 && frame.result?.event === 'stream.chunk')
    .map((frame) => frame.result.content)
    .join('');
  assert.equal(resumedText, 'hello ');

  worker.send(5, 'sdkwork/capability.invoke', {
    operation: {
      operation: 'model_chat_stream',
      model_request_id: 'model-request-3',
      session_id: sessionId,
      turn_id: 'canonical-turn-3',
      provider_session_id: 'provider-session-1',
      messages: ['interrupt this Turn'],
      timeout_ms: 5_000,
      execution_options: { require_live_provider: true },
    },
  });
  await worker.waitFor((frame) =>
    frame.id === 5
      && frame.result?.event === 'stream.event'
      && frame.result.kernel_event?.event_type === 'agent.turn.started',
  );
  worker.send(6, 'sdkwork/turn.interrupt', {
    model_request_id: 'model-request-3',
    session_id: sessionId,
    turn_id: 'canonical-turn-3',
    provider_session_id: 'provider-session-1',
    provider_turn_id: 'turn-3',
  });
  const interruptResponse = await worker.waitFor((frame) => frame.id === 6);
  assert.equal(interruptResponse.result?.accepted, true);
  const interruptedDone = await worker.waitFor((frame) =>
    frame.id === 5 && frame.result?.event === 'stream.done',
  );
  assert.equal(interruptedDone.result.finish_reason, 'cancelled');

  const kernelEvents = worker.frames
    .filter((frame) => frame.result?.event === 'stream.event')
    .map((frame) => frame.result.kernel_event);
  assert.ok(kernelEvents.length > 0);
  assert.ok(kernelEvents.every((event) => event.session_id === sessionId));
  assert.ok(kernelEvents.every((event) => event.session_id !== 'provider-session-1'));

  await worker.close();
  const capture = readCapture(fixture.capturePath);
  assert.equal(capture.filter((entry) => entry.message.method === 'initialize').length, 1);
  assert.equal(capture.filter((entry) => entry.message.method === 'thread/start').length, 1);
  assert.equal(capture.filter((entry) => entry.message.method === 'thread/resume').length, 2);
  assert.equal(capture.filter((entry) => entry.message.method === 'turn/start').length, 3);
  assert.equal(capture.filter((entry) => entry.message.method === 'turn/interrupt').length, 1);
  const currentTimeResponse = capture.find(
    (entry) => entry.message.id === 73 && entry.message.method == null,
  )?.message;
  assert.equal(Number.isSafeInteger(currentTimeResponse?.result?.currentTimeAt), true);
  assert.ok(Math.abs(currentTimeResponse.result.currentTimeAt - Math.floor(Date.now() / 1000)) < 30);
  assert.deepEqual(capture.find((entry) => entry.message.id === 72)?.message?.result, {
    contentItems: [{ type: 'inputText', text: '{"completed":true}' }],
    success: true,
  });
  assert.deepEqual(capture.find((entry) => entry.message.id === 74)?.message?.result, {
    contentItems: [{
      type: 'inputText',
      text: 'request_option_picker received invalid arguments.',
    }],
    success: false,
  });
  const dynamicOptionResponse = capture.find(
    (entry) => entry.message.id === 'option-picker-1' && entry.message.method == null,
  )?.message?.result;
  assert.equal(dynamicOptionResponse?.success, true);
  assert.deepEqual(JSON.parse(dynamicOptionResponse.contentItems[0].text), {
    action: 'submit',
    selectedOptions: ['Local'],
    freeformAnswer: null,
  });
  assert.deepEqual(new Set(capture.map((entry) => entry.pid)).size, 1);
});

function createWorker(t, fixture) {
  const child = spawn(
    process.execPath,
    ['scripts/provider-transport-workers/generic-ts-sdk-worker.mjs', '--package', '@openai/codex-sdk'],
    {
      cwd: path.resolve(import.meta.dirname, '../..'),
      env: {
        ...process.env,
        SDKWORK_AGENT_SDK_PACKAGE_PATHS: '{}',
        SDKWORK_CODEX_CLI_BIN: fixture.executablePath,
        SDKWORK_FAKE_APP_SERVER_CAPTURE: fixture.capturePath,
        SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS: '',
        SDKWORK_KERNEL_ENVIRONMENT: 'production',
        SDKWORK_KERNEL_PROFILE_ID: 'cloud.production',
      },
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    },
  );
  t.after(() => terminateProcessTree(child));
  const frames = [];
  const waiters = new Set();
  const stderr = [];
  child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
  const output = readline.createInterface({ input: child.stdout });
  output.on('line', (line) => {
    const frame = JSON.parse(line);
    frames.push(frame);
    for (const waiter of [...waiters]) {
      if (waiter.predicate(frame)) {
        clearTimeout(waiter.timer);
        waiters.delete(waiter);
        waiter.resolve(frame);
      }
    }
  });

  return {
    frames,
    send(id, method, params) {
      child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
    },
    waitFor(predicate, timeoutMs = 5_000) {
      const existing = frames.find(predicate);
      if (existing) return Promise.resolve(existing);
      return new Promise((resolve, reject) => {
        const waiter = { predicate, resolve, timer: null };
        waiter.timer = setTimeout(() => {
          waiters.delete(waiter);
          reject(new Error(`worker frame timeout: ${stderr.join('')}`));
        }, timeoutMs);
        waiters.add(waiter);
      });
    },
    async close() {
      child.stdin.end();
      const exit = await Promise.race([
        new Promise((resolve) => child.once('exit', (code, signal) => resolve({ code, signal }))),
        new Promise((_, reject) => setTimeout(
          () => reject(new Error(`worker close timeout: ${stderr.join('')}`)),
          5_000,
        )),
      ]);
      assert.equal(exit.code, 0, `worker stderr: ${stderr.join('')}`);
      output.close();
    },
  };
}

function createFixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-worker-app-server-'));
  const serverPath = path.join(directory, 'fake-app-server.mjs');
  const capturePath = path.join(directory, 'capture.jsonl');
  fs.writeFileSync(serverPath, fakeAppServerSource(), 'utf8');
  const executablePath = createExecutable(directory, serverPath);
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  return { capturePath, executablePath };
}

function createExecutable(directory, serverPath) {
  if (process.platform === 'win32') {
    const executablePath = path.join(directory, 'codex-fixture.cmd');
    fs.writeFileSync(
      executablePath,
      `@echo off\r\n"${process.execPath}" "${serverPath}"\r\n`,
      'utf8',
    );
    return executablePath;
  }
  const executablePath = path.join(directory, 'codex-fixture');
  fs.writeFileSync(
    executablePath,
    `#!/bin/sh\nexec "${process.execPath}" "${serverPath}"\n`,
    { encoding: 'utf8', mode: 0o755 },
  );
  return executablePath;
}

function fakeAppServerSource() {
  return `import fs from 'node:fs';
import readline from 'node:readline';

const capturePath = process.env.SDKWORK_FAKE_APP_SERVER_CAPTURE;
const providerSessionId = 'provider-session-1';
let turnSequence = 0;
const capture = (message) => fs.appendFileSync(
  capturePath,
  JSON.stringify({ message, pid: process.pid }) + '\\n',
  'utf8',
);
const send = (message) => process.stdout.write(JSON.stringify(message) + '\\n');
const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

input.on('line', (line) => {
  const message = JSON.parse(line);
  capture(message);
  if (message.method === 'initialize') {
    send({ id: message.id, result: { platformFamily: 'fake' } });
    return;
  }
  if (message.method === 'initialized') return;
  if (message.method === 'thread/start') {
    send({ id: message.id, result: { thread: { id: providerSessionId, turns: [] } } });
    return;
  }
  if (message.method === 'thread/resume') {
    send({ id: message.id, result: { thread: { id: message.params.threadId, turns: [] } } });
    return;
  }
  if (message.method === 'turn/start') {
    turnSequence += 1;
    const turnId = 'turn-' + turnSequence;
    if (turnSequence === 2) {
      send({
        method: 'item/agentMessage/delta',
        params: {
          threadId: providerSessionId,
          turnId: 'turn-1',
          itemId: 'stale-message',
          delta: 'stale previous Turn',
        },
      });
    }
    send({ id: message.id, result: { turn: { id: turnId, status: 'inProgress' } } });
    send({
      method: 'turn/started',
      params: { threadId: providerSessionId, turn: { id: turnId, status: 'inProgress' } },
    });
    send({
      method: 'item/agentMessage/delta',
      params: { threadId: providerSessionId, turnId, itemId: 'message-' + turnSequence, delta: 'hello ' },
    });
    if (turnSequence === 1) {
      send({
        id: 72,
        method: 'item/tool/call',
        params: {
          threadId: providerSessionId,
          turnId,
          callId: 'setup-complete-1',
          namespace: null,
          tool: 'setup_codex_step',
          arguments: { step: 'complete' },
        },
      });
    } else if (turnSequence === 2) {
      send({
        method: 'turn/completed',
        params: { threadId: providerSessionId, turn: { id: turnId, status: 'completed' } },
      });
    }
    return;
  }
  if (message.id === 72 && !message.method) {
    send({
      id: 74,
      method: 'item/tool/call',
      params: {
        threadId: providerSessionId,
        turnId: 'turn-1',
        callId: 'invalid-option-1',
        namespace: null,
        tool: 'request_option_picker',
        arguments: { question: 42, options: [] },
      },
    });
    return;
  }
  if (message.id === 74 && !message.method) {
    send({
      id: 73,
      method: 'currentTime/read',
      params: { threadId: providerSessionId },
    });
    return;
  }
  if (message.id === 73 && !message.method) {
    send({
      id: 'option-picker-1',
      method: 'item/tool/call',
      params: {
        threadId: providerSessionId,
        turnId: 'turn-1',
        callId: 'option-picker-call-1',
        namespace: null,
        tool: 'request_option_picker',
        arguments: {
          question: 'Choose a workspace',
          options: [{ label: 'Local', description: null }],
          allowMultiple: false,
          submitLabel: 'Continue',
          skipLabel: null,
        },
      },
    });
    return;
  }
  if (message.id === 'option-picker-1' && !message.method) {
    send({
      method: 'serverRequest/resolved',
      params: { threadId: providerSessionId, requestId: message.id },
    });
    send({
      id: 'approval-1',
      method: 'item/commandExecution/requestApproval',
      params: { threadId: providerSessionId, turnId: 'turn-1', itemId: 'command-1', command: 'node --test' },
    });
    return;
  }
  if (message.id === 'approval-1' && !message.method) {
    send({
      method: 'serverRequest/resolved',
      params: { threadId: providerSessionId, requestId: message.id },
    });
    send({
      method: 'item/agentMessage/delta',
      params: { threadId: providerSessionId, turnId: 'turn-1', itemId: 'message-1', delta: 'world' },
    });
    send({
      method: 'turn/completed',
      params: { threadId: providerSessionId, turn: { id: 'turn-1', status: 'completed' } },
    });
    return;
  }
  if (message.method === 'turn/interrupt') {
    send({ id: message.id, result: {} });
    send({
      method: 'turn/completed',
      params: {
        threadId: providerSessionId,
        turn: { id: message.params.turnId, status: 'interrupted' },
      },
    });
  }
});
`;
}

function readCapture(capturePath) {
  return fs.readFileSync(capturePath, 'utf8')
    .split(/\r?\n/u)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}
