import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const sdksRoot = path.resolve(testDir, "..");
const workspaceRoot = path.resolve(sdksRoot, "..");
const owner = "sdkwork-kernel";

const families = [
  {
    root: "sdkwork-agent-internal-sdk",
    authority: "sdkwork-agent-internal-api",
    input: "openapi/sdkwork-agent-internal-api.sdkgen.yaml",
    manifest: "sdk-manifest.json",
    generatedPackage:
      "sdkwork-agent-internal-sdk-typescript/generated/server-openapi/package.json",
    generatedMetadata:
      "sdkwork-agent-internal-sdk-typescript/generated/server-openapi/sdkwork-sdk.json",
    dependencies: [],
  },
];

const dependencyOwnedPathPrefixes = [
  "/app/v3/api/auth/",
  "/app/v3/api/iam/",
  "/app/v3/api/open_platform/",
  "/app/v3/api/system/iam/",
  "/backend/v3/api/auth/",
  "/backend/v3/api/iam/",
  "/backend/v3/api/open_platform/",
  "/backend/v3/api/system/iam/",
];

function readJson(relativePath) {
  return JSON.parse(readFileSync(path.join(workspaceRoot, relativePath), "utf8"));
}

function readText(relativePath) {
  return readFileSync(path.join(workspaceRoot, relativePath), "utf8");
}

function operationBlocks(openapiText) {
  const blocks = [];
  const lines = openapiText.split(/\r?\n/u);
  let currentPath = "";
  let current = null;
  for (const line of lines) {
    const pathMatch = /^  (\/[^:]+):\s*$/u.exec(line);
    if (pathMatch) {
      currentPath = pathMatch[1];
    }

    const methodMatch = /^    (get|put|post|patch|delete|head|options|trace):\s*$/u.exec(line);
    if (methodMatch) {
      if (current) {
        blocks.push(current);
      }
      current = { pathKey: currentPath, method: methodMatch[1], lines: [line] };
      continue;
    }

    if (current) {
      if (/^    [a-z][a-z]+:\s*$/u.test(line) || /^  (\/[^:]+):\s*$/u.test(line)) {
        blocks.push(current);
        current = null;
      }
      if (current) {
        current.lines.push(line);
      }
    }
  }
  if (current) {
    blocks.push(current);
  }
  return blocks;
}

test("agent SDK manifests are the family metadata source of truth", () => {
  for (const family of families) {
    const assemblyPath = path.join(workspaceRoot, "sdks", family.root, [".sdkwork", "assembly.json"].join("-"));
    assert.equal(
      existsSync(assemblyPath),
      false,
      `${family.root} must not retain removed parallel SDK metadata`,
    );

    const manifest = readJson(path.join("sdks", family.root, family.manifest));
    assert.equal(manifest.sdkOwner, owner, `${family.root} manifest must declare sdkOwner`);
    assert.equal(manifest.apiAuthority, family.authority, `${family.root} manifest must declare apiAuthority`);
    assert.equal(
      manifest.generationInputSpec,
      family.input,
      `${family.root} manifest must generate from owner-only sdkgen input`,
    );
    assert.deepEqual(
      manifest.sdkDependencies?.map((dependency) => ({
        workspace: dependency.workspace,
        apiAuthority: dependency.apiAuthority,
        dependencyMode: dependency.dependencyMode,
        generatedTransportImportPolicy: dependency.generatedTransportImportPolicy,
      })) ?? [],
      family.dependencies.map(([workspace, apiAuthority]) => ({
        workspace,
        apiAuthority,
        dependencyMode: "consumer-sdk",
        generatedTransportImportPolicy: "forbidden",
      })),
      `${family.root} must declare only dependency SDKs, not copied dependency APIs`,
    );
    assert.equal(
      manifest.languages?.[0]?.consumerPackageName,
      "@sdkwork/agent-internal-sdk",
      `${family.root} manifest must declare the composed consumer package name`,
    );
    assert.equal(
      manifest.languages?.[0]?.transportPackageName,
      "sdkwork-agent-internal-sdk-generated-typescript",
      `${family.root} manifest must declare the generated transport package name`,
    );
  }
});

