import path from 'node:path';
import { AGENT_SDK_OWNER } from '../../../sdks/_shared/agent-sdk-families.mjs';
import { SDKWORK_SDKGEN_STANDARD } from '../../../sdks/_shared/sdkgen-standard.mjs';

export function validateSdkFamilyMetadata({ family, familyRoot, packageRoot, errors, readIfExists, readJsonIfExists }) {
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
      family.sdkDependencies ?? [],
      errors
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
      family.sdkDependencies ?? [],
      errors
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
      family.sdkDependencies ?? [],
      errors
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
      const consumerPackageName = generatedMetadata.consumerPackageName;
      if (consumerPackageName !== family.packageName) {
        errors.push(`${family.familyDir} generated metadata packageName must be ${family.packageName}`);
      }
    }
    assertNoGeneratedOwnershipStandardKeys(`${family.familyDir} generated metadata`, generatedMetadata, errors);
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
      family.sdkDependencies ?? [],
      errors
    );
  }
}

function assertDependencyList(label, actual, expected, errors) {
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

function assertNoGeneratedOwnershipStandardKeys(label, metadata, errors) {
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
