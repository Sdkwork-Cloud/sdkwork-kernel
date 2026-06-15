import fs from 'node:fs';
import path from 'node:path';
import {
  AGENT_SDK_OWNER,
  AGENT_SDK_OWNERSHIP_STANDARD_VERSION
} from './agent-sdk-families.mjs';
import { SDKWORK_SDKGEN_STANDARD } from './sdkgen-standard.mjs';

const HTTP_METHOD_PATTERN = /^    (get|put|post|patch|delete|head|options|trace):\s*$/;
const ROOT_METADATA_KEYS = [
  'x-sdkwork-owner',
  'x-sdkwork-api-authority',
  'x-sdkwork-sdk-family',
  'x-sdkwork-owner-only-input',
  'x-sdkwork-standard-version'
];

const SDK_FAMILY_CANONICAL_SPECS = [
  {
    file: 'SDK_SPEC.md',
    path: '../../../sdkwork-specs/SDK_SPEC.md',
    purpose: 'SDK family naming, generation, dependency, and consumer integration rules.'
  },
  {
    file: 'SDK_WORKSPACE_GENERATION_SPEC.md',
    path: '../../../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md',
    purpose: 'SDK workspace layout, authority OpenAPI, derived input, and generated output rules.'
  },
  {
    file: 'API_SPEC.md',
    path: '../../../sdkwork-specs/API_SPEC.md',
    purpose: 'OpenAPI authority, API surface, schema, and SDK-generation contract rules.'
  },
  {
    file: 'TEST_SPEC.md',
    path: '../../../sdkwork-specs/TEST_SPEC.md',
    purpose: 'SDK generation, contract, and workspace verification rules.'
  },
  {
    file: 'DOCUMENTATION_SPEC.md',
    path: '../../../sdkwork-specs/DOCUMENTATION_SPEC.md',
    purpose: 'SDK README, examples, changelog, and generated artifact documentation rules.'
  }
];

export function annotateAgentOpenApiOwnership(openapi, family) {
  const withoutRootMetadata = openapi
    .split(/\r?\n/)
    .filter((line) => !ROOT_METADATA_KEYS.some((key) => line.startsWith(`${key}:`)))
    .join('\n');

  const rootMetadata = [
    `x-sdkwork-owner: ${AGENT_SDK_OWNER}`,
    `x-sdkwork-api-authority: ${family.authority}`,
    `x-sdkwork-sdk-family: ${family.familyDir}`,
    'x-sdkwork-owner-only-input: true',
    `x-sdkwork-standard-version: '${AGENT_SDK_OWNERSHIP_STANDARD_VERSION}'`
  ];

  const rootLines = withoutRootMetadata.split(/\r?\n/);
  const insertAt = rootLines.findIndex((line) => line === 'servers:');
  if (insertAt < 0) {
    throw new Error(`${family.authority} OpenAPI must contain a root servers section`);
  }
  rootLines.splice(insertAt, 0, ...rootMetadata);

  const withoutOperationMetadata = rootLines
    .filter(
      (line) =>
        !line.startsWith('      x-sdkwork-owner:') &&
        !line.startsWith('      x-sdkwork-api-authority:')
    )
    .join('\n');

  const outputLines = [];
  for (const line of withoutOperationMetadata.split(/\r?\n/)) {
    outputLines.push(line);
    if (HTTP_METHOD_PATTERN.test(line)) {
      outputLines.push(`      x-sdkwork-owner: ${AGENT_SDK_OWNER}`);
      outputLines.push(`      x-sdkwork-api-authority: ${family.authority}`);
    }
  }

  return ensureTrailingNewline(outputLines.join('\n'));
}

export function countAgentOpenApiOperations(openapi) {
  return openapi
    .split(/\r?\n/)
    .filter((line) => HTTP_METHOD_PATTERN.test(line))
    .length;
}

