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
  assert.equal(spec.defaults.developmentProfileId, 'standalone.unified-process.development');
  assert.ok(spec.surfaces['application.public-ingress']);
  assert.ok(spec.surfaces['platform.api-gateway']);

  for (const profileId of [
    'standalone.split-services.development',
    'standalone.unified-process.development',
    'standalone.split-services.production',
    'standalone.unified-process.production',
    'cloud.split-services.development',
    'cloud.split-services.production',
  ]) {
    const profilePath = spec.profileFiles[profileId];
    assert.equal(await exists(profilePath), true, `${profilePath} should exist`);
    const profileEnv = await read(profilePath);
    assert.match(profileEnv, /SDKWORK_KERNEL_PROFILE_ID=/);
    assert.match(profileEnv, /SDKWORK_KERNEL_AGENT_PLUGIN=/);
    assert.match(profileEnv, /VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL=/);
    assert.match(profileEnv, /VITE_SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL=/);
  }
});

test('root package.json wires @sdkwork/app-topology and standard dev scripts', async () => {
  const packageJson = await readJson('package.json');
  const topologyDep = packageJson.dependencies['@sdkwork/app-topology'];
  assert.ok(
    topologyDep === 'file:../sdkwork-app-topology' || topologyDep === 'workspace:*',
    'package.json must depend on @sdkwork/app-topology via file:../sdkwork-app-topology or workspace:*'
  );
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
  const bootstrapSource = await read('sdkwork-agent-server/src/runtime_bootstrap.rs');
  assert.match(configSource, /SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND/);
  assert.match(configSource, /SDKWORK_KERNEL_DEPLOYMENT_PROFILE/);
  assert.match(configSource, /SDKWORK_KERNEL_PROFILE_ID/);
  assert.match(configSource, /SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE/);
  assert.match(configSource, /SDKWORK_RATE_LIMIT_REDIS_URL/);
  assert.match(configSource, /SDKWORK_KERNEL_ALLOW_MOCK_PROVIDERS/);
  assert.match(configSource, /sdkwork_agent_kernel::is_production_kernel_profile/);
  assert.match(bootstrapSource, /SDKWORK_KERNEL_AGENT_PLUGIN/);
  assert.match(bootstrapSource, /parse_kernel_agent_plugin_kind/);
  assert.doesNotMatch(configSource, /ends_with\("\.production"\)/);
  assert.doesNotMatch(configSource, /SDKWORK_KERNEL_HOSTING/);
  assert.doesNotMatch(configSource, /SDKWORK_BIND_ADDRESS/);
});

test('agent server uses topology-aligned production profile detection', async () => {
  const configSource = await read('sdkwork-agent-server/src/config.rs');
  const preflightSource = await read('sdkwork-agent-server/src/preflight.rs');
  assert.match(configSource, /pub fn is_production_kernel_profile/);
  assert.match(configSource, /requires_distributed_rate_limit[\s\S]*is_production_kernel_profile/);
  assert.match(configSource, /requires_postgres_runtime_database[\s\S]*is_production_kernel_profile/);
  assert.match(preflightSource, /is_production_kernel_profile/);
});

test('adapter-core and agent-server share canonical kernel runtime topology policy', async () => {
  const kernelTopology = await read('sdkwork-agent-kernel/src/runtime_topology.rs');
  const adapterPolicy = await read(
    'sdkwork-kernel-plugins/crates/sdkwork-agent-provider-core/src/mock_policy.rs',
  );
  const serverConfig = await read('sdkwork-agent-server/src/config.rs');
  const agentRegistry = await read('sdkwork-agent-server/src/agent_registry.rs');
  assert.match(kernelTopology, /pub fn is_production_kernel_profile/);
  assert.match(kernelTopology, /ends_with\("\.production"\)/);
  assert.match(kernelTopology, /ALLOW_MOCK_PROVIDERS_ENV/);
  assert.match(adapterPolicy, /mock_provider_invocation_allowed_from_env/);
  assert.match(serverConfig, /sdkwork_agent_kernel::is_production_kernel_profile/);
  assert.match(serverConfig, /sdkwork_agent_kernel::mock_provider_invocation_allowed/);
  assert.match(agentRegistry, /kernel_agent_plugin_kind_from_env/);
  assert.match(agentRegistry, /active_hosted_agent/);
});

