export const AGENT_SDK_OWNER = 'sdkwork-kernel';
export const AGENT_SDK_OWNERSHIP_STANDARD_VERSION = '2026-06-06';

const APPBASE_APP_DEPENDENCY = {
  workspace: 'sdkwork-appbase-app-sdk',
  role: 'appbase-identity-and-session-capability',
  required: true,
  dependencyMode: 'consumer-sdk',
  apiPrefix: '/app/v3/api',
  apiAuthority: 'sdkwork-appbase-app-api',
  generatedTransportImportPolicy: 'forbidden',
  packageByLanguage: {
    typescript: '@sdkwork/appbase-app-sdk'
  }
};

const APPBASE_BACKEND_DEPENDENCY = {
  workspace: 'sdkwork-appbase-backend-sdk',
  role: 'appbase-backend-management-capability',
  required: true,
  dependencyMode: 'consumer-sdk',
  apiPrefix: '/backend/v3/api',
  apiAuthority: 'sdkwork-appbase-backend-api',
  generatedTransportImportPolicy: 'forbidden',
  packageByLanguage: {
    typescript: '@sdkwork/appbase-backend-sdk'
  }
};

export const AGENT_SDK_FAMILIES = [
  {
    key: 'open',
    familyDir: 'sdkwork-agent-sdk',
    authority: 'sdkwork-agent-open-api',
    title: 'SDKWork Agent Open API',
    description: 'Developer-facing agent Open API for SDKWork agent integrations.',
    sourceOpenApi: 'sdkwork-agent-business/specs/openapi/agent-business-app-openapi-3.1.2.yaml',
    moduleOpenApi: 'sdkwork-agent-business/specs/openapi/agent-business-open-openapi-3.1.2.yaml',
    sourcePrefix: '/app/v3/api',
    apiPrefix: '/agent/v3/api',
    sdkName: 'sdkwork-agent-sdk',
    sdkType: 'custom',
    sdkSurface: 'open',
    externalSdkgenProfileSupported: false,
    externalSdkgenProfileGap: 'sdkwork-v3 standard profile currently supports app, backend, and im prefixes only.',
    packageName: '@sdkwork/agent-sdk',
    npmPackageName: '@sdkwork/agent-sdk',
    languagePackageDir: 'sdkwork-agent-sdk-typescript',
    audience: 'developer and integration authors',
    capability: 'agent-open-sdk',
    sdkOwner: AGENT_SDK_OWNER,
    sdkDependencies: []
  },
  {
    key: 'app',
    familyDir: 'sdkwork-agent-app-sdk',
    authority: 'sdkwork-agent-app-api',
    title: 'SDKWork Agent App API',
    description: 'App-facing managed agent APIs for SDKWork user-facing clients.',
    sourceOpenApi: 'sdkwork-agent-business/specs/openapi/agent-business-app-openapi-3.1.2.yaml',
    sourcePrefix: '/app/v3/api',
    apiPrefix: '/app/v3/api',
    sdkName: 'sdkwork-agent-app-sdk',
    sdkType: 'app',
    sdkSurface: 'app',
    externalSdkgenProfileSupported: true,
    packageName: '@sdkwork/agent-app-sdk',
    npmPackageName: '@sdkwork/agent-app-sdk',
    languagePackageDir: 'sdkwork-agent-app-sdk-typescript',
    audience: 'app, desktop, mobile, H5, and user-facing clients',
    capability: 'agent-app-sdk',
    sdkOwner: AGENT_SDK_OWNER,
    sdkDependencies: [APPBASE_APP_DEPENDENCY]
  },
  {
    key: 'backend',
    familyDir: 'sdkwork-agent-backend-sdk',
    authority: 'sdkwork-agent-backend-api',
    title: 'SDKWork Agent Backend API',
    description: 'Backend-facing managed agent APIs for SDKWork operator and control-plane clients.',
    sourceOpenApi: 'sdkwork-agent-business/specs/openapi/agent-business-backend-openapi-3.1.2.yaml',
    sourcePrefix: '/backend/v3/api',
    apiPrefix: '/backend/v3/api',
    sdkName: 'sdkwork-agent-backend-sdk',
    sdkType: 'backend',
    sdkSurface: 'backend',
    externalSdkgenProfileSupported: true,
    packageName: '@sdkwork/agent-backend-sdk',
    npmPackageName: '@sdkwork/agent-backend-sdk',
    languagePackageDir: 'sdkwork-agent-backend-sdk-typescript',
    audience: 'backend console, operators, automation, and control-plane integrations',
    capability: 'agent-backend-sdk',
    sdkOwner: AGENT_SDK_OWNER,
    sdkDependencies: [APPBASE_BACKEND_DEPENDENCY]
  }
];

export function resolveAgentSdkFamily(keyOrFamilyDir) {
  const family = AGENT_SDK_FAMILIES.find(
    (candidate) =>
      candidate.key === keyOrFamilyDir || candidate.familyDir === keyOrFamilyDir
  );
  if (!family) {
    throw new Error(`Unknown agent SDK family: ${keyOrFamilyDir}`);
  }
  return family;
}

export function forbiddenAgentApiPrefixesFor(family) {
  return AGENT_SDK_FAMILIES
    .map((candidate) => candidate.apiPrefix)
    .filter((prefix) => prefix !== family.apiPrefix);
}
