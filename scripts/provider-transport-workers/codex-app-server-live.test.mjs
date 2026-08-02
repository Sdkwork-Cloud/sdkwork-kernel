import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import {
  CodexAppServerLiveTransport,
  CodexAppServerTransportError,
} from './codex-app-server-live.mjs';

const REQUEST_TIMEOUT_MS = 3_000;

test('keeps one app-server process for Session turns and preserves interactive request affinity', async (t) => {
  const fixture = createFakeAppServer('normal');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  const notifications = [];
  const resolved = [];
  transport.onNotification((event) => notifications.push(event));
  transport.on('serverRequestResolved', (event) => resolved.push(event));

  const initializeResult = await transport.connect();
  const processId = transport.pid;
  assert.equal(transport.state, 'ready');
  assert.equal(initializeResult.platformFamily, 'fake');
  assert.ok(Number.isSafeInteger(processId));

  const started = await transport.startSession({
    sessionId: 'session-canonical-1',
    cwd: fixture.directory,
    model: 'codex-test',
    sessionSource: 'startup',
  });
  assert.equal(started.sessionId, 'session-canonical-1');
  assert.equal(started.providerSessionId, 'provider-session-1');
  assert.equal(started.session.sessionId, 'session-canonical-1');
  assert.equal(started.session.providerSessionId, 'provider-session-1');
  assert.equal(started.session.parentProviderSessionId, null);
  assert.equal(Object.hasOwn(started, 'thread'), false);

  const turn = await transport.startTurn({
    sessionId: 'session-canonical-1',
    providerSessionId: started.providerSessionId,
    message: 'implement the change',
  });
  assert.equal(turn.turnId, 'turn-1');
  assert.equal(turn.providerSessionId, 'provider-session-1');

  let completedBeforeResponse = false;
  turn.completion.then(() => {
    completedBeforeResponse = true;
  });
  const serverRequest = await transport.waitForServerRequest({
    method: 'item/commandExecution/requestApproval',
    providerSessionId: started.providerSessionId,
    turnId: turn.turnId,
  });
  await delay(40);
  assert.equal(completedBeforeResponse, false, 'the provider must pause for user approval');
  assert.equal(serverRequest.requestId, 'approval-1');
  assert.equal(serverRequest.params.providerSessionId, 'provider-session-1');
  assert.equal(Object.hasOwn(serverRequest.params, 'threadId'), false);

  await transport.respondToServerRequest(serverRequest, { decision: 'accept' });
  const completed = await turn.completion;
  assert.equal(completed.method, 'turn/completed');
  assert.equal(completed.params.turn.status, 'completed');
  assert.equal(resolved.length, 1);
  assert.equal(resolved[0].known, true);
  await assert.rejects(
    transport.respondToServerRequest(serverRequest, { decision: 'decline' }),
    hasErrorCode('codex_app_server_server_request_already_settled'),
  );
  assert.equal(
    notifications.some((event) => event.method === serverRequest.method),
    false,
    'server requests must not be dispatched as notifications',
  );

  const resumed = await transport.resumeSession({
    sessionId: 'session-canonical-1',
    providerSessionId: started.providerSessionId,
  });
  assert.equal(resumed.resumed, true);
  assert.equal(resumed.providerSessionId, started.providerSessionId);
  assert.equal(transport.pid, processId, 'Session resume must reuse the resident process');

  const read = await transport.readSession({
    sessionId: 'session-canonical-1',
    providerSessionId: started.providerSessionId,
  });
  assert.equal(read.providerSessionId, started.providerSessionId);

  const compacted = await transport.compactSession({
    sessionId: 'session-canonical-1',
    providerSessionId: started.providerSessionId,
  });
  assert.equal(compacted.compacted, true);
  assert.equal(compacted.providerSessionId, started.providerSessionId);

  const forked = await transport.forkSession({
    cwd: fixture.directory,
    sessionId: 'session-canonical-1',
    providerSessionId: started.providerSessionId,
  });
  assert.equal(forked.sourceProviderSessionId, started.providerSessionId);
  assert.equal(forked.providerSessionId, 'provider-session-forked');
  assert.equal(forked.session.parentProviderSessionId, started.providerSessionId);

  const interruptedTurn = await transport.startTurn(
    started.providerSessionId,
    'stop this turn',
  );
  const interrupt = await transport.interruptTurn({
    providerSessionId: started.providerSessionId,
    turnId: interruptedTurn.turnId,
  });
  assert.equal(interrupt.accepted, true);
  const interrupted = await interrupt.completion;
  assert.equal(interrupted.params.turn.status, 'interrupted');
  assert.equal(transport.pid, processId, 'interrupt must not replace the app-server process');

  assert.ok(
    notifications.some((event) => event.method === 'session/started'),
    'provider Session lifecycle notifications must use canonical names',
  );
  assert.ok(
    notifications.some((event) =>
      event.method === 'item/agentMessage/delta' && event.params.delta === 'hello ',
    ),
  );

  const requestLedger = transport.getRequestLedgerSnapshot();
  assert.equal(requestLedger.filter((entry) => entry.method === 'initialize').length, 1);
  assert.equal(requestLedger.filter((entry) => entry.method === 'session/start').length, 1);
  assert.equal(requestLedger.filter((entry) => entry.method === 'session/resume').length, 1);
  const serverLedger = transport.getServerRequestLedgerSnapshot();
  assert.equal(serverLedger.length, 1);
  assert.equal(serverLedger[0].status, 'resolved');

  await transport.close();
  assert.equal(transport.state, 'closed');
  const capture = readCapture(fixture.capturePath);
  assert.equal(capture.filter((entry) => entry.message.method === 'initialize').length, 1);
  assert.equal(capture.filter((entry) => entry.message.method === 'initialized').length, 1);
  assert.ok(capture.findIndex((entry) => entry.message.method === 'initialize')
    < capture.findIndex((entry) => entry.message.method === 'initialized'));
  assert.deepEqual(new Set(capture.map((entry) => entry.pid)), new Set([processId]));
  assert.equal(
    capture.find((entry) => entry.message.method === 'thread/resume')
      .message.params.threadId,
    started.providerSessionId,
  );
  assert.equal(
    capture.find((entry) => entry.message.method === 'turn/interrupt')
      .message.params.turnId,
    interruptedTurn.turnId,
  );
  assert.deepEqual(
    capture.find((entry) => entry.message.method === 'thread/read').message.params,
    { threadId: started.providerSessionId, includeTurns: false },
  );
  assert.deepEqual(
    capture.find((entry) => entry.message.method === 'thread/compact/start').message.params,
    { threadId: started.providerSessionId },
  );
  assert.deepEqual(
    capture.find((entry) => entry.message.method === 'thread/fork').message.params,
    { threadId: started.providerSessionId, cwd: fixture.directory },
  );
});