test('cloud production topology documents postgres and redis runtime deps', async () => {
  const profileEnv = await read('configs/topology/cloud.split-services.production.env');
  assert.match(profileEnv, /SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres/);
  assert.match(profileEnv, /SDKWORK_AGENT_RUNTIME_DATABASE_URL/);
  assert.match(profileEnv, /SDKWORK_RATE_LIMIT_REDIS_URL/);
});

test('self-hosted production topology documents postgres and redis runtime deps', async () => {
  const profileEnv = await read('configs/topology/standalone.split-services.production.env');
  assert.match(profileEnv, /SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres/);
  assert.match(profileEnv, /SDKWORK_AGENT_RUNTIME_DATABASE_URL/);
  assert.match(profileEnv, /SDKWORK_RATE_LIMIT_REDIS_URL/);
});

test('all production topology profiles enforce token ingress and scale-out deps', async () => {
  const spec = await readJson('specs/topology.spec.json');
  const productionProfiles = Object.keys(spec.profileFiles).filter((id) => id.endsWith('.production'));
  assert.equal(productionProfiles.length, 3);
  for (const profileId of productionProfiles) {
    const profileEnv = await read(spec.profileFiles[profileId]);
    assert.match(profileEnv, /SDKWORK_KERNEL_ENVIRONMENT=production/);
    assert.match(profileEnv, /SDKWORK_KERNEL_INGRESS_AUTH_MODE=token/);
    assert.match(profileEnv, /SDKWORK_RATE_LIMIT_RPS=100/);
    assert.match(profileEnv, /SDKWORK_RATE_LIMIT_BURST=200/);
    assert.match(profileEnv, /SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE=postgres/);
    assert.match(profileEnv, /SDKWORK_RATE_LIMIT_REDIS_URL=/);
    assert.match(profileEnv, /SDKWORK_KERNEL_AGENT_PLUGIN=rig/);
  }
});

const ALLOWED_KERNEL_AGENT_PLUGINS = new Set(['rig', 'openclaw', 'hermes', 'codex']);

test('topology profiles declare supported kernel agent plugin values', async () => {
  const spec = await readJson('specs/topology.spec.json');
  for (const profileId of Object.keys(spec.profileFiles)) {
    const profileEnv = await read(spec.profileFiles[profileId]);
    const match = profileEnv.match(/^SDKWORK_KERNEL_AGENT_PLUGIN=(.+)$/m);
    assert.ok(match, `${profileId} must set SDKWORK_KERNEL_AGENT_PLUGIN`);
    const plugin = match[1].trim();
    assert.equal(
      ALLOWED_KERNEL_AGENT_PLUGINS.has(plugin),
      true,
      `${profileId} uses unsupported SDKWORK_KERNEL_AGENT_PLUGIN=${plugin}`,
    );
  }
});

test('topology smoke probes canonical internal-api snapshot path only', async () => {
  const smokeScript = await read('scripts/dev/sdkwork-kernel-topology-smoke.mjs');
  assert.match(
    smokeScript,
    /\/internal\/v3\/api\/intelligence\/runtime\/snapshot/,
    'topology smoke must probe canonical internal-api snapshot'
  );
  assert.doesNotMatch(
    smokeScript,
    /\/api\/kernel\/snapshot/,
    'topology smoke must not probe retired legacy alias'
  );
  assert.doesNotMatch(
    smokeScript,
    /\/api\/sessions/,
    'topology smoke must not probe retired legacy session API'
  );
});
