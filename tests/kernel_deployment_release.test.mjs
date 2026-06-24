import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');

const requiredDeploymentArtifacts = [
  'deployments/docker/Dockerfile',
  'deployments/docker/docker-compose.cloud.yml',
  'deployments/kubernetes/deployment.yaml',
  'deployments/kubernetes/service.yaml',
  'deployments/kubernetes/configmap.yaml',
  'deployments/kubernetes/pvc.yaml',
  'deployments/kubernetes/postgres-redis.yaml',
  'deployments/runbooks/production-rollout.md',
  'scripts/release/generate-kernel-sbom.mjs',
  'scripts/release/generate-kernel-checksums.mjs',
  'scripts/release/validate-release-artifacts.mjs',
];

test('production deployment and release evidence artifacts exist', () => {
  for (const relativePath of requiredDeploymentArtifacts) {
    const absolutePath = path.join(root, relativePath);
    assert.equal(fs.existsSync(absolutePath), true, `${relativePath} should exist`);
    assert.ok(fs.statSync(absolutePath).size > 0, `${relativePath} should not be empty`);
  }
});

test('cloud compose provisions postgres and redis for agent-server', () => {
  const compose = fs.readFileSync(
    path.join(root, 'deployments/docker/docker-compose.cloud.yml'),
    'utf8',
  );
  assert.match(compose, /^\s*postgres:/m);
  assert.match(compose, /^\s*redis:/m);
  assert.match(compose, /env_file:/);
  assert.match(compose, /configs\/topology\/cloud\.split-services\.production\.env/);
  assert.match(compose, /SDKWORK_AGENT_RUNTIME_DATABASE_URL:/);
  assert.doesNotMatch(compose, /SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL:\s*http:\/\/127\.0\.0\.1:18280/);
  assert.doesNotMatch(compose, /SDKWORK_KERNEL_HOSTING/);
  assert.doesNotMatch(compose, /SDKWORK_BIND_ADDRESS/);
});

test('production rollout runbook uses topology public HTTP env without hardcoded fallback', () => {
  const runbook = fs.readFileSync(
    path.join(root, 'deployments/runbooks/production-rollout.md'),
    'utf8',
  );
  assert.match(runbook, /SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL/);
  assert.match(runbook, /SDKWORK_KERNEL_AGENT_PLUGIN=rig/);
  assert.doesNotMatch(runbook, /http:\/\/127\.0\.0\.1:18280/);
});

test('production docker image defaults to cloud deployment profile', () => {
  const dockerfile = fs.readFileSync(
    path.join(root, 'deployments/docker/Dockerfile'),
    'utf8',
  );
  assert.match(dockerfile, /SDKWORK_KERNEL_DEPLOYMENT_PROFILE=cloud/);
  assert.match(dockerfile, /SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND=0\.0\.0\.0:18280/);
  assert.match(dockerfile, /SDKWORK_KERNEL_AGENT_PLUGIN=rig/);
  assert.doesNotMatch(dockerfile, /SDKWORK_KERNEL_HOSTING/);
  assert.doesNotMatch(dockerfile, /SDKWORK_BIND_ADDRESS/);
});

test('kubernetes configmap documents cloud deployment profile', () => {
  const configMap = fs.readFileSync(
    path.join(root, 'deployments/kubernetes/configmap.yaml'),
    'utf8',
  );
  assert.match(configMap, /SDKWORK_KERNEL_DEPLOYMENT_PROFILE:\s*cloud/);
  assert.match(configMap, /SDKWORK_KERNEL_APPLICATION_PUBLIC_INGRESS_BIND:\s*0\.0\.0\.0:18280/);
  assert.match(configMap, /SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE:\s*postgres/);
  assert.match(configMap, /SDKWORK_KERNEL_AGENT_PLUGIN:\s*rig/);
  assert.match(configMap, /SDKWORK_RATE_LIMIT_REDIS_URL:/);
  assert.doesNotMatch(configMap, /SDKWORK_KERNEL_HOSTING/);
  assert.doesNotMatch(configMap, /SDKWORK_BIND_ADDRESS/);
});

test('app manifest requires SBOM and checksum evidence', () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, 'sdkwork.app.config.json'), 'utf8'));
  assert.equal(manifest.security?.sbomRequired, true);
  assert.equal(manifest.security?.checksumRequired, true);
  assert.equal(manifest.metadata?.topologySpec, 'specs/topology.spec.json');
  assert.equal(
    manifest.environments?.development?.topologyProfileId,
    'standalone.split-services.development',
  );
  assert.equal(
    manifest.environments?.production?.topologyProfileId,
    'cloud.split-services.production',
  );
  assert.equal(
    manifest.environments?.production?.accessUrlEnv,
    'SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL',
  );
});

test('workflow declares SBOM generation and release validation', () => {
  const workflow = JSON.parse(fs.readFileSync(path.join(root, 'sdkwork.workflow.json'), 'utf8'));
  const sbomSteps = workflow.lifecycle?.sbom ?? [];
  const validateSteps = workflow.lifecycle?.validate ?? [];
  assert.ok(
    sbomSteps.some((step) => step.run?.includes('generate-kernel-sbom.mjs')),
    'workflow should generate SBOM',
  );
  assert.ok(
    validateSteps.some((step) => step.run?.includes('validate-release-artifacts.mjs')),
    'workflow should validate release artifacts',
  );
  assert.equal(workflow.security?.sbomRequired, true);
});

test('kernel verification workflow checks out platform sibling repositories', () => {
  const workflow = fs.readFileSync(
    path.join(root, '.github/workflows/kernel-verification.yml'),
    'utf8',
  );
  for (const sibling of [
    'sdkwork-database',
    'sdkwork-utils',
    'sdkwork-web-framework',
    'sdkwork-iam',
  ]) {
    assert.match(
      workflow,
      new RegExp(sibling),
      `kernel-verification.yml should checkout ${sibling}`,
    );
  }
  assert.match(
    workflow,
    /Link platform siblings for path dependencies/,
    'kernel-verification.yml should link sibling repos for Cargo and pnpm path deps',
  );
});