export function buildAgentSdkAssembly(family, operationCount) {
  const input = `openapi/${family.authority}.sdkgen.yaml`;
  const authoritySpec = `openapi/${family.authority}.openapi.yaml`;
  return {
    workspace: family.familyDir,
    title: family.title,
    apiVersion: '0.1.0',
    openapiVersion: '3.1.2',
    authoritySpec,
    generationInputSpec: input,
    derivedSpecs: {
      default: input
    },
    apiAuthority: family.authority,
    discoverySurface: {
      sdkTarget: family.sdkSurface,
      apiPrefix: family.apiPrefix,
      schemaUrl: schemaUrlFor(family),
      generatedProtocols: ['http-openapi'],
      manualTransports: []
    },
    languages: [
      {
        language: 'typescript',
        workspace: family.languagePackageDir,
        generationState: family.key === 'open' ? 'derived' : 'materialized',
        releaseState: 'not_published',
        generatedPath: `${family.languagePackageDir}/${SDKWORK_SDKGEN_STANDARD.generatedOutput}`,
        manifestPath: `${family.languagePackageDir}/${SDKWORK_SDKGEN_STANDARD.generatedOutput}/package.json`,
        name: family.packageName,
        version: '0.1.0',
        description: `Generator-owned TypeScript transport SDK for ${family.title}.`,
        consumerSurface: {
          primaryClient: primaryClientFor(family),
          apiPrefix: family.apiPrefix
        }
      }
    ],
    sdkOwner: AGENT_SDK_OWNER,
    sdkDependencies: cloneDependencies(family),
    metadata: {
      managedBy: 'sdks/_shared/agent-sdk-ownership.mjs',
      standardVersion: AGENT_SDK_OWNERSHIP_STANDARD_VERSION,
      ownerOnlyOperationCount: operationCount
    }
  };
}

export function buildAgentComponentSpec(family) {
  return {
    schemaVersion: 1,
    kind: 'sdkwork.component.spec',
    component: {
      name: family.familyDir,
      displayName: `${family.title.replace(/ API$/u, '')} SDK`,
      version: '0.1.0',
      type: 'sdk-family',
      root: `sdks/${family.familyDir}`,
      domain: 'agent',
      capability: family.capability,
      surface: componentSurfaceFor(family),
      status: 'standardized',
      languages: ['typescript'],
      generated: true,
      private: false,
      manifests: ['.sdkwork-assembly.json']
    },
    canonicalSpecs: SDK_FAMILY_CANONICAL_SPECS,
    sdk: {
      family: family.familyDir,
      authority: family.authority,
      sdkOwner: AGENT_SDK_OWNER,
      apiPrefix: family.apiPrefix,
      packageName: family.packageName,
      sdkName: family.sdkName,
      sdkType: family.sdkType,
      sdkSurface: family.sdkSurface,
      externalSdkgenProfileSupported: family.externalSdkgenProfileSupported,
      externalSdkgenProfileGap: family.externalSdkgenProfileGap,
      generatedOutput: `${family.languagePackageDir}/${SDKWORK_SDKGEN_STANDARD.generatedOutput}`,
      standardProfile: SDKWORK_SDKGEN_STANDARD.standardProfile
    },
    contracts: {
      apiAuthority: {
        name: family.authority,
        owner: AGENT_SDK_OWNER,
        prefix: family.apiPrefix,
        authorityOpenApi: `openapi/${family.authority}.openapi.yaml`,
        derivedOpenApi: [`openapi/${family.authority}.sdkgen.yaml`],
        standard: '../../specs/SDK_SPEC.md'
      },
      publicExports: [],
      runtimeEntrypoints: ['.sdkwork-assembly.json'],
      routeManifest: null,
      sdkDependencies: cloneDependencies(family),
      dependencyApiExports: [],
      dependencyApiSurfaces: [],
      sdkClients: [primaryClientFor(family)],
      events: [],
      configKeys: ['.sdkwork-assembly.json']
    },
    verification: {
      commands: [
        'node sdks/materialize-agent-v3-openapi-boundaries.mjs',
        `node sdks/${family.familyDir}/bin/verify-sdk.mjs`,
        'node sdks/test/verify-agent-sdk-ownership-boundaries.test.mjs',
        'node scripts/check-agent-sdk-workspace.mjs'
      ]
    },
    metadata: {
      managedBy: 'sdks/_shared/agent-sdk-ownership.mjs',
      standardVersion: AGENT_SDK_OWNERSHIP_STANDARD_VERSION
    }
  };
}

export function decoratePackageMetadata(packageJson, family) {
  return {
    ...packageJson,
    sdkwork: {
      ...(packageJson.sdkwork ?? {}),
      sdkName: family.sdkName,
      authority: family.authority,
      sdkOwner: AGENT_SDK_OWNER,
      apiPrefix: family.apiPrefix,
      sdkType: family.sdkType,
      sdkSurface: family.sdkSurface,
      generatedOutput: SDKWORK_SDKGEN_STANDARD.generatedOutput,
      standardProfile: SDKWORK_SDKGEN_STANDARD.standardProfile,
      sdkDependencies: cloneDependencies(family)
    }
  };
}

