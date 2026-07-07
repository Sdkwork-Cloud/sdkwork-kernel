import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function writeReleaseFixture({ workspaceRoot, packageId, binaryRelativePath, legacyEvidence = false }) {
  const binaryPath = path.join(workspaceRoot, binaryRelativePath);
  fs.mkdirSync(path.dirname(binaryPath), { recursive: true });
  fs.writeFileSync(binaryPath, 'release-binary-fixture', 'utf8');

  const evidencePackageId = legacyEvidence ? 'sdkwork-agent-server' : packageId;
  const releaseDir = path.join(workspaceRoot, 'dist', 'release', evidencePackageId);
  fs.mkdirSync(releaseDir, { recursive: true });
  writeJson(path.join(releaseDir, `${evidencePackageId}-0.1.0.cyclonedx.json`), {
    bomFormat: 'CycloneDX',
    specVersion: '1.5',
    version: 1,
    metadata: {
      component: {
        type: 'application',
        name: evidencePackageId,
        version: '0.1.0',
      },
    },
    components: [],
  });
  const checksum = createHash('sha256').update(fs.readFileSync(binaryPath)).digest('hex');
  fs.writeFileSync(
    path.join(releaseDir, `${evidencePackageId}-0.1.0.sha256`),
    `${checksum}  ${path.basename(binaryPath)}\n`,
    'utf8',
  );
}

function writeReleaseWorkspaceFixture(workspaceRoot) {
  const packageId = 'windows-x64-cloud-server-zip';
  const binaryRelativePath = `dist/release/${packageId}/sdkwork-kernel-${packageId}-0.1.0.zip`;
  writeJson(path.join(workspaceRoot, 'sdkwork.app.config.json'), {
    schemaVersion: 3,
    kind: 'sdkwork.app',
    app: { key: 'sdkwork-kernel' },
    runtime: {
      supportedDeploymentProfiles: ['cloud'],
      defaultDeploymentProfile: 'cloud',
    },
    environments: {
      production: {
        topologyProfileId: 'cloud.split-services.production',
        accessUrlEnv: 'SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL',
      },
    },
    artifacts: {
      installConfig: {
        packages: [
          {
            id: packageId,
            name: 'SDKWork Kernel Windows x64 Server',
            sourceType: 'BINARY_URL',
            packageFormat: 'ZIP',
            platform: 'API',
            architecture: 'x64',
            deploymentProfile: 'cloud',
            runtimeTarget: 'server',
            enabled: true,
          },
        ],
      },
    },
    release: {
      currentVersion: '0.1.0',
      notes: [
        {
          version: '0.1.0',
          channel: 'STABLE',
          current: true,
          packageIds: [packageId],
        },
      ],
    },
    security: {
      checksumRequired: true,
      sbomRequired: true,
      signatureRequired: false,
    },
    metadata: {
      topologySpec: 'specs/topology.spec.json',
    },
  });
  writeJson(path.join(workspaceRoot, 'sdkwork.workflow.json'), {
    schemaVersion: '2026-06-06.sdkwork.workflow.v1',
    app: {
      id: 'sdkwork-kernel',
      repository: 'Sdkwork-Cloud/sdkwork-kernel',
      sourcePath: '.',
    },
    release: {
      artifactPrefix: 'sdkwork-kernel',
      defaultVersion: '0.1.0',
    },
    lifecycle: {},
    targets: [
      {
        id: packageId,
        profile: 'server',
        platform: 'windows',
        architecture: 'x64',
        formats: ['zip'],
        runner: 'windows-2022',
        deploymentProfile: 'cloud',
        runtimeTarget: 'server',
        outputGlobs: [binaryRelativePath],
      },
    ],
    security: {
      artifactAttestations: true,
      checksumRequired: true,
      sbomRequired: true,
      signingRequired: false,
    },
  });

  return { packageId, binaryRelativePath };
}

function runReleaseValidator(workspaceRoot, env = {}) {
  return spawnSync(process.execPath, [path.join(root, 'scripts/release/validate-release-artifacts.mjs')], {
    cwd: workspaceRoot,
    env: { ...process.env, ...env },
    encoding: 'utf8',
  });
}

const requiredDeploymentArtifacts = [
  'deployments/docker/Dockerfile',
  'deployments/docker/docker-compose.cloud.yml',
  'deployments/kubernetes/deployment.yaml',
  'deployments/kubernetes/service.yaml',
  'deployments/kubernetes/configmap.yaml',
  'deployments/kubernetes/pvc.yaml',
  'deployments/kubernetes/postgres-redis.yaml',
  'deployments/runbooks/production-rollout.md',
  'scripts/release/package-kernel-artifact.mjs',
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
  assert.match(compose, /SDKWORK_RATE_LIMIT_REDIS_URL:/);
  assert.match(compose, /requirepass/);
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
  assert.doesNotMatch(configMap, /SDKWORK_RATE_LIMIT_REDIS_URL:/);
  assert.doesNotMatch(configMap, /SDKWORK_AGENT_RUNTIME_DATABASE_URL:/);
  assert.doesNotMatch(configMap, /SDKWORK_KERNEL_HOSTING/);
  assert.doesNotMatch(configMap, /SDKWORK_BIND_ADDRESS/);
});