test('fails closed when the app-server response id does not match the request ledger', async (t) => {
  const fixture = createFakeAppServer('mismatched-response-id');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  const protocolErrors = [];
  transport.on('protocolError', (error) => protocolErrors.push(error));
  await assert.rejects(
    transport.connect(),
    hasErrorCode('codex_app_server_response_id_mismatch'),
  );
  assert.equal(transport.state, 'failed');
  assert.equal(protocolErrors.length, 1);
  assert.equal(protocolErrors[0].code, 'codex_app_server_response_id_mismatch');
});

test('rejects pending work when the resident app-server closes unexpectedly', async (t) => {
  const fixture = createFakeAppServer('unexpected-close');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  await transport.connect();
  await assert.rejects(
    transport.startSession({ sessionId: 'session-close-error' }),
    (error) => {
      assert.ok(error instanceof CodexAppServerTransportError);
      assert.equal(error.code, 'codex_app_server_process_exited');
      assert.match(error.message, /fixture app-server failure/u);
      return true;
    },
  );
  assert.equal(transport.state, 'failed');
  await transport.close();
  await assert.rejects(
    transport.startSession({ sessionId: 'session-after-close' }),
    hasErrorCode('codex_app_server_closed'),
  );
});

test('rejects server-request waiters immediately when the transport closes', async (t) => {
  const fixture = createFakeAppServer('normal');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  await transport.connect();
  const waiting = assert.rejects(
    transport.waitForServerRequest({
      method: 'item/fileChange/requestApproval',
      timeoutMs: 10_000,
    }),
    hasErrorCode('codex_app_server_closed'),
  );
  await transport.close();
  await waiting;
  assert.equal(transport.state, 'closed');
});

