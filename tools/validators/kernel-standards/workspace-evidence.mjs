import fs from 'node:fs';
import path from 'node:path';

export const standardRootDirectories = [
  'apis',
  'apps',
  'crates',
  'sdks',
  'jobs',
  'tools',
  'plugins',
  'examples',
  'configs',
  'deployments',
  'scripts',
  'docs',
  'tests'
];

const requiredSdkworkSpecFiles = [
  'README.md',
  'SOUL.md',
  'AGENTS_SPEC.md',
  'SDKWORK_WORKSPACE_SPEC.md',
  'CODE_STYLE_SPEC.md',
  'NAMING_SPEC.md'
];

const requiredWorkspaceFiles = [
  'AGENTS.md',
  'CLAUDE.md',
  'GEMINI.md',
  'CODEX.md',
  '.sdkwork/README.md',
  '.sdkwork/.gitignore',
  '.sdkwork/skills/README.md',
  '.sdkwork/plugins/README.md'
];

const staleRootReadmeCommands = [
  'node kernel/scripts/check-kernel-standards.mjs',
  'node kernel/sdkwork-kernel-ui/scripts/check-kernel-ui-architecture.mjs',
  'cargo test --manifest-path kernel/sdkwork-agent-kernel/Cargo.toml',
  'cargo test --manifest-path kernel/sdkwork-agent-business/Cargo.toml',
  'cargo test --manifest-path kernel/sdkwork-code-kernel/Cargo.toml',
  'pnpm --dir kernel/sdkwork-kernel-ui'
];

