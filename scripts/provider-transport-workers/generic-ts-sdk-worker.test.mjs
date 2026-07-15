import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
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

console.log('generic-ts-sdk-worker production fail-closed contract passed.');
