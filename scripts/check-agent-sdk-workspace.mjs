import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import {
  AGENT_SDK_FAMILIES,
  AGENT_SDK_OWNER,
  forbiddenAgentApiPrefixesFor
} from '../sdks/_shared/agent-sdk-families.mjs';
import {
  SDKWORK_SDKGEN_STANDARD,
  resolveSdkgenEntrypoint
} from '../sdks/_shared/sdkgen-standard.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sdksRoot = path.join(root, 'sdks');
const families = AGENT_SDK_FAMILIES;

const errors = [];

ensureFile('sdks/README.md');
ensureFile('sdks/materialize-agent-v3-openapi-boundaries.mjs');
ensureFile('sdks/workspace-agent-sdkgen.mjs');
ensureFile('specs/SDK_SPEC.md');

const sdkSpec = readIfExists(path.join(root, 'specs', 'SDK_SPEC.md'));
const sdkWorkspaceReadme = readIfExists(path.join(root, 'sdks', 'README.md'));
const workspaceSdkgen = readIfExists(path.join(root, 'sdks', 'workspace-agent-sdkgen.mjs'));
const sdkgenCommands = readIfExists(
  path.join(root, 'sdkwork-agent-business', 'specs', 'sdkgen', 'commands.md')
);
const sdkgenReport = readJsonIfExists(path.join(root, 'sdks', '.sdkgen-agent-workspace-report.json'));
const latestVerificationReport = readJsonIfExists(
  path.join(root, 'sdkwork-agent-business', 'specs', 'sdkgen', 'verification-latest.json')
);
const ciVerificationReport = readJsonIfExists(
  path.join(root, 'sdkwork-agent-business', 'specs', 'sdkgen', 'verification-ci.json')
);

for (const [label, content] of [
  ['specs/SDK_SPEC.md', sdkSpec],
  ['sdks/README.md', sdkWorkspaceReadme]
]) {
  if (!content.includes(SDKWORK_SDKGEN_STANDARD.canonicalRootWin)) {
    errors.push(
      `${label} must mention canonical sdkwork-sdk-generator root ${SDKWORK_SDKGEN_STANDARD.canonicalRootWin}`
    );
  }
  if (!content.includes(SDKWORK_SDKGEN_STANDARD.canonicalEntrypointWin)) {
    errors.push(
      `${label} must mention canonical sdkgen entrypoint ${SDKWORK_SDKGEN_STANDARD.canonicalEntrypointWin}`
    );
  }
}

if (!workspaceSdkgen.includes('resolveSdkgenEntrypoint')) {
  errors.push('sdks/workspace-agent-sdkgen.mjs must resolve sdkgen through shared standard module');
}
if (!readIfExists(path.join(root, 'sdks', '_shared', 'sdkgen-standard.mjs')).includes(
  SDKWORK_SDKGEN_STANDARD.canonicalEntrypointPosix
)) {
  errors.push(
    `sdks/_shared/sdkgen-standard.mjs must default to ${SDKWORK_SDKGEN_STANDARD.canonicalEntrypointPosix}`
  );
}

for (const [label, content] of [
  ['specs/SDK_SPEC.md', sdkSpec],
  ['sdks/README.md', sdkWorkspaceReadme],
  ['sdks/workspace-agent-sdkgen.mjs', workspaceSdkgen],
  ['sdkwork-agent-business/specs/sdkgen/commands.md', sdkgenCommands],
  [
    'sdkwork-agent-business/specs/sdkgen/verification-latest.md',
    readIfExists(path.join(root, 'sdkwork-agent-business', 'specs', 'sdkgen', 'verification-latest.md'))
  ]
]) {
  if (content.includes(SDKWORK_SDKGEN_STANDARD.deprecatedEntrypointFragment)) {
    errors.push(
      `${label} must not use deprecated sdkgen path ${SDKWORK_SDKGEN_STANDARD.deprecatedEntrypointFragment}`
    );
  }
  if (content.includes('external generator')) {
    errors.push(`${label} must not describe sdkwork-sdk-generator as an external generator`);
  }
}

for (const [label, report] of [
  ['sdks/.sdkgen-agent-workspace-report.json', sdkgenReport],
  ['sdkwork-agent-business/specs/sdkgen/verification-latest.json', latestVerificationReport],
  ['sdkwork-agent-business/specs/sdkgen/verification-ci.json', ciVerificationReport]
]) {
  if (report) {
    validateSdkgenReport(label, report);
  }
}

for (const required of [
  'all SDKs',
  'sdkwork-sdk-generator',
  'sdkwork-code-generator',
  'Generated SDK output must not be hand-edited'
]) {
  if (!sdkSpec.includes(required)) {
    errors.push(`specs/SDK_SPEC.md must define SDK generator rule: ${required}`);
  }
}

const agentBusinessApi = readIfExists(path.join(root, 'sdkwork-agent-business', 'src', 'api.rs'));
for (const required of [
  'AGENT_OPEN_API_PREFIX',
  '"/agent/v3/api"',
  'AGENT_OPEN_API_OPERATIONS'
]) {
  if (!agentBusinessApi.includes(required)) {
    errors.push(`sdkwork-agent-business/src/api.rs must include ${required}`);
  }
}

const agentBusinessHttp = readIfExists(path.join(root, 'sdkwork-agent-business', 'src', 'http.rs'));
for (const required of ['build_open_router', '"/agent/v3/api/ai/agents"']) {
  if (!agentBusinessHttp.includes(required)) {
    errors.push(`sdkwork-agent-business/src/http.rs must include ${required}`);
  }
}

