import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  listComponentSpecFiles,
  validateComponentCanonicalSpecPaths,
  validateComponentSpecMetadata
} from './component-specs.mjs';
import { validateAgentKnowledgeMemoryContracts } from './agent-knowledge-memory-contracts.mjs';
import { validateKernelContracts } from './kernel-contracts.mjs';
import { validateKernelUiPackages } from './ui-packages.mjs';
import { validateKernelTopology } from './kernel-topology.mjs';
import { validatePlatformIntegration } from './platform-integration.mjs';
import { validateWorkspaceEvidence } from './workspace-evidence.mjs';

export function runKernelStandardsCheck() {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const kernelRoot = resolveKernelRoot();
  const sdkworkSpecsRoot = path.resolve(kernelRoot, '..', 'sdkwork-specs');

  const errors = [];

  validateWorkspaceEvidence({ kernelRoot, sdkworkSpecsRoot, errors, ensureFile, readFileIfExists });
  validatePlatformIntegration({ kernelRoot, errors, ensureFile, readFileIfExists });

  const componentSpecContext = { kernelRoot, errors };
  for (const manifestPath of listComponentSpecFiles(kernelRoot)) {
    validateComponentCanonicalSpecPaths(manifestPath, componentSpecContext);
    validateComponentSpecMetadata(manifestPath, componentSpecContext);
  }

  function ensureFile(relativePath) {
    const filePath = path.join(kernelRoot, relativePath);
    if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
      errors.push(`missing file: ${relativePath}`);
    }
  }

  function readJson(relativePath) {
    const filePath = path.join(kernelRoot, relativePath);
    try {
      return JSON.parse(fs.readFileSync(filePath, 'utf8'));
    } catch (error) {
      errors.push(`invalid json: ${relativePath}: ${error.message}`);
      return null;
    }
  }

  validateKernelContracts({ kernelRoot, errors, ensureFile, readJson, readFileIfExists });
  validateKernelUiPackages({ kernelRoot, errors, ensureFile, readJson });
  validateKernelTopology({ kernelRoot, errors, readJson });

  ensureFile(path.join('scripts', 'check-agent-sdk-workspace.mjs'));
  ensureFile(path.join('scripts', 'check-external-agent-sdk-bindings.mjs'));
  const agentSdkWorkspaceCheck = spawnSync(
    process.execPath,
    [path.join(kernelRoot, 'scripts', 'check-agent-sdk-workspace.mjs')],
    {
      cwd: kernelRoot,
      encoding: 'utf8'
    }
  );
  if (agentSdkWorkspaceCheck.status !== 0) {
    errors.push(
      `agent SDK workspace check failed:\n${agentSdkWorkspaceCheck.stdout}${agentSdkWorkspaceCheck.stderr}`
    );
  }

  const externalAgentSdkBindingCheck = spawnSync(
    process.execPath,
    [path.join(kernelRoot, 'scripts', 'check-external-agent-sdk-bindings.mjs')],
    {
      cwd: kernelRoot,
      encoding: 'utf8'
    }
  );
  if (externalAgentSdkBindingCheck.status !== 0) {
    errors.push(
      `external agent SDK binding check failed:\n${externalAgentSdkBindingCheck.stdout}${externalAgentSdkBindingCheck.stderr}`
    );
  }

  validateAgentKnowledgeMemoryContracts({ kernelRoot, errors, readFileIfExists });

  if (errors.length > 0) {
    console.error(errors.map((error) => `- ${error}`).join('\n'));
    process.exit(1);
  }

  console.log('Kernel standards conformance check passed.');

  function resolveKernelRoot() {
    const candidates = [process.cwd(), path.resolve(process.cwd(), 'kernel'), path.resolve(scriptDir, '..')];
    const visited = new Set();

    for (const candidate of candidates) {
      const resolved = path.resolve(candidate);
      if (visited.has(resolved)) {
        continue;
      }
      visited.add(resolved);

      if (
        fs.existsSync(path.join(resolved, 'AGENTS.md')) &&
        fs.existsSync(path.join(resolved, 'specs', 'README.md')) &&
        fs.existsSync(path.join(resolved, 'sdkwork-agent-kernel', 'Cargo.toml'))
      ) {
        return resolved;
      }
    }

    console.error('Unable to resolve sdkwork-kernel repository root. Run from the repository root or scripts directory.');
    process.exit(1);
  }

  function readFileIfExists(filePath) {
    if (!fs.existsSync(filePath)) {
      return '';
    }

    return fs.readFileSync(filePath, 'utf8');
  }

}
