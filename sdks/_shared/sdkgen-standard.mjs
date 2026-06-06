export const SDKWORK_SDKGEN_STANDARD = Object.freeze({
  standardProfile: 'sdkwork-v3',
  canonicalRootWin: String.raw`D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator`,
  canonicalEntrypointWin: String.raw`D:\javasource\spring-ai-plus\sdk\sdkwork-sdk-generator\bin\sdkgen.js`,
  canonicalEntrypointPosix:
    'D:/javasource/spring-ai-plus/sdk/sdkwork-sdk-generator/bin/sdkgen.js',
  envOverride: 'SDKWORK_SDKGEN_PATH',
  deprecatedEntrypointFragment:
    'spring-ai-plus-business/sdk/sdkwork-sdk-generator',
  generatedOutput: 'generated/server-openapi'
});

export function resolveSdkgenEntrypoint(env = process.env) {
  return env[SDKWORK_SDKGEN_STANDARD.envOverride] ??
    SDKWORK_SDKGEN_STANDARD.canonicalEntrypointPosix;
}