for (const family of families) {
  const familyRoot = path.join(sdksRoot, family.familyDir);
  const authorityPath = path.join(
    familyRoot,
    'openapi',
    `${family.authority}.openapi.yaml`
  );
  const sdkgenPath = path.join(
    familyRoot,
    'openapi',
    `${family.authority}.sdkgen.yaml`
  );
  const packageRoot = path.join(familyRoot, `${family.familyDir}-typescript`);

  ensureFile(path.join('sdks', family.familyDir, 'README.md'));
  ensureFile(path.join('sdks', family.familyDir, '.sdkwork-assembly.json'));
  ensureFile(path.join('sdks', family.familyDir, 'sdk-manifest.json'));
  ensureFile(path.join('sdks', family.familyDir, 'specs', 'README.md'));
  ensureFile(path.join('sdks', family.familyDir, 'specs', 'component.spec.json'));
  ensureFile(path.join('sdks', family.familyDir, 'bin', 'verify-sdk.mjs'));
  ensureFile(
    path.join('sdks', family.familyDir, `${family.familyDir}-typescript`, 'README.md')
  );
  ensureFile(
    path.join('sdks', family.familyDir, `${family.familyDir}-typescript`, 'package.json')
  );
  ensureDirectory(
    path.join('sdks', family.familyDir, `${family.familyDir}-typescript`, 'generated', 'server-openapi')
  );
  ensureFile(path.relative(root, authorityPath));
  ensureFile(path.relative(root, sdkgenPath));
  const generatedApiPath = path.join(
    packageRoot,
    SDKWORK_SDKGEN_STANDARD.generatedOutput,
    'src',
    'api',
    'ai.ts'
  );
  ensureFile(path.relative(root, generatedApiPath));

  const readme = readIfExists(path.join(familyRoot, 'README.md'));
  for (const required of [
    family.familyDir,
    family.authority,
    family.packageName,
    family.apiPrefix,
    `--standard-profile ${SDKWORK_SDKGEN_STANDARD.standardProfile}`
  ]) {
    if (!readme.includes(required)) {
      errors.push(`${family.familyDir}/README.md must mention ${required}`);
    }
  }

  const assembly = readJsonIfExists(path.join(familyRoot, '.sdkwork-assembly.json'));
  const sdkManifest = readJsonIfExists(path.join(familyRoot, 'sdk-manifest.json'));
  const packageJson = readJsonIfExists(path.join(packageRoot, 'package.json'));
  const generatedPackageJson = readJsonIfExists(
    path.join(packageRoot, SDKWORK_SDKGEN_STANDARD.generatedOutput, 'package.json')
  );
  const generatedMetadata = readJsonIfExists(
    path.join(packageRoot, SDKWORK_SDKGEN_STANDARD.generatedOutput, 'sdkwork-sdk.json')
  );
  if (assembly) {
    if (assembly.sdkOwner !== AGENT_SDK_OWNER) {
      errors.push(`${family.familyDir} assembly sdkOwner must be ${AGENT_SDK_OWNER}`);
    }
    if (assembly.apiAuthority !== family.authority) {
      errors.push(`${family.familyDir} assembly apiAuthority must be ${family.authority}`);
    }
    if (assembly.generationInputSpec !== `openapi/${family.authority}.sdkgen.yaml`) {
      errors.push(`${family.familyDir} assembly generationInputSpec must be openapi/${family.authority}.sdkgen.yaml`);
    }
    assertDependencyList(
      `${family.familyDir} assembly sdkDependencies`,
      assembly.sdkDependencies ?? [],
      family.sdkDependencies ?? []
    );
  }
  if (sdkManifest) {
    if (sdkManifest.sdkName !== family.sdkName) {
      errors.push(`${family.familyDir} sdk-manifest sdkName must be ${family.sdkName}`);
    }
    if (sdkManifest.packageName !== family.packageName) {
      errors.push(`${family.familyDir} sdk-manifest packageName must be ${family.packageName}`);
    }
    if (sdkManifest.sdkOwner !== AGENT_SDK_OWNER) {
      errors.push(`${family.familyDir} sdk-manifest sdkOwner must be ${AGENT_SDK_OWNER}`);
    }
    if (sdkManifest.apiAuthority !== family.authority) {
      errors.push(`${family.familyDir} sdk-manifest apiAuthority must be ${family.authority}`);
    }
    if (sdkManifest.sdkFamily !== family.familyDir) {
      errors.push(`${family.familyDir} sdk-manifest sdkFamily must be ${family.familyDir}`);
    }
    if (sdkManifest.sdkType !== family.sdkType) {
      errors.push(`${family.familyDir} sdk-manifest sdkType must be ${family.sdkType}`);
    }
    if (sdkManifest.sdkSurface !== family.sdkSurface) {
      errors.push(`${family.familyDir} sdk-manifest sdkSurface must be ${family.sdkSurface}`);
    }
    if (sdkManifest.apiPrefix !== family.apiPrefix) {
      errors.push(`${family.familyDir} sdk-manifest apiPrefix must be ${family.apiPrefix}`);
    }
    if (sdkManifest.generationInputSpec !== `openapi/${family.authority}.sdkgen.yaml`) {
      errors.push(`${family.familyDir} sdk-manifest generationInputSpec must be openapi/${family.authority}.sdkgen.yaml`);
    }
    const expectedGeneratedOutput = `${family.languagePackageDir}/${SDKWORK_SDKGEN_STANDARD.generatedOutput}`;
    if (sdkManifest.generatedOutput !== expectedGeneratedOutput) {
      errors.push(`${family.familyDir} sdk-manifest generatedOutput must be ${expectedGeneratedOutput}`);
    }
    if (sdkManifest.standardProfile !== SDKWORK_SDKGEN_STANDARD.standardProfile) {
      errors.push(
        `${family.familyDir} sdk-manifest standardProfile must be ${SDKWORK_SDKGEN_STANDARD.standardProfile}`
      );
    }
    assertDependencyList(
      `${family.familyDir} sdk-manifest sdkDependencies`,
      sdkManifest.sdkDependencies ?? [],
      family.sdkDependencies ?? []
    );
  }
  if (packageJson) {
    if (packageJson.name !== family.packageName) {
      errors.push(`${family.familyDir} TypeScript package name must be ${family.packageName}`);
    }
    if (packageJson.sdkwork?.sdkName !== family.sdkName) {
      errors.push(`${family.familyDir} package sdkwork.sdkName must be ${family.sdkName}`);
    }
    if (packageJson.sdkwork?.sdkOwner !== AGENT_SDK_OWNER) {
      errors.push(`${family.familyDir} package sdkwork.sdkOwner must be ${AGENT_SDK_OWNER}`);
    }
    if (packageJson.sdkwork?.authority !== family.authority) {
      errors.push(`${family.familyDir} package sdkwork.authority must be ${family.authority}`);
    }
    if (packageJson.sdkwork?.sdkSurface !== family.sdkSurface) {
      errors.push(`${family.familyDir} package sdkwork.sdkSurface must be ${family.sdkSurface}`);
    }
    if (packageJson.sdkwork?.apiPrefix !== family.apiPrefix) {
      errors.push(`${family.familyDir} package sdkwork.apiPrefix must be ${family.apiPrefix}`);
    }
    if (packageJson.sdkwork?.sdkType !== family.sdkType) {
      errors.push(`${family.familyDir} package sdkwork.sdkType must be ${family.sdkType}`);
    }
    if (packageJson.sdkwork?.generatedOutput !== SDKWORK_SDKGEN_STANDARD.generatedOutput) {
      errors.push(
        `${family.familyDir} package sdkwork.generatedOutput must be ${SDKWORK_SDKGEN_STANDARD.generatedOutput}`
      );
    }
    if (packageJson.sdkwork?.standardProfile !== SDKWORK_SDKGEN_STANDARD.standardProfile) {
      errors.push(
        `${family.familyDir} package sdkwork.standardProfile must be ${SDKWORK_SDKGEN_STANDARD.standardProfile}`
      );
    }
    assertDependencyList(
      `${family.familyDir} package sdkwork.sdkDependencies`,
      packageJson.sdkwork?.sdkDependencies ?? [],
      family.sdkDependencies ?? []
    );
  }
  if (generatedPackageJson && Object.hasOwn(generatedPackageJson, 'sdkwork')) {
    errors.push(`${family.familyDir} generated package.json must not carry sdkwork ownership metadata`);
  }

  if (generatedMetadata) {
    if (generatedMetadata.name !== family.sdkName) {
      errors.push(`${family.familyDir} generated metadata name must be ${family.sdkName}`);
    }
    if (generatedMetadata.sdkType !== family.sdkType) {
      errors.push(`${family.familyDir} generated metadata sdkType must be ${family.sdkType}`);
    }
    if (generatedMetadata.packageName !== family.packageName) {
      errors.push(`${family.familyDir} generated metadata packageName must be ${family.packageName}`);
    }
    assertNoGeneratedOwnershipStandardKeys(`${family.familyDir} generated metadata`, generatedMetadata);
    if (family.key === 'open') {
      if (generatedMetadata.sdkSurface !== family.sdkSurface) {
        errors.push(`${family.familyDir} generated metadata sdkSurface must be ${family.sdkSurface}`);
      }
      if (generatedMetadata.authority !== family.authority) {
        errors.push(`${family.familyDir} generated metadata authority must be ${family.authority}`);
      }
      if (generatedMetadata.apiPrefix !== family.apiPrefix) {
        errors.push(`${family.familyDir} generated metadata apiPrefix must be ${family.apiPrefix}`);
      }
      if (generatedMetadata.derivedFrom?.reason !== family.externalSdkgenProfileGap) {
        errors.push(
          `${family.familyDir} generated metadata derivedFrom.reason must be ${family.externalSdkgenProfileGap}`
        );
      }
    }
  }

  const spec = readJsonIfExists(path.join(familyRoot, 'specs', 'component.spec.json'));
  if (spec) {
    if (spec.component?.name !== family.familyDir) {
      errors.push(`${family.familyDir} component name must be ${family.familyDir}`);
    }
    if (spec.sdk?.sdkOwner !== AGENT_SDK_OWNER) {
      errors.push(`${family.familyDir} component sdk.sdkOwner must be ${AGENT_SDK_OWNER}`);
    }
    if (spec.sdk?.authority !== family.authority) {
      errors.push(`${family.familyDir} component sdk.authority must be ${family.authority}`);
    }
    if (spec.sdk?.packageName !== family.packageName) {
      errors.push(`${family.familyDir} component sdk.packageName must be ${family.packageName}`);
    }
    if (spec.sdk?.apiPrefix !== family.apiPrefix) {
      errors.push(`${family.familyDir} component sdk.apiPrefix must be ${family.apiPrefix}`);
    }
    if (spec.sdk?.sdkType !== family.sdkType) {
      errors.push(`${family.familyDir} component sdk.sdkType must be ${family.sdkType}`);
    }
    if (spec.sdk?.sdkSurface !== family.sdkSurface) {
      errors.push(`${family.familyDir} component sdk.sdkSurface must be ${family.sdkSurface}`);
    }
    const expectedGeneratedOutput = `${family.languagePackageDir}/${SDKWORK_SDKGEN_STANDARD.generatedOutput}`;
    if (spec.sdk?.generatedOutput !== expectedGeneratedOutput) {
      errors.push(`${family.familyDir} component sdk.generatedOutput must be ${expectedGeneratedOutput}`);
    }
    if (spec.sdk?.standardProfile !== SDKWORK_SDKGEN_STANDARD.standardProfile) {
      errors.push(
        `${family.familyDir} component sdk.standardProfile must be ${SDKWORK_SDKGEN_STANDARD.standardProfile}`
      );
    }
    assertDependencyList(
      `${family.familyDir} component contracts.sdkDependencies`,
      spec.contracts?.sdkDependencies ?? [],
      family.sdkDependencies ?? []
    );
  }

  const authority = readIfExists(authorityPath);
  const sdkgen = readIfExists(sdkgenPath);
  validateOpenApi(`${family.familyDir} authority`, authority, family);
  validateOpenApi(`${family.familyDir} sdkgen`, sdkgen, family);
  if (sdkgen.includes("$ref: '#/components/responses/Problem'")) {
    errors.push(`${family.familyDir} sdkgen input must inline explicit problem responses`);
  }

  validateGeneratedAgentApi(
    `${family.familyDir} generated TypeScript API`,
    readIfExists(generatedApiPath),
    family
  );
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join('\n'));
  process.exit(1);
}

