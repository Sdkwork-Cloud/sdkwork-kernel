import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { invokeModelChatLive, resolvePackageExportSpecifier } from './engine-sdk-live.mjs';

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'opencode-repro-'));
const mirror = path.join(tempRoot, 'opencode-sdk');
fs.mkdirSync(mirror, { recursive: true });
fs.writeFileSync(path.join(mirror, 'package.json'), JSON.stringify({
  type: 'module', name: '@opencode-ai/sdk', version: '0.0.0-test',
  exports: { '.': './index.js', './v2': './v2.js' },
}));
fs.writeFileSync(path.join(mirror, 'index.js'), `
export async function createOpencodeServer(options = {}) {
  return { url: 'http://127.0.0.1:4096', async close() {} };
}
export function createOpencodeClient(options = {}) {
  return { session: { list: async () => ({ data: [] }), messages: async () => ({ data: [] }) } };
}
`);
fs.writeFileSync(path.join(mirror, 'v2.js'), `
export function createOpencodeClient(options = {}) {
  return { session: { list: async () => ({ data: [] }), messages: async () => ({ data: [] }) } };
}
`);
process.env.SDKWORK_AGENT_SDK_PACKAGE_PATHS = JSON.stringify({ '@opencode-ai/sdk': mirror });
console.log('v2 specifier:', resolvePackageExportSpecifier('@opencode-ai/sdk', './v2'));
console.log('OPENCODE_SERVER_URL env:', process.env.OPENCODE_SERVER_URL ?? '(unset)');
const result = await invokeModelChatLive('@opencode-ai/sdk', {
  model_request_id: 'req-repro',
  session_id: 'session-canonical-repro',
  model_id: 'opencode/big-pickle',
  working_directory: 'C:/sdkwork/opencode-workspace',
  timeout_ms: 2_000,
  messages: ['OpenCode prompt'],
  execution_options: { approval_policy: 'allow-edits' },
});
console.log('RESULT ok=', result.ok, 'mode=', result.mode, 'messages=', result.messages);