const staleDictionaryPatterns = [
  ['sdkwork-birdcoder/kernel', /sdkwork-birdcoder\/kernel/],
  ['apps/sdkwork-birdcoder', /apps\/sdkwork-birdcoder/],
  ['apps/docs/ARCHITECT.md', /apps\/docs\/ARCHITECT\.md/],
  ['old root specs markdown links', /\]\((?:\.\.\/){3,}specs\//]
];

export const requiredStandardsEvidenceFiles = [
  {
    relativePath: path.join(
      'docs',
      'architecture',
      'decisions',
      'ADR-20260612-sdkwork-kernel-root-dictionary.md'
    ),
    requiredText: [
      'Status: accepted',
      'Owner:',
      'Date: 2026-06-12',
      'Specs:',
      '## Context',
      '## Decision',
      '## Alternatives',
      '## Consequences',
      '## Verification',
      'SDKWORK_WORKSPACE_SPEC.md',
      'COMPONENT_SPEC.md',
      'SDK_SPEC.md',
      'ARCHITECTURE_DECISION_SPEC.md',
      'QUALITY_GATE_SPEC.md',
      'sdkwork-agent-kernel/',
      'sdkwork-code-kernel/',
      'sdkwork-agent-business/',
      'sdkwork-kernel-ui/',
      'sdkwork-kernel-plugins/'
    ]
  },
  {
    relativePath: path.join(
      'docs',
      'architecture',
      'decisions',
      'ADR-20260612-agent-implementation-type.md'
    ),
    requiredText: [
      'Status: accepted',
      'Requirement: agent-implementation-type',
      'Date: 2026-06-12',
      'ARCHITECTURE_DECISION_SPEC.md',
      'DATABASE_SPEC.md',
      'API_SPEC.md',
      'SDK_SPEC.md',
      'implementationKind',
      'implementationType',
      'sdkwork-native',
      'openai-agents',
      'langgraph',
      'a_agent_business.implementation_type',
      'Generated SDK output was refreshed through generator commands',
      '## Verification'
    ]
  },
  {
    relativePath: path.join(
      'docs',
      'architecture',
      'decisions',
      'ADR-20260618-platform-framework-adoption.md'
    ),
    requiredText: [
      'Status: accepted',
      'Requirement: platform-framework-alignment',
      'Date: 2026-06-18',
      'WEB_FRAMEWORK_SPEC.md',
      'DATABASE_SPEC.md',
      'sdkwork-web-framework',
      'sdkwork-database',
      'sdkwork-discovery',
      'Phase 0',
      'Phase 1',
      'Phase 2',
      'Phase 3',
      'crates/sdkwork-routes-agent-open-api',
      'crates/sdkwork-routes-agent-app-api',
      'crates/sdkwork-routes-agent-backend-api',
      'crates/sdkwork-routes-agent-internal-api',
      '## Verification'
    ]
  },
  {
    relativePath: path.join(
      'docs',
      'architecture',
      'tech',
      'TECH-sdkwork-standards-alignment-20260612.md'
    ),
    requiredText: [
      'SDKWork Standards Alignment Quality Gate Evidence',
      'Date: 2026-06-12',
      'QUALITY_GATE_SPEC.md',
      'ARCHITECTURE_DECISION_SPEC.md',
      'SDKWORK_WORKSPACE_SPEC.md',
      'COMPONENT_SPEC.md',
      'SDK_SPEC.md',
      'SDK_WORKSPACE_GENERATION_SPEC.md',
      'TEST_SPEC.md',
      'Definition Of Ready',
      'Definition Of Done',
      'node --test tests\\*.test.mjs',
      'node scripts\\check-kernel-standards.mjs',
      'node sdkwork-kernel-ui\\scripts\\check-kernel-ui-architecture.mjs',
      'node scripts\\check-agent-sdk-workspace.mjs',
      '7 tests pass',
      '8 tests pass',
      '9 tests pass',
      '10 tests pass',
      '11 tests pass',
      '12 tests pass',
      '13 tests pass',
      '14 tests pass',
      '15 tests pass',
      '16 tests pass',
      '17 tests pass',
      '18 tests pass',
      'Kernel standards conformance check passed.',
      'Kernel UI architecture check passed for 10 packages.',
      'Agent SDK workspace check passed.',
      'No generated output was hand-edited',
      'SDK metadata scan',
      'SDK surface scan',
      'tools/validators/kernel-standards/check-kernel-standards.mjs',
      'tools/validators/kernel-standards/component-specs.mjs',
      'tools/validators/kernel-standards/workspace-evidence.mjs',
      'tools/validators/kernel-standards/agent-knowledge-memory-contracts.mjs',
      'tools/validators/kernel-standards/kernel-contracts.mjs',
      'tools/validators/kernel-standards/ui-packages.mjs',
      'tools/validators/kernel-standards/platform-integration.mjs',
      'tools/validators/agent-sdk-workspace/check-agent-sdk-workspace.mjs',
      'tools/validators/agent-sdk-workspace/sdkgen-standard-checks.mjs',
      'tools/validators/agent-sdk-workspace/sdk-family-metadata-checks.mjs',
      'tools/validators/agent-sdk-workspace/openapi-checks.mjs',
      'tools/validators/agent-sdk-workspace/generated-typescript-api-surface-checks.mjs',
      'Generated TypeScript API surface validator split',
      'Agent Implementation Type Follow-up',
      'ADR-20260612-agent-implementation-type.md',
      '2026-06-12-agent-implementation-type.md',
      'cargo test --features http-axum --manifest-path sdkwork-agent-business/Cargo.toml',
      'cargo test --features postgres-sync --manifest-path sdkwork-agent-business/Cargo.toml',
      'node sdks/workspace-agent-sdkgen.mjs --mode dry-run',
      'hasChanges: false',
      'Agent implementation type evidence is enforced by tools/validators/kernel-standards/workspace-evidence.mjs',
      '19 tests pass',
      '20 tests pass',
      'Component Surface Applicability Follow-up',
      'surfaceNotRequiredReason',
      '19 non-SDK component specs document why component.surface is not required',
      'Component Dependency Policy Follow-up',
      'All component specs explicitly declare contracts.sdkDependencies and contracts.dependencyApiExports',
      'Component Dependency API Surface Follow-up',
      'All component specs explicitly declare contracts.dependencyApiSurfaces',
      'Component SDK Route Manifest Applicability Follow-up',
      'SDK family root component specs explicitly declare contracts.routeManifest as null',
      'contracts.routeManifest',
      '"gapCount": 0',
      'Platform Framework Adoption Follow-up',
      'ADR-20260618-platform-framework-adoption.md',
      'apis/agent-business/authority-index.json',
      'platform-integration.mjs'
    ]
  }
];

export function validateWorkspaceEvidence({ kernelRoot, sdkworkSpecsRoot, errors, ensureFile, readFileIfExists }) {
  for (const specFile of requiredSdkworkSpecFiles) {
    ensureExternalFile(path.join(sdkworkSpecsRoot, specFile), `../sdkwork-specs/${specFile}`, errors);
  }

  for (const workspaceFile of requiredWorkspaceFiles) {
    ensureFile(workspaceFile);
  }

  for (const directory of standardRootDirectories) {
    ensureDirectory(kernelRoot, directory, errors);
    ensureFile(path.join(directory, 'README.md'));
  }

  for (const workspaceReadme of [
    '.sdkwork/README.md',
    '.sdkwork/skills/README.md',
    '.sdkwork/plugins/README.md'
  ]) {
    const content = readFileIfExists(path.join(kernelRoot, workspaceReadme));
    if (/\$(name|specPath)/.test(content)) {
      errors.push(`${workspaceReadme} must not retain template variables`);
    }
    if (!content.includes('../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md')) {
      errors.push(`${workspaceReadme} must cite ../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`);
    }
  }

  const rootReadme = readFileIfExists(path.join(kernelRoot, 'README.md'));
  if (!rootReadme.includes('../sdkwork-specs/README.md')) {
    errors.push('README.md must link the sibling ../sdkwork-specs/README.md standards entrypoint');
  }
  if (!rootReadme.includes('SDKWORK_WORKSPACE_SPEC.md')) {
    errors.push('README.md must cite SDKWORK_WORKSPACE_SPEC.md for the root directory dictionary');
  }
  for (const staleCommand of staleRootReadmeCommands) {
    if (rootReadme.includes(staleCommand)) {
      errors.push(`README.md must not document stale command: ${staleCommand}`);
    }
  }

  for (const filePath of listCurrentDictionaryFiles(kernelRoot)) {
    const content = readFileIfExists(filePath);
    for (const [label, pattern] of staleDictionaryPatterns) {
      if (pattern.test(content)) {
        errors.push(`${path.relative(kernelRoot, filePath)} must not contain stale ${label}`);
      }
    }
  }

  for (const evidenceFile of requiredStandardsEvidenceFiles) {
    validateRequiredStandardsEvidenceFile({ kernelRoot, errors, ensureFile, readFileIfExists, evidenceFile });
  }
}

export function listCurrentDictionaryFiles(kernelRoot) {
  const roots = [
    path.join(kernelRoot, 'README.md'),
    path.join(kernelRoot, 'AGENTS.md'),
    path.join(kernelRoot, 'specs'),
    path.join(kernelRoot, 'sdkwork-agent-kernel', 'README.md'),
    path.join(kernelRoot, 'sdkwork-agent-kernel', 'specs'),
    path.join(kernelRoot, 'sdkwork-code-kernel', 'README.md'),
    path.join(kernelRoot, 'sdkwork-code-kernel', 'specs'),
    path.join(kernelRoot, 'sdkwork-kernel-plugins', 'README.md'),
    path.join(kernelRoot, 'sdkwork-kernel-plugins', 'specs'),
    path.join(kernelRoot, 'sdkwork-kernel-ui', 'README.md'),
    path.join(kernelRoot, 'sdkwork-kernel-ui', 'specs'),
    path.join(kernelRoot, 'sdkwork-kernel-ui', 'packages')
  ];

  return roots.flatMap((scanRoot) => listDictionaryFiles(scanRoot));
}

function validateRequiredStandardsEvidenceFile({ kernelRoot, errors, ensureFile, readFileIfExists, evidenceFile }) {
  ensureFile(evidenceFile.relativePath);

  const content = readFileIfExists(path.join(kernelRoot, evidenceFile.relativePath));
  for (const text of evidenceFile.requiredText) {
    if (!content.includes(text)) {
      errors.push(`${evidenceFile.relativePath} must include ${text}`);
    }
  }
}

function ensureDirectory(kernelRoot, relativePath, errors) {
  const directoryPath = path.join(kernelRoot, relativePath);
  if (!fs.existsSync(directoryPath) || !fs.statSync(directoryPath).isDirectory()) {
    errors.push(`missing directory: ${relativePath}`);
  }
}

function ensureExternalFile(filePath, label, errors) {
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    errors.push(`missing file: ${label}`);
  }
}

function listDictionaryFiles(scanPath) {
  if (!fs.existsSync(scanPath)) {
    return [];
  }

  const stat = fs.statSync(scanPath);
  if (stat.isFile()) {
    return isDictionaryFile(scanPath) ? [scanPath] : [];
  }

  return fs.readdirSync(scanPath, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = path.join(scanPath, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name === 'generated') {
        return [];
      }
      return listDictionaryFiles(entryPath);
    }

    return isDictionaryFile(entryPath) ? [entryPath] : [];
  });
}

function isDictionaryFile(filePath) {
  return /(?:README\.md|component\.spec\.json|AGENTS?\.md|.*_SPEC\.md)$/.test(path.basename(filePath));
}