console.log('Agent SDK workspace check passed.');

function validateOpenApi(label, content, family) {
  if (!content) {
    return;
  }
  for (const required of [
    'openapi: 3.1.2',
    `title: SDKWork Agent ${titleKind(family.key)} API`,
    `x-sdkwork-owner: ${AGENT_SDK_OWNER}`,
    `x-sdkwork-api-authority: ${family.authority}`,
    family.apiPrefix,
    'operationId: agents.list',
    'operationId: agents.create',
    'operationId: agents.providerBindings.create',
    'operationId: agents.deployments.create',
    'operationId: knowledgeBases.list',
    'operationId: knowledgeBases.create',
    'operationId: knowledgeBases.retrieve',
    'operationId: knowledgeBases.update',
    'operationId: knowledgeBases.delete',
    'operationId: knowledgeBases.restore',
    'operationId: knowledgeSources.list',
    'operationId: knowledgeSources.create',
    'operationId: knowledgeSources.retrieve',
    'operationId: knowledgeSources.update',
    'operationId: knowledgeSources.delete',
    'operationId: knowledgeSources.restore',
    'operationId: knowledgeList.list',
    'operationId: knowledgeDocuments.create',
    'operationId: knowledgeDocuments.update',
    'operationId: knowledgeDocuments.delete',
    'operationId: knowledgeDocuments.restore',
    'operationId: knowledgeRead.read',
    'operationId: knowledgeSearch.search',
    'operationId: knowledgeChunks.list',
    'operationId: knowledgeChunks.create',
    'operationId: knowledgeChunks.retrieve',
    'operationId: knowledgeIndexes.list',
    'operationId: knowledgeIndexes.upsert',
    'operationId: knowledgeIndexes.retrieve',
    'operationId: knowledgeBindings.list',
    'operationId: knowledgeBindings.create',
    'operationId: knowledgeBindings.retrieve',
    'operationId: knowledgeSyncJobs.list',
    'operationId: knowledgeSyncJobs.create',
    'operationId: knowledgeSyncJobs.retrieve',
    'operationId: knowledgeSyncJobs.start',
    'operationId: knowledgeSyncJobs.complete',
    'operationId: knowledgeSyncJobs.fail',
    'operationId: knowledgeSyncJobs.cancel',
    'operationId: memoryStores.create',
    'operationId: memoryStores.retrieve',
    'operationId: memoryStores.update',
    'operationId: memoryProfiles.create',
    'operationId: memoryProfiles.retrieve',
    'operationId: memoryBindings.create',
    'operationId: memoryBindings.retrieve',
    'operationId: memoryNamespaces.create',
    'operationId: memoryNamespaces.retrieve',
    'operationId: memoryRecords.list',
    'operationId: memoryRecords.create',
    'operationId: memoryRecords.retrieve',
    'operationId: memoryRecords.delete',
    'operationId: memoryRecords.restore',
    'operationId: memorySources.list',
    'operationId: memorySources.create',
    'operationId: memoryRelations.list',
    'operationId: memoryRelations.create',
    'operationId: memoryRetrievalIndexes.list',
    'operationId: memoryRetrievalIndexes.upsert',
    'UpdateKnowledgeBaseRequest:',
    'UpdateKnowledgeSourceRequest:',
    'UpdateKnowledgeDocumentRequest:',
    'StartKnowledgeSyncJobRequest:',
    'CompleteKnowledgeSyncJobRequest:',
    'FailKnowledgeSyncJobRequest:',
    'CancelKnowledgeSyncJobRequest:',
    'CreateMemoryStoreRequest:',
    'UpdateMemoryStoreRequest:',
    'CreateMemoryProfileRequest:',
    'CreateMemoryBindingRequest:',
    'CreateMemoryNamespaceRequest:',
    'CreateMemoryRecordRequest:',
    'CreateMemorySourceRequest:',
    'CreateMemoryRelationRequest:',
    'UpsertMemoryRetrievalIndexRequest:',
    'MemoryStoreKind:',
    'MemoryIndexKind:',
    'MemoryBindingScopeKind:',
    'MemoryNamespaceKind:',
    'MemoryRecordKind:',
    'MemorySourceKind:',
    'MemoryRelationKind:',
    'MemoryStoreRecord:',
    'MemoryProfileRecord:',
    'MemoryBindingRecord:',
    'MemoryNamespaceRecord:',
    'MemoryRecord:',
    'MemorySourceRecord:',
    'MemoryRelationRecord:',
    'MemoryRetrievalIndexRecord:',
    'KnowledgeSourceIdPath:',
    'KnowledgeChunkIdPath:',
    'KnowledgeIndexIdPath:',
    'KnowledgeBindingIdPath:',
    'KnowledgeSyncJobIdPath:',
    'MemoryStoreIdPath:',
    'MemoryProfileIdPath:',
    'MemoryBindingIdPath:',
    'MemoryNamespaceIdPath:',
    'MemoryIdPath:',
    'x-sdkwork-resource: knowledgeBases',
    'x-sdkwork-resource: knowledgeSources',
    'x-sdkwork-resource: knowledgeDocuments',
    'x-sdkwork-resource: knowledgeList',
    'x-sdkwork-resource: knowledgeRead',
    'x-sdkwork-resource: knowledgeSearch',
    'x-sdkwork-resource: knowledgeChunks',
    'x-sdkwork-resource: knowledgeIndexes',
    'x-sdkwork-resource: knowledgeBindings',
    'x-sdkwork-resource: knowledgeSyncJobs',
    'x-sdkwork-resource: memoryStores',
    'x-sdkwork-resource: memoryProfiles',
    'x-sdkwork-resource: memoryBindings',
    'x-sdkwork-resource: memoryNamespaces',
    'x-sdkwork-resource: memoryRecords',
    'x-sdkwork-resource: memorySources',
    'x-sdkwork-resource: memoryRelations',
    'x-sdkwork-resource: memoryRetrievalIndexes',
    'x-sdkwork-permission: agent.business.knowledge.base.list',
    'x-sdkwork-permission: agent.business.knowledge.base.create',
    'x-sdkwork-permission: agent.business.knowledge.base.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.base.update',
    'x-sdkwork-permission: agent.business.knowledge.base.delete',
    'x-sdkwork-permission: agent.business.knowledge.base.restore',
    'x-sdkwork-permission: agent.business.knowledge.source.list',
    'x-sdkwork-permission: agent.business.knowledge.source.create',
    'x-sdkwork-permission: agent.business.knowledge.source.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.source.update',
    'x-sdkwork-permission: agent.business.knowledge.source.delete',
    'x-sdkwork-permission: agent.business.knowledge.source.restore',
    'x-sdkwork-permission: agent.business.knowledge.list',
    'x-sdkwork-permission: agent.business.knowledge.document.create',
    'x-sdkwork-permission: agent.business.knowledge.document.update',
    'x-sdkwork-permission: agent.business.knowledge.document.delete',
    'x-sdkwork-permission: agent.business.knowledge.document.restore',
    'x-sdkwork-permission: agent.business.knowledge.read',
    'x-sdkwork-permission: agent.business.knowledge.search',
    'x-sdkwork-permission: agent.business.knowledge.chunk.list',
    'x-sdkwork-permission: agent.business.knowledge.chunk.create',
    'x-sdkwork-permission: agent.business.knowledge.chunk.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.index.list',
    'x-sdkwork-permission: agent.business.knowledge.index.upsert',
    'x-sdkwork-permission: agent.business.knowledge.index.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.binding.list',
    'x-sdkwork-permission: agent.business.knowledge.binding.create',
    'x-sdkwork-permission: agent.business.knowledge.binding.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.list',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.create',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.start',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.complete',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.fail',
    'x-sdkwork-permission: agent.business.knowledge.sync_job.cancel',
    'x-sdkwork-permission: agent.business.memory.store.create',
    'x-sdkwork-permission: agent.business.memory.store.retrieve',
    'x-sdkwork-permission: agent.business.memory.store.update',
    'x-sdkwork-permission: agent.business.memory.profile.create',
    'x-sdkwork-permission: agent.business.memory.profile.retrieve',
    'x-sdkwork-permission: agent.business.memory.binding.create',
    'x-sdkwork-permission: agent.business.memory.binding.retrieve',
    'x-sdkwork-permission: agent.business.memory.namespace.create',
    'x-sdkwork-permission: agent.business.memory.namespace.retrieve',
    'x-sdkwork-permission: agent.business.memory.record.list',
    'x-sdkwork-permission: agent.business.memory.record.create',
    'x-sdkwork-permission: agent.business.memory.record.retrieve',
    'x-sdkwork-permission: agent.business.memory.record.delete',
    'x-sdkwork-permission: agent.business.memory.record.restore',
    'x-sdkwork-permission: agent.business.memory.source.list',
    'x-sdkwork-permission: agent.business.memory.source.create',
    'x-sdkwork-permission: agent.business.memory.relation.list',
    'x-sdkwork-permission: agent.business.memory.relation.create',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.list',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.upsert',
    'components:',
    'application/problem+json',
    'Access-Token'
  ]) {
    if (!content.includes(required)) {
      errors.push(`${label} must include ${required}`);
    }
  }
  for (const forbidden of forbiddenAgentApiPrefixesFor(family)) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not include ${forbidden}`);
    }
  }
  if (content.includes('X-Request-Id')) {
    errors.push(`${label} must not expose X-Request-Id`);
  }
  validateOpenApiOwnership(label, content, family);
  for (const forbidden of [
    'operationId: knowledgeDocuments.list',
    'operationId: knowledgeDocuments.retrieve',
    'operationId: knowledgeChunks.update',
    'operationId: knowledgeChunks.delete',
    'operationId: knowledgeChunks.restore',
    'operationId: knowledgeIndexes.update',
    'operationId: knowledgeIndexes.delete',
    'operationId: knowledgeIndexes.restore',
    'operationId: knowledgeBindings.update',
    'operationId: knowledgeBindings.delete',
    'operationId: knowledgeBindings.restore',
    'x-sdkwork-permission: agent.business.knowledge.document.list',
    'x-sdkwork-permission: agent.business.knowledge.document.retrieve',
    'x-sdkwork-permission: agent.business.knowledge.chunk.update',
    'x-sdkwork-permission: agent.business.knowledge.chunk.delete',
    'x-sdkwork-permission: agent.business.knowledge.chunk.restore',
    'x-sdkwork-permission: agent.business.knowledge.index.update',
    'x-sdkwork-permission: agent.business.knowledge.index.delete',
    'x-sdkwork-permission: agent.business.knowledge.index.restore',
    'x-sdkwork-permission: agent.business.knowledge.binding.update',
    'x-sdkwork-permission: agent.business.knowledge.binding.delete',
    'x-sdkwork-permission: agent.business.knowledge.binding.restore',
    'operationId: memoryStores.delete',
    'operationId: memoryStores.restore',
    'operationId: memoryProfiles.update',
    'operationId: memoryProfiles.delete',
    'operationId: memoryProfiles.restore',
    'operationId: memoryBindings.update',
    'operationId: memoryBindings.delete',
    'operationId: memoryBindings.restore',
    'operationId: memoryNamespaces.update',
    'operationId: memoryNamespaces.delete',
    'operationId: memoryNamespaces.restore',
    'operationId: memoryRecords.update',
    'operationId: memorySources.update',
    'operationId: memorySources.delete',
    'operationId: memorySources.restore',
    'operationId: memoryRelations.update',
    'operationId: memoryRelations.delete',
    'operationId: memoryRelations.restore',
    'operationId: memoryRetrievalIndexes.update',
    'operationId: memoryRetrievalIndexes.delete',
    'operationId: memoryRetrievalIndexes.restore',
    'x-sdkwork-permission: agent.business.memory.store.delete',
    'x-sdkwork-permission: agent.business.memory.store.restore',
    'x-sdkwork-permission: agent.business.memory.profile.update',
    'x-sdkwork-permission: agent.business.memory.profile.delete',
    'x-sdkwork-permission: agent.business.memory.profile.restore',
    'x-sdkwork-permission: agent.business.memory.binding.update',
    'x-sdkwork-permission: agent.business.memory.binding.delete',
    'x-sdkwork-permission: agent.business.memory.binding.restore',
    'x-sdkwork-permission: agent.business.memory.namespace.update',
    'x-sdkwork-permission: agent.business.memory.namespace.delete',
    'x-sdkwork-permission: agent.business.memory.namespace.restore',
    'x-sdkwork-permission: agent.business.memory.record.update',
    'x-sdkwork-permission: agent.business.memory.source.update',
    'x-sdkwork-permission: agent.business.memory.source.delete',
    'x-sdkwork-permission: agent.business.memory.source.restore',
    'x-sdkwork-permission: agent.business.memory.relation.update',
    'x-sdkwork-permission: agent.business.memory.relation.delete',
    'x-sdkwork-permission: agent.business.memory.relation.restore',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.update',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.delete',
    'x-sdkwork-permission: agent.business.memory.retrieval_index.restore'
  ]) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not expose unsupported agent RAG lifecycle contract ${forbidden}`);
    }
  }
}