test('kubernetes deployment injects runtime database and redis URLs from secrets', () => {
  const deployment = fs.readFileSync(
    path.join(root, 'deployments/kubernetes/deployment.yaml'),
    'utf8',
  );
  assert.match(deployment, /SDKWORK_AGENT_RUNTIME_DATABASE_URL/);
  assert.match(deployment, /runtime-database-url/);
  assert.match(deployment, /SDKWORK_RATE_LIMIT_REDIS_URL/);
  assert.match(deployment, /runtime-redis-url/);
});

test('kubernetes data dependency manifests are not presented as production HA databases', () => {
  const dataManifest = fs.readFileSync(
    path.join(root, 'deployments/kubernetes/postgres-redis.yaml'),
    'utf8',
  );
  assert.match(dataManifest, /sdkwork\.com\/production-suitability:\s*"reference-only"/);
  assert.match(dataManifest, /managed HA Postgres/);
  assert.match(dataManifest, /managed HA Redis/);

  const runbook = fs.readFileSync(
    path.join(root, 'deployments/runbooks/production-rollout.md'),
    'utf8',
  );
  assert.match(runbook, /managed HA Postgres/);
  assert.match(runbook, /managed HA Redis/);
  assert.match(runbook, /postgres-redis\.yaml.*local\/staging/i);
  assert.doesNotMatch(runbook, /Apply `deployments\/kubernetes\/postgres-redis\.yaml` \(or connect to managed Postgres\/Redis\)/);
});

test('app manifest requires SBOM and checksum evidence', () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, 'sdkwork.app.config.json'), 'utf8'));
  assert.equal(manifest.security?.sbomRequired, true);
  assert.equal(manifest.security?.checksumRequired, true);
  assert.equal(manifest.metadata?.topologySpec, 'specs/topology.spec.json');
  assert.equal(
    manifest.environments?.development?.topologyProfileId,
    'standalone.unified-process.development',
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
  const packageSteps = workflow.lifecycle?.package ?? [];
  const sbomSteps = workflow.lifecycle?.sbom ?? [];
  const validateSteps = workflow.lifecycle?.validate ?? [];
  assert.ok(
    packageSteps.some((step) => step.run?.includes('package-kernel-artifact.mjs')),
    'workflow should build real package archives before release validation',
  );
  assert.ok(
    sbomSteps.some((step) => step.run?.includes('generate-kernel-sbom.mjs')),
    'workflow should generate SBOM',
  );
  assert.ok(
    validateSteps.some((step) => step.run?.includes('validate-release-artifacts.mjs')),
    'workflow should validate release artifacts',
  );
  assert.equal(workflow.security?.sbomRequired, true);
  for (const target of workflow.targets ?? []) {
    assert.ok(
      target.outputGlobs.some((glob) => glob.includes(`dist/release/${target.id}/`)),
      `${target.id} should publish target-scoped release evidence`,
    );
    assert.ok(
      target.outputGlobs.some((glob) => glob.includes('.sha256')),
      `${target.id} should publish checksum evidence`,
    );
    assert.ok(
      target.outputGlobs.some((glob) => glob.includes('.cyclonedx.json')),
      `${target.id} should publish SBOM evidence`,
    );
    assert.equal(
      target.outputGlobs.some((glob) => glob.startsWith('target/release/')),
      false,
      `${target.id} must not publish bare release binaries as final packages`,
    );
  }
});

test('release validation rejects legacy crate-scoped evidence that is not tied to manifest package ids', () => {
  const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-kernel-release-'));
  const { packageId, binaryRelativePath } = writeReleaseWorkspaceFixture(workspaceRoot);
  writeReleaseFixture({
    workspaceRoot,
    packageId,
    binaryRelativePath,
    legacyEvidence: true,
  });

  const result = runReleaseValidator(workspaceRoot);

  assert.notEqual(result.status, 0);
  assert.match(
    `${result.stdout}${result.stderr}`,
    /windows-x64-cloud-server-zip|SDKWORK_PACKAGE_ID|missing SBOM/,
  );
});

