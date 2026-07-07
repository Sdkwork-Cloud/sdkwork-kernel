import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import readline from 'node:readline';

function invokeWorker(operation, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      'python',
      [
        'scripts/provider-transport-workers/generic_python_sdk_worker.py',
        '--package',
        'sdkwork_missing_python_sdk',
      ],
      {
        env: {
          ...process.env,
          SDKWORK_KERNEL_PROFILE_ID: 'cloud.split-services.production',
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
  assert.equal(response.result.mode, 'sdk_live_failed');
  assert.match(response.result.error, /mock fallback is disabled|unsupported operation/);
}

const devResponse = await invokeWorker(
  { operation: 'model_chat', model_request_id: 'model-1', messages: ['hello'] },
  {
    SDKWORK_KERNEL_PROFILE_ID: 'standalone.unified-process.development',
    SDKWORK_KERNEL_ENVIRONMENT: 'development',
  },
);
assert.equal(devResponse.result.ok, true, 'development profile can still use SDK probe fallback');

console.log('generic-python-sdk-worker production fail-closed contract passed.');