function validateOpenApiOwnership(label, content, family) {
  const lines = content.split(/\r?\n/);
  let currentPath = '';
  let current = null;

  function finishCurrent() {
    if (!current) {
      return;
    }
    const block = current.lines.join('\n');
    if (!block.includes(`      x-sdkwork-owner: ${AGENT_SDK_OWNER}`)) {
      errors.push(`${label} ${current.method.toUpperCase()} ${current.pathKey} must declare x-sdkwork-owner ${AGENT_SDK_OWNER}`);
    }
    if (!block.includes(`      x-sdkwork-api-authority: ${family.authority}`)) {
      errors.push(`${label} ${current.method.toUpperCase()} ${current.pathKey} must declare x-sdkwork-api-authority ${family.authority}`);
    }
    current = null;
  }

  for (const line of lines) {
    const pathMatch = /^  (\/[^:]+):\s*$/.exec(line);
    if (pathMatch) {
      finishCurrent();
      currentPath = pathMatch[1];
      continue;
    }

    const methodMatch = /^    (get|put|post|patch|delete|head|options|trace):\s*$/.exec(line);
    if (methodMatch) {
      finishCurrent();
      current = { pathKey: currentPath, method: methodMatch[1], lines: [line] };
      continue;
    }

    if (current) {
      current.lines.push(line);
    }
  }
  finishCurrent();
}

