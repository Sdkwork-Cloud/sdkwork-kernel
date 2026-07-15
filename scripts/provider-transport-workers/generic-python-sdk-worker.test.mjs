import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import readline from 'node:readline';

function invokeWorker(operation, env = {}, packageName = 'sdkwork_missing_python_sdk') {
  return new Promise((resolve, reject) => {
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
  { operation: 'model_chat', model_request_id: 'model-1', messages: ['hello'] },
  { operation: 'tool_invoke', tool_id: 'tool-1' },
  { operation: 'skill_invoke', skill_id: 'skill-1' },
  { operation: 'unknown_operation' },
]) {
  const response = await invokeWorker(operation);
  assert.equal(response.result.ok, false, `${operation.operation} must fail closed`);
  if (operation.operation === 'tool_invoke' || operation.operation === 'skill_invoke' || operation.operation === 'unknown_operation') {
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

const gatewayRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-hermes-gateway-worker-'));
const gatewayPackage = path.join(gatewayRoot, 'tui_gateway');
fs.mkdirSync(gatewayPackage, { recursive: true });
fs.writeFileSync(path.join(gatewayPackage, '__init__.py'), '', 'utf8');
fs.writeFileSync(
  path.join(gatewayPackage, 'entry.py'),
  `import json
import sys

for line in sys.stdin:
    request = json.loads(line)
    if request.get('method') == 'llm.oneshot':
        text = request.get('params', {}).get('input', '')
        print(json.dumps({
            'jsonrpc': '2.0',
            'id': request.get('id'),
            'result': {'text': 'gateway:' + text},
        }), flush=True)
`,
  'utf8',
);
const gatewayResponse = await invokeWorker(
  {
    operation: 'model_chat',
    model_request_id: 'hermes-gateway-1',
    messages: ['legacy prompt'],
    wire_messages: [{ role: 'user', content: [{ type: 'text', text: 'gateway prompt' }] }],
    timeout_ms: 5_000,
  },
  {
    PYTHONPATH: [gatewayRoot, process.env.PYTHONPATH].filter(Boolean).join(path.delimiter),
  },
  'tui_gateway',
);
assert.equal(gatewayResponse.result.ok, true, 'Hermes TUI gateway should execute a real JSON-RPC model call');
assert.equal(gatewayResponse.result.mode, 'sdk_live');
assert.equal(gatewayResponse.result.gateway_method, 'llm.oneshot');
assert.deepEqual(gatewayResponse.result.messages, ['gateway:gateway prompt']);

const unsupportedModelResponse = await invokeWorker(
  {
    operation: 'model_chat',
    model_request_id: 'hermes-gateway-model-id',
    model_id: 'unsupported-by-oneshot',
    messages: ['reject model override'],
  },
  {
    PYTHONPATH: [gatewayRoot, process.env.PYTHONPATH].filter(Boolean).join(path.delimiter),
  },
  'tui_gateway',
);
assert.equal(unsupportedModelResponse.result.ok, false);
assert.match(unsupportedModelResponse.result.error, /does not support per-request model selection/);
fs.rmSync(gatewayRoot, { recursive: true, force: true });

console.log('generic-python-sdk-worker production fail-closed contract passed.');
