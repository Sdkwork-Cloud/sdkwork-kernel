import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';

const ROOT = process.cwd();

async function exists(relativePath) {
  try {
    await stat(path.join(ROOT, relativePath));
    return true;
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return false;
    }
    throw error;
  }
}

async function read(relativePath) {
  return readFile(path.join(ROOT, relativePath), 'utf8');
}

async function readJson(relativePath) {
  return JSON.parse(await read(relativePath));
}

test('declares v2 topology spec and profile env files for sdkwork-kernel', async () => {
  assert.equal(await exists('specs/topology.spec.json'), true);
  assert.equal(await exists('scripts/lib/kernel-topology.mjs'), true);
  assert.equal(await exists('scripts/kernel-dev.mjs'), true);
  assert.equal(await exists('docs/topology-standard.md'), true);

  const spec = await readJson('specs/topology.spec.json');
  assert.equal(spec.schemaVersion, 2);
  assert.equal(spec.kind, 'sdkwork.app.topology');
  assert.equal(spec.appId, 'sdkwork-kernel');
  assert.equal(spec.archetype, 'realtime-application-platform');
  assert.equal(spec.defaults.developmentProfileId, 'self-hosted.split-services.development');
  assert.ok(spec.surfaces['application.public-ingress']);
  assert.ok(spec.surfaces['platform.api-gateway']);

  for (const profileId of [
    'self-hosted.split-services.development',
    'self-hosted.unified-process.development',
    'self-hosted.split-services.production',
    'cloud-hosted.split-services.development',
    'cloud-hosted.split-services.production',
  ]) {
    const profilePath = spec.profileFiles[profileId];
    assert.equal(await exists(profilePath), true, `${profilePath} should exist`);
    const profileEnv = await read(profilePath);
    assert.match(profileEnv, /SDKWORK_KERNEL_PROFILE_ID=/);
    assert.match(profileEnv, /VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL=/);
    assert.match(profileEnv, /VITE_SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL=/);
  }
});

test('root package.json wires @sdkwork/app-topology and standard dev scripts', async () => {
  const packageJson = await readJson('package.json');
  assert.equal(packageJson.dependencies['@sdkwork/app-topology'], 'file:../sdkwork-app-topology');
  assert.match(packageJson.scripts.dev, /scripts\/sdkwork-command\.mjs dev/);
  assert.match(packageJson.scripts['topology:validate'], /sdkwork-topology\.mjs validate/);
});

test('kernel dev orchestrator rejects retired --topology and --hosting flags', async () => {
  const devScript = await read('scripts/kernel-dev.mjs');
  assert.match(devScript, /--topology is retired/);
  assert.match(devScript, /--hosting is retired/);
});

test('agent server reads topology bind env keys', async () => {
  const configSource = await read('sdkwork-agent-server/src/config.rs');
  assert.match(configSource, /SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND/);
});

test('kernel UI client prefers topology surface env keys', async () => {
  const clientSource = await read('sdkwork-kernel-ui/src/kernel-ui-client.ts');
  assert.match(clientSource, /VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL/);
  assert.match(clientSource, /VITE_KERNEL_API_URL/);
});

test('topology smoke probes canonical internal-api snapshot path', async () => {
  const smokeScript = await read('scripts/dev/sdkwork-kernel-topology-smoke.mjs');
  assert.match(
    smokeScript,
    /\/internal\/v3\/api\/intelligence\/runtime\/snapshot/,
    'topology smoke must probe canonical internal-api snapshot'
  );
  assert.match(
    smokeScript,
    /\/api\/kernel\/snapshot/,
    'topology smoke must retain legacy alias parity probe'
  );
});
