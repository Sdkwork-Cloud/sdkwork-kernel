import path from 'node:path';
import {
  SDKWORK_SDKGEN_STANDARD,
  resolveSdkgenEntrypoint
} from '../../../sdks/_shared/sdkgen-standard.mjs';

export function validateSdkgenStandard({ root, errors, ensureFile, readIfExists, readJsonIfExists, families }) {
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
      validateSdkgenReport({ label, report, errors, families });
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
}

function validateSdkgenReport({ label, report, errors, families }) {
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

function containsLocalDriveAbsolutePath(value) {
  return /(^|[^A-Za-z0-9])[A-Za-z]:[\\/]/.test(value);
}