test('cancels unanswered server requests when the app-server connection is lost', async (t) => {
  const fixture = createFakeAppServer('pending-server-request-close');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  await transport.connect();
  const turn = await transport.startTurn({
    providerSessionId: 'provider-session-1',
    message: 'wait for approval',
  });
  const completionFailure = assert.rejects(
    turn.completion,
    hasErrorCode('codex_app_server_process_exited'),
  );
  const request = await transport.waitForServerRequest({
    method: 'item/commandExecution/requestApproval',
    providerSessionId: 'provider-session-1',
    turnId: turn.turnId,
  });
  assert.equal(request.requestId, 'approval-1');
  await assert.rejects(
    transport.waitForServerRequest({
      method: 'item/fileChange/requestApproval',
      timeoutMs: 10_000,
    }),
    hasErrorCode('codex_app_server_process_exited'),
  );
  await completionFailure;

  const [record] = transport.getServerRequestLedgerSnapshot();
  assert.equal(record.status, 'cancelled');
  assert.ok(record.cancelledAt);
  assert.equal(record.respondedAt, null);
  assert.equal(record.resolutionUnknownAt, null);
});

test('marks a sent response resolution unknown when provider cleanup is not observed', async (t) => {
  const fixture = createFakeAppServer('response-close-before-resolved');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  await transport.connect();
  const turn = await transport.startTurn({
    providerSessionId: 'provider-session-1',
    message: 'approve before disconnect',
  });
  const completionFailure = assert.rejects(
    turn.completion,
    hasErrorCode('codex_app_server_process_exited'),
  );
  const request = await transport.waitForServerRequest({
    method: 'item/commandExecution/requestApproval',
    providerSessionId: 'provider-session-1',
    turnId: turn.turnId,
  });
  const response = await transport.respondToServerRequest(request, { decision: 'accept' });
  assert.equal(response.status, 'responded');
  await completionFailure;

  const [record] = transport.getServerRequestLedgerSnapshot();
  assert.equal(record.status, 'resolutionUnknown');
  assert.ok(record.respondedAt);
  assert.ok(record.resolutionUnknownAt);
  assert.equal(record.resolvedAt, null);
});

test('keeps provider cleanup distinct when no response was sent on this connection', async (t) => {
  const fixture = createFakeAppServer('provider-clears-without-response');
  const transport = createTransport(fixture);
  registerFixtureCleanup(t, transport, fixture);

  const resolved = new Promise((resolve) => {
    transport.once('serverRequestResolved', resolve);
  });
  await transport.connect();
  const turn = await transport.startTurn({
    providerSessionId: 'provider-session-1',
    message: 'provider clears independently',
  });
  const completionFailure = assert.rejects(
    turn.completion,
    hasErrorCode('codex_app_server_closed'),
  );
  const request = await transport.waitForServerRequest({
    method: 'item/commandExecution/requestApproval',
    providerSessionId: 'provider-session-1',
    turnId: turn.turnId,
  });
  assert.equal(request.requestId, 'approval-1');
  await resolved;

  const [record] = transport.getServerRequestLedgerSnapshot();
  assert.equal(record.status, 'providerCleared');
  assert.ok(record.providerClearedAt);
  assert.equal(record.resolvedAt, null);
  await assert.rejects(
    transport.respondToServerRequest(request, { decision: 'accept' }),
    hasErrorCode('codex_app_server_server_request_already_settled'),
  );
  await transport.close();
  await completionFailure;
});

function createTransport(fixture) {
  return new CodexAppServerLiveTransport({
    args: [fixture.fixturePath],
    closeTimeoutMs: 1_000,
    cwd: fixture.directory,
    env: {
      ...process.env,
      SDKWORK_CODEX_APP_SERVER_TEST_CAPTURE: fixture.capturePath,
      SDKWORK_CODEX_APP_SERVER_TEST_MODE: fixture.mode,
    },
    executable: process.execPath,
    requestTimeoutMs: REQUEST_TIMEOUT_MS,
  });
}

function createFakeAppServer(mode) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-codex-app-server-'));
  const fixturePath = path.join(directory, 'fake-app-server.mjs');
  const capturePath = path.join(directory, 'capture.jsonl');
  fs.writeFileSync(fixturePath, fakeAppServerSource(), 'utf8');
  return { capturePath, directory, fixturePath, mode };
}

function registerFixtureCleanup(t, transport, fixture) {
  t.after(async () => {
    try {
      await transport.close();
    } finally {
      fs.rmSync(fixture.directory, {
        recursive: true,
        force: true,
        maxRetries: 10,
        retryDelay: 50,
      });
    }
  });
}