test('release validation accepts target-scoped evidence for the declared workflow package id', () => {
  const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'sdkwork-kernel-release-'));
  const { packageId, binaryRelativePath } = writeReleaseWorkspaceFixture(workspaceRoot);
  writeReleaseFixture({
    workspaceRoot,
    packageId,
    binaryRelativePath,
  });

  const result = runReleaseValidator(workspaceRoot, {
    SDKWORK_PACKAGE_ID: packageId,
    SDKWORK_PACKAGE_VERSION: '0.1.0',
  });

  assert.equal(
    result.status,
    0,
    `release validator should pass target-scoped evidence\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
  );
  assert.match(result.stdout, /Release artifact validation passed/);
});

test('commercial release verification requires live dependencies explicitly', () => {
  const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
  assert.equal(
    packageJson.scripts?.['verify:commercial'],
    'node scripts/verify-kernel-audit-remediation.mjs --commercial-release && pnpm run check:app-composition',
  );

  const verifier = fs.readFileSync(
    path.join(root, 'scripts/verify-kernel-audit-remediation.mjs'),
    'utf8',
  );
  assert.match(verifier, /--commercial-release/);
  assert.match(verifier, /SDKWORK_KERNEL_COMMERCIAL_RELEASE_VERIFY/);
  assert.match(verifier, /SDKWORK_AGENT_RUNTIME_POSTGRES_URI/);
  assert.match(verifier, /SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS/);
  assert.match(verifier, /SDKWORK_KERNEL_STAGING_LIVE_SDK/);
  assert.match(verifier, /commercial release verification requires live runtime PostgreSQL/);
  assert.match(verifier, /check-agent-workflow-standard\.mjs/);
  assert.match(verifier, /generic-ts-sdk-worker\.test\.mjs/);
  assert.match(verifier, /generic-python-sdk-worker\.test\.mjs/);
  assert.match(verifier, /sdkwork-agent-provider-transport-ipc\/Cargo\.toml/);
  assert.match(verifier, /sdkwork-agent-provider-transport-node\/Cargo\.toml/);
  assert.match(verifier, /sdkwork-agent-provider-transport-python\/Cargo\.toml/);

  const failedCommercialPreflight = spawnSync(
    process.execPath,
    ['scripts/verify-kernel-audit-remediation.mjs', '--commercial-release'],
    {
      cwd: root,
      env: { ...process.env, SDKWORK_AGENT_RUNTIME_POSTGRES_URI: '   ' },
      encoding: 'utf8',
      timeout: 10_000,
    },
  );
  assert.notEqual(failedCommercialPreflight.status, 0);
  assert.match(
    `${failedCommercialPreflight.stdout}${failedCommercialPreflight.stderr}`,
    /commercial release verification requires live runtime PostgreSQL/,
  );

  const runbook = fs.readFileSync(
    path.join(root, 'deployments/runbooks/production-rollout.md'),
    'utf8',
  );
  assert.match(runbook, /pnpm verify:commercial/);
  assert.doesNotMatch(runbook, /Live Postgres \(optional\)/);
});

test('commercial readiness docs distinguish merge and release dependency gates', () => {
  const readiness = fs.readFileSync(
    path.join(root, 'docs/product/prd/PRD-03-commercial-readiness-baseline.md'),
    'utf8',
  );
  assert.match(readiness, /Commercial release verification/);
  assert.match(readiness, /Production data plane HA/);
  assert.match(readiness, /Provider worker synthetic operation gate/);
  assert.match(readiness, /Rust transport crate tests for IPC\/Node\/Python/);
  assert.match(readiness, /pnpm verify:commercial/);
  assert.match(readiness, /commercial release verification fails closed/i);
  assert.match(readiness, /live runtime PostgreSQL/);
  assert.match(readiness, /managed HA Postgres/);
  assert.match(readiness, /managed HA Redis/);
  assert.doesNotMatch(
    readiness,
    /Enterprise GA readiness \| \*\*Pending\*\* \| REQ-2026-0001 artifact publishing \+ staging credential population/,
  );

  const requirement = fs.readFileSync(
    path.join(root, 'docs/product/requirements/REQ-2026-0001-commercial-hardening.md'),
    'utf8',
  );
  assert.match(requirement, /pnpm verify:commercial/);
  assert.match(requirement, /commercial release verification fails closed/i);
  assert.match(requirement, /SDKWORK_AGENT_RUNTIME_POSTGRES_URI/);
  assert.match(requirement, /SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS/);
  assert.match(requirement, /Rust IPC\/Node\/Python provider transport stubs/);
  assert.match(requirement, /managed HA Postgres/);
  assert.match(requirement, /managed HA Redis/);

  const prd = fs.readFileSync(path.join(root, 'docs/product/prd/PRD.md'), 'utf8');
  assert.match(prd, /Provision managed HA Postgres and managed HA Redis/);
  assert.match(prd, /pnpm verify:commercial/);
});

test('staging live SDK workflow is opt-in and credential-gated', () => {
  const workflow = fs.readFileSync(
    path.join(root, '.github/workflows/kernel-staging-live-sdk.yml'),
    'utf8',
  );
  assert.match(workflow, /workflow_dispatch:/);
  assert.match(workflow, /SDKWORK_KERNEL_STAGING_LIVE_SDK/);
  assert.match(workflow, /SDKWORK_KERNEL_STAGING_REQUIRE_CREDENTIALS/);
  assert.match(workflow, /engine-sdk-live-staging\.mjs/);
  assert.doesNotMatch(workflow, /push:\s*\n\s*branches:/);
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
    'sdkwork-id',
    'sdkwork-prompts',
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