export function buildAgentSdkManifest(family, operationCount) {
  return {
    schemaVersion: 1,
    sdkName: family.sdkName,
    packageName: family.packageName,
    sdkOwner: AGENT_SDK_OWNER,
    apiAuthority: family.authority,
    sdkFamily: family.familyDir,
    sdkType: family.sdkType,
    sdkSurface: family.sdkSurface,
    language: 'typescript',
    apiPrefix: family.apiPrefix,
    generationInputSpec: `openapi/${family.authority}.sdkgen.yaml`,
    generatedOutput: `${family.languagePackageDir}/${SDKWORK_SDKGEN_STANDARD.generatedOutput}`,
    standardProfile: SDKWORK_SDKGEN_STANDARD.standardProfile,
    sdkDependencies: cloneDependencies(family),
    ownerOnlyOperationCount: operationCount,
    standardVersion: AGENT_SDK_OWNERSHIP_STANDARD_VERSION,
    managedBy: 'sdks/_shared/agent-sdk-ownership.mjs'
  };
}

export function syncAgentSdkOwnershipFamily(root, family) {
  const familyRoot = path.join(root, 'sdks', family.familyDir);
  const sdkgenPath = path.join(familyRoot, 'openapi', `${family.authority}.sdkgen.yaml`);
  const sdkgen = fs.existsSync(sdkgenPath) ? fs.readFileSync(sdkgenPath, 'utf8') : '';
  const operationCount = sdkgen ? countAgentOpenApiOperations(sdkgen) : 0;

  writeJsonIfChanged(
    path.join(familyRoot, '.sdkwork-assembly.json'),
    buildAgentSdkAssembly(family, operationCount)
  );
  writeJsonIfChanged(
    path.join(familyRoot, 'specs', 'component.spec.json'),
    buildAgentComponentSpec(family)
  );
  writeJsonIfChanged(
    path.join(familyRoot, 'sdk-manifest.json'),
    buildAgentSdkManifest(family, operationCount)
  );

  const packageRoot = path.join(familyRoot, family.languagePackageDir);
  const packageJsonPath = path.join(packageRoot, 'package.json');
  updateJsonIfExists(packageJsonPath, (packageJson) => decoratePackageMetadata(packageJson, family));
}

export function syncAgentSdkOwnershipWorkspace(root, families) {
  for (const family of families) {
    syncAgentSdkOwnershipFamily(root, family);
  }
}

export function cloneDependencies(family) {
  return structuredClone(Array.isArray(family.sdkDependencies) ? family.sdkDependencies : []);
}

function updateJsonIfExists(filePath, updater) {
  if (!fs.existsSync(filePath)) {
    return;
  }
  const updated = updater(JSON.parse(fs.readFileSync(filePath, 'utf8')));
  writeJsonIfChanged(filePath, updated);
}

function writeJsonIfChanged(filePath, value) {
  const content = `${JSON.stringify(value, null, 2)}\n`;
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  if (fs.existsSync(filePath) && fs.readFileSync(filePath, 'utf8') === content) {
    return;
  }
  fs.writeFileSync(filePath, content, 'utf8');
}

function schemaUrlFor(family) {
  switch (family.key) {
    case 'open':
      return '/agent/v3/openapi.json';
    case 'app':
      return '/app/v3/openapi.json';
    case 'backend':
      return '/backend/v3/openapi.json';
    default:
      return `${family.apiPrefix}/openapi.json`;
  }
}

function primaryClientFor(family) {
  switch (family.key) {
    case 'open':
      return 'SdkworkAgentClient';
    case 'app':
      return 'SdkworkAppClient';
    case 'backend':
      return 'SdkworkBackendClient';
    default:
      return 'SdkworkClient';
  }
}

function componentSurfaceFor(family) {
  switch (family.sdkSurface) {
    case 'open':
    case 'custom':
      return 'open-api';
    case 'app':
      return 'app-api';
    case 'backend':
      return 'backend-admin';
    default:
      return 'open-api';
  }
}

function ensureTrailingNewline(content) {
  return content.endsWith('\n') ? content : `${content}\n`;
}
