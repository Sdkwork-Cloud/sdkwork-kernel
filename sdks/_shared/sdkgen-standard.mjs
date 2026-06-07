export const SDKWORK_SDKGEN_STANDARD = Object.freeze({
  standardProfile: 'sdkwork-v3',
  canonicalRootWin: String.raw`..\sdkwork-sdk-generator`,
  canonicalEntrypointWin: String.raw`..\sdkwork-sdk-generator\bin\sdkgen.js`,
  canonicalEntrypointPosix:
    '../sdkwork-sdk-generator/bin/sdkgen.js',
  envOverride: 'SDKWORK_SDKGEN_PATH',
  deprecatedEntrypointFragment:
    ['java', 'source'].join(''),
  generatedOutput: 'generated/server-openapi'
});

export function resolveSdkgenEntrypoint(env = process.env) {
  return env[SDKWORK_SDKGEN_STANDARD.envOverride] ??
    SDKWORK_SDKGEN_STANDARD.canonicalEntrypointPosix;
}