function fakeAppServerSource() {
  return `import fs from 'node:fs';
import readline from 'node:readline';

const capturePath = process.env.SDKWORK_CODEX_APP_SERVER_TEST_CAPTURE;
const mode = process.env.SDKWORK_CODEX_APP_SERVER_TEST_MODE;
const providerSessionId = 'provider-session-1';
let turnSequence = 0;

const capture = (message) => {
  fs.appendFileSync(capturePath, JSON.stringify({ message, pid: process.pid }) + '\\n', 'utf8');
};
const send = (message) => process.stdout.write(JSON.stringify(message) + '\\n');
const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

input.on('line', (line) => {
  const message = JSON.parse(line);
  capture(message);

  if (message.method === 'initialize') {
    const id = mode === 'mismatched-response-id' ? message.id + 100 : message.id;
    send({ id, result: { codexHome: '/fake/codex', platformFamily: 'fake', platformOs: 'test' } });
    return;
  }
  if (message.method === 'initialized') {
    return;
  }
  if (message.method === 'thread/start') {
    if (mode === 'unexpected-close') {
      process.stderr.write('fixture app-server failure');
      setTimeout(() => process.exit(17), 10);
      return;
    }
    const thread = {
      id: providerSessionId,
      name: 'Fixture session',
      parentThreadId: null,
      turns: [],
    };
    send({ id: message.id, result: { thread } });
    send({ method: 'thread/started', params: { thread } });
    return;
  }
  if (message.method === 'thread/resume') {
    send({
      id: message.id,
      result: {
        thread: {
          id: message.params.threadId,
          name: 'Fixture session',
          parentThreadId: null,
          turns: [],
        },
      },
    });
    return;
  }
  if (message.method === 'thread/read') {
    send({
      id: message.id,
      result: {
        thread: {
          id: message.params.threadId,
          name: 'Fixture session',
          parentThreadId: null,
          turns: [],
        },
      },
    });
    return;
  }
  if (message.method === 'thread/compact/start') {
    send({ id: message.id, result: {} });
    return;
  }
  if (message.method === 'thread/fork') {
    send({
      id: message.id,
      result: {
        thread: {
          id: 'provider-session-forked',
          name: 'Forked fixture session',
          parentThreadId: message.params.threadId,
          turns: [],
        },
        cwd: message.params.cwd,
        model: 'codex-test',
        modelProvider: 'openai',
      },
    });
    return;
  }
  if (message.method === 'turn/start') {
    turnSequence += 1;
    const turnId = 'turn-' + turnSequence;
    const turn = { id: turnId, status: 'inProgress', items: [], error: null };
    send({ id: message.id, result: { turn } });
    send({
      method: 'turn/started',
      params: { threadId: message.params.threadId, turn },
    });
    if (turnSequence === 1) {
      send({
        method: 'item/agentMessage/delta',
        params: {
          delta: 'hello ',
          itemId: 'agent-message-1',
          threadId: message.params.threadId,
          turnId,
        },
      });
      send({
        id: 'approval-1',
        method: 'item/commandExecution/requestApproval',
        params: {
          command: 'node --test',
          itemId: 'command-1',
          threadId: message.params.threadId,
          turnId,
        },
      });
      if (mode === 'pending-server-request-close') {
        process.stderr.write('fixture closed with a pending server request');
        setTimeout(() => process.exit(18), 100);
      }
      if (mode === 'provider-clears-without-response') {
        setTimeout(() => {
          send({
            method: 'serverRequest/resolved',
            params: { requestId: 'approval-1', threadId: providerSessionId },
          });
        }, 20);
      }
    }
    return;
  }
  if (message.id === 'approval-1' && !message.method) {
    if (mode === 'response-close-before-resolved') {
      process.stderr.write('fixture closed before provider resolution');
      setTimeout(() => process.exit(19), 20);
      return;
    }
    send({
      method: 'serverRequest/resolved',
      params: { requestId: message.id, threadId: providerSessionId },
    });
    send({
      method: 'item/agentMessage/delta',
      params: {
        delta: 'world',
        itemId: 'agent-message-1',
        threadId: providerSessionId,
        turnId: 'turn-1',
      },
    });
    send({
      method: 'turn/completed',
      params: {
        threadId: providerSessionId,
        turn: { id: 'turn-1', status: 'completed', items: [], error: null },
      },
    });
    return;
  }
  if (message.method === 'turn/interrupt') {
    send({ id: message.id, result: {} });
    send({
      method: 'turn/completed',
      params: {
        threadId: message.params.threadId,
        turn: {
          id: message.params.turnId,
          status: 'interrupted',
          items: [],
          error: null,
        },
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

function hasErrorCode(code) {
  return (error) => {
    assert.ok(error instanceof CodexAppServerTransportError);
    assert.equal(error.code, code);
    return true;
  };
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
