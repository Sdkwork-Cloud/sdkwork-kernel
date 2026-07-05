export const AGENT_SDK_OWNER = 'sdkwork-kernel';
export const AGENT_SDK_OWNERSHIP_STANDARD_VERSION = '2026-06-26';

export const AGENT_SDK_FAMILIES = [
  {
    key: 'internal',
    familyDir: 'sdkwork-agent-internal-sdk',
    authority: 'sdkwork-agent-internal-api',
    title: 'SDKWork Agent Internal API',
    description:
      'Application-ingress internal runtime API for SDKWork agent runtime hosts and trusted in-app automation.',
    authorityOpenApi: 'apis/internal-api/intelligence/sdkwork-agent-internal-api.openapi.yaml',
    apiPrefix: '/internal/v3/api',
    sdkName: 'sdkwork-agent-internal-sdk',
    sdkType: 'custom',
    sdkSurface: 'internal',
    externalSdkgenProfileSupported: true,
    packageName: '@sdkwork/agent-internal-sdk',
    npmPackageName: '@sdkwork/agent-internal-sdk',
    languagePackageDir: 'sdkwork-agent-internal-sdk-typescript',
    audience: 'product shells, embedded consoles, and trusted in-app automation on application ingress',
    capability: 'agent-internal-sdk',
    sdkOwner: AGENT_SDK_OWNER,
    sdkDependencies: [],
  },
];

export function resolveAgentSdkFamily(keyOrFamilyDir) {
  const family = AGENT_SDK_FAMILIES.find(
    (candidate) => candidate.key === keyOrFamilyDir || candidate.familyDir === keyOrFamilyDir,
  );
  if (!family) {
    throw new Error(`Unknown agent SDK family: ${keyOrFamilyDir}`);
  }
  return family;
}

export function forbiddenAgentApiPrefixesFor(family) {
  return AGENT_SDK_FAMILIES.map((candidate) => candidate.apiPrefix).filter(
    (prefix) => prefix !== family.apiPrefix,
  );
}
