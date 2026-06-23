import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('kernel UI services expose auth provider seam', () => {
  const authProviderPath = path.join(
    root,
    'packages/sdkwork-kernel-ui-services/src/service/kernel-ui-auth.provider.ts'
  );
  const realClientPath = path.join(root, 'packages/sdkwork-kernel-ui-services/src/service/kernel-ui.real.ts');

  assert.equal(fs.existsSync(authProviderPath), true);
  const authProviderSource = fs.readFileSync(authProviderPath, 'utf8');
  assert.match(authProviderSource, /createStaticKernelUiAuthProvider/);
  assert.match(authProviderSource, /createBrowserStorageKernelUiAuthProvider/);
  assert.match(authProviderSource, /readBrowserKernelUiAuthSession/);
  assert.match(authProviderSource, /clearBrowserKernelUiAuthSession/);
  assert.match(authProviderSource, /buildKernelUiAuthHeaders/);

  const realClientSource = fs.readFileSync(realClientPath, 'utf8');
  assert.match(realClientSource, /auth\?: KernelUiAuthProvider/);
  assert.match(realClientSource, /@sdkwork\/agent-internal-sdk/);
  assert.match(realClientSource, /createClient\(/);
  assert.match(realClientSource, /buildKernelUiAuthHeaders/);
  assert.match(authProviderSource, /x-api-key/);
  assert.match(authProviderSource, /x-sdkwork-identity-mac/);
  assert.match(realClientSource, /response\.items \?\? \[\]/);
  assert.doesNotMatch(realClientSource, /\/api\/kernel\//);
  assert.doesNotMatch(realClientSource, /\/api\/sessions/);
  assert.doesNotMatch(realClientSource, /\/api\/chat/);
  assert.doesNotMatch(realClientSource, /new EventSource\(/);
});

test('kernel UI commons exposes english i18n catalog', () => {
  const i18nPath = path.join(root, 'packages/sdkwork-kernel-ui-commons/src/i18n/kernel-ui.en.ts');
  assert.equal(fs.existsSync(i18nPath), true);
  const source = fs.readFileSync(i18nPath, 'utf8');
  assert.match(source, /translateKernelUi/);
  assert.match(source, /permission\.pending/);
  assert.match(source, /auth\.save/);
});

test('kernel UI shell exposes session gate for remote API without env tokens', () => {
  const appPath = path.join(root, 'src/App.tsx');
  const clientPath = path.join(root, 'src/kernel-ui-client.ts');
  const panelPath = path.join(root, 'src/KernelUiSessionPanel.tsx');
  assert.equal(fs.existsSync(panelPath), true);
  const appSource = fs.readFileSync(appPath, 'utf8');
  const clientSource = fs.readFileSync(clientPath, 'utf8');
  assert.match(appSource, /KernelUiSessionPanel/);
  assert.match(appSource, /needsKernelUiSessionGate/);
  assert.match(clientSource, /needsKernelUiSessionGate/);
  assert.match(clientSource, /readBrowserKernelUiAuthSession/);
  assert.match(clientSource, /VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL/);
  assert.match(clientSource, /createKernelUiClient\(/);
});

test('kernel UI types export auth contracts', () => {
  const typesPath = path.join(root, 'packages/sdkwork-kernel-ui-types/src/auth/kernel-ui-auth.types.ts');
  const indexPath = path.join(root, 'packages/sdkwork-kernel-ui-types/src/index.ts');
  assert.equal(fs.existsSync(typesPath), true);
  const indexSource = fs.readFileSync(indexPath, 'utf8');
  assert.match(indexSource, /KernelUiAuthProvider/);
});