test("agent component specs mirror owner and SDK dependency boundaries", () => {
  for (const family of families) {
    const componentSpec = readJson(path.join("sdks", family.root, "specs", "component.spec.json"));
    assert.equal(componentSpec.component?.name, family.root, `${family.root} component name must match`);
    assert.equal(componentSpec.sdk?.sdkOwner, owner, `${family.root} component sdkOwner must match`);
    assert.equal(componentSpec.sdk?.authority, family.authority, `${family.root} component authority must match`);
    assert.deepEqual(
      componentSpec.contracts?.sdkDependencies?.map((dependency) => ({
        workspace: dependency.workspace,
        apiAuthority: dependency.apiAuthority,
        dependencyMode: dependency.dependencyMode,
        generatedTransportImportPolicy: dependency.generatedTransportImportPolicy,
      })) ?? [],
      family.dependencies.map(([workspace, apiAuthority]) => ({
        workspace,
        apiAuthority,
        dependencyMode: "consumer-sdk",
        generatedTransportImportPolicy: "forbidden",
      })),
      `${family.root} component spec must mirror dependency SDKs`,
    );
  }
});

test("agent SDK manifests record owner and dependency boundaries outside generator-owned metadata", () => {
  for (const family of families) {
    const manifest = readJson(path.join("sdks", family.root, family.manifest));
    assert.equal(manifest.sdkOwner, owner, `${family.root} manifest must declare sdkOwner`);
    assert.equal(manifest.apiAuthority, family.authority, `${family.root} manifest must declare apiAuthority`);
    assert.equal(
      manifest.generationInputSpec,
      family.input,
      `${family.root} manifest must point at owner-only sdkgen input`,
    );
    assert.deepEqual(
      manifest.sdkDependencies?.map((dependency) => ({
        workspace: dependency.workspace,
        apiAuthority: dependency.apiAuthority,
        dependencyMode: dependency.dependencyMode,
        generatedTransportImportPolicy: dependency.generatedTransportImportPolicy,
      })) ?? [],
      family.dependencies.map(([workspace, apiAuthority]) => ({
        workspace,
        apiAuthority,
        dependencyMode: "consumer-sdk",
        generatedTransportImportPolicy: "forbidden",
      })),
      `${family.root} manifest must mirror dependency SDKs`,
    );

    const generatedMetadata = readJson(path.join("sdks", family.root, family.generatedMetadata));
    for (const forbiddenKey of [
      "sdkOwner",
      "apiAuthority",
      "sdkFamily",
      "generationInputSpec",
      "sdkDependencies",
      "ownerOnlyOperationCount",
      "standardProfile",
      "standardVersion",
    ]) {
      assert.equal(
        Object.hasOwn(generatedMetadata, forbiddenKey),
        false,
        `${family.root} generated metadata must not carry ownership standard key ${forbiddenKey}`,
      );
    }

    const generatedPackage = readJson(path.join("sdks", family.root, family.generatedPackage));
    assert.equal(
      Object.hasOwn(generatedPackage, "sdkwork"),
      false,
      `${family.root} generated package.json must not carry SDK ownership standard metadata`,
    );
  }
});

test("agent generated OpenAPI inputs contain only sdkwork-kernel owned operations", () => {
  for (const family of families) {
    const openapiText = readText(path.join("sdks", family.root, family.input));
    assert.match(openapiText, /^x-sdkwork-owner: sdkwork-kernel$/mu, `${family.root} root owner is required`);
    assert.match(
      openapiText,
      new RegExp(`^x-sdkwork-api-authority: ${family.authority}$`, "mu"),
      `${family.root} root api authority is required`,
    );

    const blocks = operationBlocks(openapiText);
    assert.ok(blocks.length > 0, `${family.root} must expose OpenAPI operations`);
    for (const block of blocks) {
      const text = block.lines.join("\n");
      assert.match(
        text,
        /^      x-sdkwork-owner: sdkwork-kernel$/mu,
        `${family.root} ${block.method.toUpperCase()} ${block.pathKey} must be kernel-owned`,
      );
      assert.match(
        text,
        new RegExp(`^      x-sdkwork-api-authority: ${family.authority}$`, "mu"),
        `${family.root} ${block.method.toUpperCase()} ${block.pathKey} must use ${family.authority}`,
      );
      assert(
        !dependencyOwnedPathPrefixes.some((prefix) => block.pathKey.startsWith(prefix)),
        `${family.root} must not copy dependency-owned route ${block.method.toUpperCase()} ${block.pathKey}`,
      );
    }
  }
});