function validateGeneratedAgentApi(label, content, family) {
  if (!content) {
    return;
  }
  const scopeFreeCallSurface = usesScopeFreeCallSurface(family);
  for (const required of [
    'export class AiMemoryStoresApi',
    'export class AiMemoryProfilesApi',
    'export class AiMemoryBindingsApi',
    'export class AiMemoryNamespacesApi',
    'export class AiMemoryRecordsApi',
    'export class AiMemorySourcesApi',
    'export class AiMemoryRelationsApi',
    'export class AiMemoryRetrievalIndexesApi',
    'async create(body: CreateMemoryStoreRequest',
    'async retrieve(memoryStoreId: string',
    'async update(memoryStoreId: string, body: UpdateMemoryStoreRequest',
    'async create(memoryStoreId: string, body: CreateMemoryProfileRequest',
    'async retrieve(memoryProfileId: string',
    'async create(memoryProfileId: string, body: CreateMemoryBindingRequest',
    'async retrieve(memoryBindingId: string',
    'async create(body: CreateMemoryNamespaceRequest',
    'async retrieve(memoryNamespaceId: string',
    'async list(memoryNamespaceId: string',
    'async create(memoryNamespaceId: string, body: CreateMemoryRecordRequest',
    'async retrieve(memoryId: string',
    'async delete(memoryId: string',
    'async restore(memoryId: string, body: RestoreAgentRequest',
    scopeFreeCallSurface
      ? 'async list(memoryId: string, params?: AiMemorySourcesListParams'
      : 'async list(memoryId: string, params: AiMemorySourcesListParams',
    'async create(memoryId: string, body: CreateMemorySourceRequest',
    scopeFreeCallSurface
      ? 'async list(memoryId: string, params?: AiMemoryRelationsListParams'
      : 'async list(memoryId: string, params: AiMemoryRelationsListParams',
    'async create(memoryId: string, body: CreateMemoryRelationRequest',
    scopeFreeCallSurface
      ? 'async list(memoryId: string, params?: AiMemoryRetrievalIndexesListParams'
      : 'async list(memoryId: string, params: AiMemoryRetrievalIndexesListParams',
    'async upsert(body: UpsertMemoryRetrievalIndexRequest',
    'CreateMemoryStoreRequest',
    'UpdateMemoryStoreRequest',
    'CreateMemoryProfileRequest',
    'CreateMemoryBindingRequest',
    'CreateMemoryNamespaceRequest',
    'CreateMemoryRecordRequest',
    'CreateMemorySourceRequest',
    'CreateMemoryRelationRequest',
    'UpsertMemoryRetrievalIndexRequest',
    'MemoryStoreResponse',
    'MemoryProfileResponse',
    'MemoryBindingResponse',
    'MemoryNamespaceResponse',
    'MemoryRecordResponse',
    'MemoryRecordListResponse',
    'MemorySourceResponse',
    'MemorySourceListResponse',
    'MemoryRelationResponse',
    'MemoryRelationListResponse',
    'MemoryRetrievalIndexResponse',
    'MemoryRetrievalIndexListResponse',
    'public readonly memoryStores: AiMemoryStoresApi',
    'public readonly memoryProfiles: AiMemoryProfilesApi',
    'public readonly memoryBindings: AiMemoryBindingsApi',
    'public readonly memoryNamespaces: AiMemoryNamespacesApi',
    'public readonly memoryRecords: AiMemoryRecordsApi',
    'public readonly memorySources: AiMemorySourcesApi',
    'public readonly memoryRelations: AiMemoryRelationsApi',
    'public readonly memoryRetrievalIndexes: AiMemoryRetrievalIndexesApi',
    'export class AiKnowledgeBasesApi',
    'export class AiKnowledgeSourcesApi',
    'export class AiKnowledgeListApi',
    'export class AiKnowledgeDocumentsApi',
    'export class AiKnowledgeReadApi',
    'export class AiKnowledgeSearchApi',
    'export class AiKnowledgeChunksApi',
    'export class AiKnowledgeIndexesApi',
    'export class AiKnowledgeBindingsApi',
    'export class AiKnowledgeSyncJobsApi',
    scopeFreeCallSurface
      ? 'async list(params?: AiKnowledgeBasesListParams): Promise<KnowledgeBaseListResponse>'
      : 'async list(params: AiKnowledgeBasesListParams): Promise<KnowledgeBaseListResponse>',
    scopeFreeCallSurface
      ? 'async create(body: CreateKnowledgeBaseRequest): Promise<KnowledgeBaseResponse>'
      : 'async create(body: CreateKnowledgeBaseRequest, params: AiKnowledgeBasesCreateParams): Promise<KnowledgeBaseResponse>',
    'async retrieve(knowledgeBaseId: string',
    'async update(knowledgeBaseId: string, body: UpdateKnowledgeBaseRequest',
    'async delete(knowledgeBaseId: string',
    'async restore(knowledgeBaseId: string, body: RestoreAgentRequest',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeSourcesListParams): Promise<KnowledgeSourceListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeSourcesListParams): Promise<KnowledgeSourceListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeSourceRequest): Promise<KnowledgeSourceResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeSourceRequest, params: AiKnowledgeSourcesCreateParams): Promise<KnowledgeSourceResponse>',
    'async retrieve(knowledgeSourceId: string',
    'async update(knowledgeSourceId: string, body: UpdateKnowledgeSourceRequest',
    'async delete(knowledgeSourceId: string',
    'async restore(knowledgeSourceId: string, body: RestoreAgentRequest',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeDocumentRequest): Promise<KnowledgeDocumentResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeDocumentRequest, params: AiKnowledgeDocumentsCreateParams): Promise<KnowledgeDocumentResponse>',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeListListParams): Promise<KnowledgeDocumentListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeListListParams): Promise<KnowledgeDocumentListResponse>',
    scopeFreeCallSurface
      ? 'async read(knowledgeDocumentId: string): Promise<KnowledgeDocumentResponse>'
      : 'async read(knowledgeDocumentId: string, params: AiKnowledgeReadReadParams): Promise<KnowledgeDocumentResponse>',
    scopeFreeCallSurface
      ? 'async search(knowledgeBaseId: string, body: SearchKnowledgeRequest): Promise<KnowledgeSearchResponse>'
      : 'async search(knowledgeBaseId: string, body: SearchKnowledgeRequest, params: AiKnowledgeSearchSearchParams): Promise<KnowledgeSearchResponse>',
    'async update(knowledgeDocumentId: string, body: UpdateKnowledgeDocumentRequest',
    'async delete(knowledgeDocumentId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeDocumentId: string, params?: AiKnowledgeChunksListParams): Promise<KnowledgeChunkListResponse>'
      : 'async list(knowledgeDocumentId: string, params: AiKnowledgeChunksListParams): Promise<KnowledgeChunkListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeDocumentId: string, body: CreateKnowledgeChunkRequest): Promise<KnowledgeChunkResponse>'
      : 'async create(knowledgeDocumentId: string, body: CreateKnowledgeChunkRequest, params: AiKnowledgeChunksCreateParams): Promise<KnowledgeChunkResponse>',
    'async retrieve(knowledgeChunkId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeDocumentId: string, params?: AiKnowledgeIndexesListParams): Promise<KnowledgeIndexListResponse>'
      : 'async list(knowledgeDocumentId: string, params: AiKnowledgeIndexesListParams): Promise<KnowledgeIndexListResponse>',
    scopeFreeCallSurface
      ? 'async upsert(body: UpsertKnowledgeIndexRequest): Promise<KnowledgeIndexResponse>'
      : 'async upsert(body: UpsertKnowledgeIndexRequest, params: AiKnowledgeIndexesUpsertParams): Promise<KnowledgeIndexResponse>',
    'async retrieve(knowledgeIndexId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeBindingsListParams): Promise<KnowledgeBindingListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeBindingsListParams): Promise<KnowledgeBindingListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeBindingRequest): Promise<KnowledgeBindingResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeBindingRequest, params: AiKnowledgeBindingsCreateParams): Promise<KnowledgeBindingResponse>',
    'async retrieve(knowledgeBindingId: string',
    scopeFreeCallSurface
      ? 'async list(knowledgeBaseId: string, params?: AiKnowledgeSyncJobsListParams): Promise<KnowledgeSyncJobListResponse>'
      : 'async list(knowledgeBaseId: string, params: AiKnowledgeSyncJobsListParams): Promise<KnowledgeSyncJobListResponse>',
    scopeFreeCallSurface
      ? 'async create(knowledgeBaseId: string, body: CreateKnowledgeSyncJobRequest): Promise<KnowledgeSyncJobResponse>'
      : 'async create(knowledgeBaseId: string, body: CreateKnowledgeSyncJobRequest, params: AiKnowledgeSyncJobsCreateParams): Promise<KnowledgeSyncJobResponse>',
    'async retrieve(syncJobId: string',
    'async start(syncJobId: string, body: StartKnowledgeSyncJobRequest',
    'async complete(syncJobId: string, body: CompleteKnowledgeSyncJobRequest',
    'async fail(syncJobId: string, body: FailKnowledgeSyncJobRequest',
    'async cancel(syncJobId: string, body: CancelKnowledgeSyncJobRequest',
    'CreateKnowledgeBaseRequest',
    'CreateKnowledgeSourceRequest',
    'CreateKnowledgeDocumentRequest',
    'CreateKnowledgeChunkRequest',
    'SearchKnowledgeRequest',
    'UpsertKnowledgeIndexRequest',
    'CreateKnowledgeBindingRequest',
    'CreateKnowledgeSyncJobRequest',
    'UpdateKnowledgeBaseRequest',
    'UpdateKnowledgeSourceRequest',
    'UpdateKnowledgeDocumentRequest',
    'StartKnowledgeSyncJobRequest',
    'CompleteKnowledgeSyncJobRequest',
    'FailKnowledgeSyncJobRequest',
    'CancelKnowledgeSyncJobRequest',
    'KnowledgeBaseListResponse',
    'KnowledgeSourceListResponse',
    'KnowledgeDocumentListResponse',
    'KnowledgeChunkListResponse',
    'KnowledgeIndexListResponse',
    'KnowledgeBindingListResponse',
    'KnowledgeSyncJobListResponse',
    'public readonly knowledgeBases: AiKnowledgeBasesApi',
    'public readonly knowledgeSources: AiKnowledgeSourcesApi',
    'public readonly knowledgeList: AiKnowledgeListApi',
    'public readonly knowledgeDocuments: AiKnowledgeDocumentsApi',
    'public readonly knowledgeRead: AiKnowledgeReadApi',
    'public readonly knowledgeSearch: AiKnowledgeSearchApi',
    'public readonly knowledgeChunks: AiKnowledgeChunksApi',
    'public readonly knowledgeIndexes: AiKnowledgeIndexesApi',
    'public readonly knowledgeBindings: AiKnowledgeBindingsApi',
    'public readonly knowledgeSyncJobs: AiKnowledgeSyncJobsApi'
  ]) {
    if (!content.includes(required)) {
      errors.push(`${label} must include generated SDK surface ${required}`);
    }
  }

  const documentDeleteParams = boundedBlock(
    content,
    'export interface AiKnowledgeDocumentsDeleteParams',
    '}'
  );
  for (const required of [
    ...(scopeFreeCallSurface ? [] : ['tenantId: Int64String;']),
    'expectedVersion?: Int64String;',
    'requestedAt: string;'
  ]) {
    if (!documentDeleteParams.includes(required)) {
      errors.push(`${label} AiKnowledgeDocumentsDeleteParams must include ${required}`);
    }
  }

  const documentDeleteMethod = boundedBlock(
    content,
    'async delete(knowledgeDocumentId: string',
    'async restore(knowledgeDocumentId: string'
  );
  for (const required of [
    ...(scopeFreeCallSurface ? [] : ["{ name: 'tenant_id', value: params.tenantId"]),
    "{ name: 'expected_version', value: params.expectedVersion",
    "{ name: 'requested_at', value: params.requestedAt",
    'return this.client.delete<KnowledgeDocumentResponse>'
  ]) {
    if (!documentDeleteMethod.includes(required)) {
      errors.push(`${label} knowledge document delete method must include ${required}`);
    }
  }
  if (documentDeleteMethod.includes(', body')) {
    errors.push(`${label} knowledge document delete method must not send a request body`);
  }
  if (scopeFreeCallSurface) {
    for (const forbidden of [
      'tenantId:',
      'organizationId:',
      'ownerUserId:',
      "'tenant_id'",
      "'organization_id'",
      "'owner_user_id'",
      'AiKnowledgeBasesCreateParams',
      'AiKnowledgeDocumentsCreateParams',
      'AiKnowledgeReadReadParams',
      'AiKnowledgeSearchSearchParams'
    ]) {
      if (content.includes(forbidden)) {
        errors.push(`${label} call surface must not include caller-provided scope ${forbidden}`);
      }
    }
  }

  for (const forbidden of [
    'async update(knowledgeChunkId: string',
    'async delete(knowledgeChunkId: string',
    'async restore(knowledgeChunkId: string',
    'async update(knowledgeIndexId: string',
    'async delete(knowledgeIndexId: string',
    'async restore(knowledgeIndexId: string',
    'async update(knowledgeBindingId: string',
    'async delete(knowledgeBindingId: string',
    'async restore(knowledgeBindingId: string',
    'async delete(memoryStoreId: string',
    'async restore(memoryStoreId: string',
    'async update(memoryProfileId: string',
    'async delete(memoryProfileId: string',
    'async restore(memoryProfileId: string',
    'async update(memoryBindingId: string',
    'async delete(memoryBindingId: string',
    'async restore(memoryBindingId: string',
    'async update(memoryNamespaceId: string',
    'async delete(memoryNamespaceId: string',
    'async restore(memoryNamespaceId: string',
    'async update(memoryId: string',
    'async update(memorySourceId: string',
    'async delete(memorySourceId: string',
    'async restore(memorySourceId: string',
    'async update(memoryRelationId: string',
    'async delete(memoryRelationId: string',
    'async restore(memoryRelationId: string',
    'async update(memoryIndexId: string',
    'async delete(memoryIndexId: string',
    'async restore(memoryIndexId: string'
  ]) {
    if (content.includes(forbidden)) {
      errors.push(`${label} must not expose ${forbidden}`);
    }
  }
}

