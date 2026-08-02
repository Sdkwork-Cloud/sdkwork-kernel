import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';

function spawnWorker(env = {}, packageName = 'sdkwork_missing_python_sdk') {
  const child = spawn(
    'python',
    [
      'scripts/provider-transport-workers/generic_python_sdk_worker.py',
      '--package',
      packageName,
    ],
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
  return { child, stderr };
}

function invokeWorker(operation, env = {}, packageName = 'sdkwork_missing_python_sdk') {
  return new Promise((resolve, reject) => {
    const { child, stderr } = spawnWorker(env, packageName);
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

/** Collects all JSON-RPC responses for a streaming capability invoke. */
function invokeWorkerStreaming(operation, env = {}, packageName = 'tui_gateway') {
  return new Promise((resolve, reject) => {
    const { child, stderr } = spawnWorker(env, packageName);
    const frames = [];
    const rl = readline.createInterface({ input: child.stdout });
    rl.on('line', (line) => {
      try {
        frames.push(JSON.parse(line));
        const result = frames.at(-1).result;
        if (result?.event === 'stream.done') {
          child.kill();
          resolve(frames);
        }
      } catch (error) {
        child.kill();
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
        id: 7,
        method: 'sdkwork/capability.invoke',
        params: { operation },
      })}\n`,
    );
  });
}

/** Sends one control request (e.g. sdkwork/serverRequest.respond). */
function invokeWorkerControl(control, env = {}, packageName = 'tui_gateway') {
  return new Promise((resolve, reject) => {
    const { child, stderr } = spawnWorker(env, packageName);
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
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id: 9, ...control })}\n`);
  });
}

// ---------------------------------------------------------------------------
// Fake Hermes TUI gateway implementing the desktop app wire protocol:
// session.create/resume -> prompt.submit -> event stream with blocking
// approval.request interactions.
// ---------------------------------------------------------------------------

const FAKE_GATEWAY = `import json
import sys
import threading
import time

SESSIONS = {}
STDOUT_LOCK = threading.Lock()

def emit(session_id, event_type, payload=None):
    frame = json.dumps({"jsonrpc": "2.0", "method": "event",
                        "params": {"type": event_type, "session_id": session_id,
                                   "payload": payload or {}}})
    with STDOUT_LOCK:
        print(frame, flush=True)

def reply(rid, result):
    with STDOUT_LOCK:
        print(json.dumps({"jsonrpc": "2.0", "id": rid, "result": result}), flush=True)

for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    params = request.get("params") or {}
    rid = request.get("id")
    if method == "session.create":
        sid = "live0001"
        stored = "20260802_120000_abc123"
        SESSIONS[sid] = stored
        reply(rid, {"session_id": sid, "stored_session_id": stored,
                    "message_count": 0, "messages": [], "info": {}})
    elif method == "session.resume":
        stored = params.get("session_id")
        sid = "live0002"
        reply(rid, {"session_id": sid, "stored_session_id": stored,
                    "resumed": True, "messages": [
                        {"id": "m1", "role": "user", "content": "earlier question",
                         "timestamp": "2026-08-02T10:00:00Z"},
                        {"id": "m2", "role": "assistant", "content": "earlier answer",
                         "timestamp": "2026-08-02T10:00:10Z"},
                    ], "info": {}})
    elif method == "prompt.submit":
        sid = params.get("session_id")
        reply(rid, {"status": "streaming"})
        def stream_turn():
            emit(sid, "message.start")
            emit(sid, "message.delta", {"text": "Hello "})
            if params.get("text") == "trigger approval":
                emit(sid, "approval.request", {"request_id": 42, "command": "rm -rf /"})
                deadline = time.time() + 5
                while time.time() < deadline:
                    if "approval_resolved" in SESSIONS:
                        break
                    time.sleep(0.05)
            emit(sid, "message.delta", {"text": "world"})
            emit(sid, "message.complete", {"text": "Hello world", "status": "completed",
                                           "usage": {"input_tokens": 7, "output_tokens": 2}})
        threading.Thread(target=stream_turn, daemon=True).start()
    elif method == "approval.respond":
        SESSIONS["approval_resolved"] = params.get("choice")
        reply(rid, {"resolved": True})
    elif method == "session.list":
        reply(rid, {"sessions": [
            {"id": "20260801_090000_def456", "title": "Review task",
             "preview": "Review the provider", "started_at": "2026-08-01T09:00:00Z",
             "message_count": 12, "source": "cli"},
        ]})
    elif method == "session.interrupt":
        reply(rid, {"status": "ok"})
    else:
        reply(rid, {"status": "ok"})
`;

// ---------------------------------------------------------------------------

const gatewayRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-hermes-gateway-worker-'));
const gatewayPackage = path.join(gatewayRoot, 'tui_gateway');
fs.mkdirSync(gatewayPackage, { recursive: true });
fs.writeFileSync(path.join(gatewayPackage, '__init__.py'), '', 'utf8');
fs.writeFileSync(path.join(gatewayPackage, 'entry.py'), FAKE_GATEWAY, 'utf8');
const gatewayEnv = {
  PYTHONPATH: [gatewayRoot, process.env.PYTHONPATH].filter(Boolean).join(path.delimiter),
};

for (const operation of [
  { operation: 'session_create', agent_id: 'agent-1' },
  { operation: 'model_chat', model_request_id: 'model-1', messages: ['hello'] },
  { operation: 'tool_invoke', tool_id: 'tool-1' },
  { operation: 'skill_invoke', skill_id: 'skill-1' },
  { operation: 'unknown_operation' },
]) {
  const response = await invokeWorker(operation);
  assert.equal(response.result.ok, false, `${operation.operation} must fail closed`);
  if (
    operation.operation === 'tool_invoke'
    || operation.operation === 'skill_invoke'
    || operation.operation === 'unknown_operation'
  ) {
    assert.equal(response.result.mode, 'unsupported_operation');
    assert.match(response.result.error, /not implemented by the official provider SDK adapter/);
  } else {
    assert.equal(response.result.mode, 'sdk_live_failed');
    assert.match(response.result.error, /mock fallback is disabled|unsupported operation/);
  }
}

const devResponse = await invokeWorker(
  { operation: 'model_chat', model_request_id: 'model-1', messages: ['hello'] },
  {
    SDKWORK_KERNEL_PROFILE_ID: 'standalone.development',
    SDKWORK_KERNEL_ENVIRONMENT: 'development',
  },
);
assert.equal(devResponse.result.ok, true, 'development profile can still use SDK probe fallback');

// -- model_chat through the real gateway protocol (session.create + prompt.submit)
const chatResponse = await invokeWorker(
  {
    operation: 'model_chat',
    model_request_id: 'hermes-chat-1',
    messages: ['hello hermes'],
    wire_messages: [{ role: 'user', content: [{ type: 'text', text: 'hello hermes' }] }],
    timeout_ms: 10_000,
  },
  gatewayEnv,
  'tui_gateway',
);
assert.equal(chatResponse.result.ok, true, 'Hermes model_chat should succeed');
assert.equal(chatResponse.result.mode, 'sdk_live');
assert.equal(chatResponse.result.gateway_method, 'prompt.submit');
assert.deepEqual(chatResponse.result.messages, ['Hello world']);
assert.equal(chatResponse.result.provider_session_id, '20260802_120000_abc123');
assert.ok(
  chatResponse.result.diagnostics.some((line) => line === 'sdk_runtime_session_id=20260802_120000_abc123'),
  'diagnostics must carry the persistent provider session id',
);

// -- model_chat resume: provider_session_id routes to session.resume
const resumeResponse = await invokeWorker(
  {
    operation: 'model_chat',
    model_request_id: 'hermes-resume-1',
    messages: ['continue'],
    provider_session_id: '20260701_080000_xyz789',
    timeout_ms: 10_000,
  },
  gatewayEnv,
  'tui_gateway',
);
assert.equal(resumeResponse.result.ok, true, 'Hermes resume should succeed');
assert.deepEqual(resumeResponse.result.messages, ['Hello world']);

// -- model_chat_stream: multi-frame streaming protocol
const streamFrames = await invokeWorkerStreaming(
  {
    operation: 'model_chat_stream',
    model_request_id: 'hermes-stream-1',
    messages: ['stream this'],
    timeout_ms: 10_000,
  },
  gatewayEnv,
);
const chunkFrames = streamFrames.filter((frame) => frame.result?.event === 'stream.chunk');
assert.ok(chunkFrames.length >= 2, `expected stream chunks, got ${streamFrames.length} frames`);
assert.equal(chunkFrames.map((frame) => frame.result.content).join(''), 'Hello world');
const doneFrame = streamFrames.at(-1).result;
assert.equal(doneFrame.event, 'stream.done');
assert.equal(doneFrame.finish_reason, 'stop');
assert.equal(doneFrame.provider_session_id, '20260802_120000_abc123');
const kernelEventFrames = streamFrames.filter(
  (frame) => frame.result?.event === 'stream.event' && frame.result.kernel_event,
);
assert.ok(
  kernelEventFrames.some((frame) => frame.result.kernel_event.event_type === 'agent.turn.completed'),
  'streaming must emit a turn.completed kernel event',
);

// -- session_list through the gateway
const listResponse = await invokeWorker(
  { operation: 'session_list', limit: 50 },
  gatewayEnv,
  'tui_gateway',
);
assert.equal(listResponse.result.ok, true);
assert.equal(listResponse.result.items.length, 1);
assert.equal(listResponse.result.items[0].provider_session_id, '20260801_090000_def456');
assert.equal(listResponse.result.items[0].title, 'Review task');
assert.equal(listResponse.result.items[0].message_count, 12);

// -- session_history through the gateway (session.resume transcript)
const historyResponse = await invokeWorker(
  {
    operation: 'session_history',
    provider_session_id: '20260701_080000_xyz789',
    limit: 50,
  },
  gatewayEnv,
  'tui_gateway',
);
assert.equal(historyResponse.result.ok, true);
assert.equal(historyResponse.result.items.length, 2);
assert.equal(historyResponse.result.items[0].role, 'user');
assert.equal(historyResponse.result.items[0].parts[0].text, 'earlier question');
assert.equal(historyResponse.result.items[1].role, 'assistant');

// -- session_interrupt through the gateway
const interruptResponse = await invokeWorker(
  {
    operation: 'session_interrupt',
    provider_session_id: '20260701_080000_xyz789',
  },
  gatewayEnv,
  'tui_gateway',
);
assert.equal(interruptResponse.result.ok, true);
assert.equal(interruptResponse.result.status, 'applied');

// -- blocking approval interaction resolved through sdkwork/serverRequest.respond
const approvalWorker = spawnWorker(gatewayEnv, 'tui_gateway');
const approvalFrames = [];
const approvalRl = readline.createInterface({ input: approvalWorker.child.stdout });
approvalRl.on('line', (line) => {
  try {
    const frame = JSON.parse(line);
    approvalFrames.push(frame);
    const result = frame.result;
    if (result?.event === 'stream.done') {
      approvalWorker.child.kill();
    }
  } catch {
    // ignore malformed frames
  }
});
approvalWorker.child.stdin.write(
  `${JSON.stringify({
    jsonrpc: '2.0',
    id: 21,
    method: 'sdkwork/capability.invoke',
    params: {
      operation: {
        operation: 'model_chat_stream',
        model_request_id: 'hermes-approval-1',
        messages: ['trigger approval'],
        timeout_ms: 15_000,
      },
    },
  })}\n`,
);
// Wait for the agent.policy.paused kernel event to surface the interaction.
await new Promise((resolve, reject) => {
  const deadline = Date.now() + 10_000;
  const poll = setInterval(() => {
    const paused = approvalFrames.some(
      (frame) => frame.result?.event === 'stream.event'
        && frame.result.kernel_event?.event_type === 'agent.policy.paused',
    );
    if (paused) {
      clearInterval(poll);
      resolve();
    } else if (Date.now() > deadline) {
      clearInterval(poll);
      reject(new Error('agent.policy.paused event did not arrive'));
    }
  }, 100);
});
const pausedFrame = approvalFrames.find(
  (frame) => frame.result?.event === 'stream.event'
    && frame.result.kernel_event?.event_type === 'agent.policy.paused',
);
const interaction = pausedFrame.result.kernel_event.payload.interaction;
assert.equal(interaction.kind, 'approval');
assert.equal(interaction.category, 'approval');

const respondResultPromise = new Promise((resolve) => {
  approvalRl.on('line', (line) => {
    try {
      const frame = JSON.parse(line);
      if (frame.id === 99) {
        resolve(frame);
      }
    } catch {
      // ignore malformed frames
    }
  });
});
approvalWorker.child.stdin.write(
  `${JSON.stringify({
    jsonrpc: '2.0',
    id: 99,
    method: 'sdkwork/serverRequest.respond',
    params: {
      model_request_id: 'hermes-approval-1',
      session_id: '',
      turn_id: '',
      provider_session_id: '20260802_120000_abc123',
      provider_turn_id: '',
      provider_request_id: 42,
      resolution: { choice: 'allow' },
    },
  })}\n`,
);
const respondResult = await respondResultPromise;
assert.equal(respondResult.result.ok, true);
assert.equal(respondResult.result.interaction_kind, 'approval');
assert.equal(respondResult.result.status, 'responded');

// The turn completes after the approval is resolved.
await new Promise((resolve, reject) => {
  const deadline = Date.now() + 10_000;
  const poll = setInterval(() => {
    const done = approvalFrames.some(
      (frame) => frame.result?.event === 'stream.done',
    );
    if (done) {
      clearInterval(poll);
      resolve();
    } else if (Date.now() > deadline) {
      clearInterval(poll);
      reject(new Error('stream.done did not arrive after approval resolution'));
    }
  }, 100);
});
const approvalDone = approvalFrames.find((frame) => frame.result?.event === 'stream.done');
assert.equal(approvalDone.result.finish_reason, 'stop');
approvalWorker.child.kill();

fs.rmSync(gatewayRoot, { recursive: true, force: true });

console.log('generic-python-sdk-worker production fail-closed + Hermes TUI gateway contract passed.');
