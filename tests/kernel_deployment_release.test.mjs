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
  assert.match(compose, /SDKWORK_AGENT_RUNTIME_DATABASE_ENGINE:\s*postgres/);
  assert.match(compose, /SDKWORK_RATE_LIMIT_REDIS_URL:/);
});

test('app manifest requires SBOM and checksum evidence', () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, 'sdkwork.app.config.json'), 'utf8'));
  assert.equal(manifest.security?.sbomRequired, true);
  assert.equal(manifest.security?.checksumRequired, true);
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