function boundedBlock(content, startMarker, endMarker) {
  const start = content.indexOf(startMarker);
  if (start < 0) {
    return '';
  }
  const afterStart = content.slice(start);
  const end = afterStart.indexOf(endMarker);
  if (end < 0) {
    return afterStart;
  }
  return afterStart.slice(0, end + endMarker.length);
}

function titleKind(key) {
  switch (key) {
    case 'open':
      return 'Open';
    case 'app':
      return 'App';
    case 'backend':
      return 'Backend';
    default:
      throw new Error(`unknown family key: ${key}`);
  }
}

function assertDependencyList(label, actual, expected) {
  const compact = (dependencies) =>
    dependencies.map((dependency) => ({
      workspace: dependency.workspace,
      apiAuthority: dependency.apiAuthority,
      dependencyMode: dependency.dependencyMode,
      generatedTransportImportPolicy: dependency.generatedTransportImportPolicy
    }));
  const actualText = JSON.stringify(compact(actual));
  const expectedText = JSON.stringify(compact(expected));
  if (actualText !== expectedText) {
    errors.push(`${label} must be ${expectedText}, received ${actualText}`);
  }
}

function assertNoGeneratedOwnershipStandardKeys(label, metadata) {
  for (const key of [
    'sdkOwner',
    'apiAuthority',
    'sdkFamily',
    'generationInputSpec',
    'sdkDependencies',
    'ownerOnlyOperationCount',
    'standardProfile',
    'standardVersion'
  ]) {
    if (Object.hasOwn(metadata, key)) {
      errors.push(`${label} must not carry ownership standard key ${key}`);
    }
  }
}

function validateSdkgenReport(label, report) {
  const expectedEntrypoint = resolveSdkgenEntrypoint();
  if (report.sdkgenPath !== expectedEntrypoint) {
    errors.push(`${label} sdkgenPath must be ${expectedEntrypoint}`);
  }
  if (containsLocalDriveAbsolutePath(JSON.stringify(report))) {
    errors.push(`${label} must use repository-relative report paths instead of local drive absolute paths`);
  }
  if (!String(report.sdkgenPath ?? '').includes('sdkwork-sdk-generator')) {
    errors.push(`${label} sdkgenPath must point to sdkwork-sdk-generator`);
  }
  if (report.standardProfile !== SDKWORK_SDKGEN_STANDARD.standardProfile) {
    errors.push(`${label} standardProfile must be ${SDKWORK_SDKGEN_STANDARD.standardProfile}`);
  }
  if (!Array.isArray(report.families)) {
    errors.push(`${label} must include families array`);
    return;
  }
  for (const family of families) {
    const familyReport = report.families.find((candidate) => candidate.key === family.key);
    if (!familyReport) {
      errors.push(`${label} must include family report ${family.key}`);
      continue;
    }
    if (familyReport.authority !== family.authority) {
      errors.push(`${label} ${family.key} authority must be ${family.authority}`);
    }
    if (familyReport.apiPrefix !== family.apiPrefix) {
      errors.push(`${label} ${family.key} apiPrefix must be ${family.apiPrefix}`);
    }
    if (familyReport.packageName !== family.packageName) {
      errors.push(`${label} ${family.key} packageName must be ${family.packageName}`);
    }
    if (familyReport.skipReason && familyReport.skipReason !== family.externalSdkgenProfileGap) {
      errors.push(`${label} ${family.key} skipReason must be ${family.externalSdkgenProfileGap}`);
    }
    if (!familyReport.skipped && familyReport.hasChanges !== false) {
      errors.push(`${label} ${family.key} standardized SDK report must have hasChanges=false after standard generation`);
    }
    if (familyReport.key === 'open' && familyReport.derivedHasChanges !== false) {
      errors.push(`${label} ${family.key} derivedHasChanges must be false after standard generation`);
    }
  }
  if (report.openSdkDerivation?.hasChanges !== false) {
    errors.push(`${label} open SDK derivation must have hasChanges=false after standard generation`);
  }
}

function usesScopeFreeCallSurface(family) {
  return family?.key === 'app' || family?.key === 'open';
}

function containsLocalDriveAbsolutePath(value) {
  return /(^|[^A-Za-z0-9])[A-Za-z]:[\\/]/.test(value);
}

function ensureFile(relativePath) {
  const filePath = path.join(root, relativePath);
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    errors.push(`missing file: ${relativePath}`);
  }
}

function ensureDirectory(relativePath) {
  const directoryPath = path.join(root, relativePath);
  if (!fs.existsSync(directoryPath) || !fs.statSync(directoryPath).isDirectory()) {
    errors.push(`missing directory: ${relativePath}`);
  }
}

function readIfExists(filePath) {
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    return '';
  }
  return fs.readFileSync(filePath, 'utf8');
}

function readJsonIfExists(filePath) {
  const content = readIfExists(filePath);
  if (!content) {
    return null;
  }
  try {
    return JSON.parse(content);
  } catch (error) {
    errors.push(`invalid json: ${path.relative(root, filePath)}: ${error.message}`);
    return null;
  }
}
