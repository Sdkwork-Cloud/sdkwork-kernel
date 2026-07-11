import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  buildProfileId,
  createTopologyRuntime,
  isTcpPortReachable,
  loadTopologySpec,
  normalizeText,
  waitForHttpHealthy,
} from '@sdkwork/app-topology';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const REPO_ROOT = path.resolve(__dirname, '..', '..');
export const SPEC_PATH = path.join(REPO_ROOT, 'specs/topology.spec.json');
export const API_GATEWAY_REPO = path.resolve(REPO_ROOT, '..', 'sdkwork-api-cloud-gateway');
export const IAM_REPO_ROOT = path.resolve(REPO_ROOT, '..', 'sdkwork-iam');

export const IAM_APPLICATION_BOOTSTRAP_ENV = {
  SDKWORK_APP_ROOT: REPO_ROOT,
  SDKWORK_IAM_APP_ROOT: IAM_REPO_ROOT,
  SDKWORK_KERNEL_APP_ROOT: REPO_ROOT,
};

const spec = loadTopologySpec(SPEC_PATH);
const runtime = createTopologyRuntime(spec, REPO_ROOT);

export const DEFAULT_DEV_PROFILE_ID = runtime.defaults.developmentProfileId;
export const DEFAULT_PRODUCTION_PROFILE_ID = runtime.defaults.productionProfileId;

export function resolveDevProfileId(deploymentProfile, environment = 'development') {
  const normalizedDeploymentProfile = runtime.assertDeploymentProfile(deploymentProfile);
  const normalizedEnvironment = runtime.assertEnvironment(environment);
  return buildProfileId(normalizedDeploymentProfile, normalizedEnvironment);
}

export function bridgeLegacyServiceEnv(profileEnv = {}) {
  const applicationHttpUrl =
    profileEnv.SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL
    ?? profileEnv.VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL;

  return {
    VITE_KERNEL_API_URL: profileEnv.VITE_KERNEL_API_URL ?? applicationHttpUrl,
  };
}

export const loadProfile = runtime.loadProfile;
export const applyProfileEnv = runtime.applyProfileEnv;
export const mergeRuntimeEnv = runtime.mergeRuntimeEnv;
export const loadEnvFile = runtime.loadEnvFile;
export const assertHosting = runtime.assertHosting;
export const resolveSurfaceHttpUrl = runtime.resolveSurfaceHttpUrl.bind(runtime);
export const resolveSurfaceWebsocketOrigin = runtime.resolveSurfaceWebsocketOrigin.bind(runtime);
export const resolveSurfaceBind = runtime.resolveSurfaceBind.bind(runtime);
export const shouldAutostartGateway = runtime.shouldAutostartGateway;
export const resolveGatewayBind = runtime.resolveGatewayBind;
export const resolveGatewayBaseUrl = runtime.resolveGatewayBaseUrl;
export const resolveIamDevEnv = runtime.resolveIamDevEnv;
export const listOrchestrationProcesses = runtime.listOrchestrationProcesses;
export const listHealthSurfaces = runtime.listHealthSurfaces;

export { buildProfileId, normalizeText, isTcpPortReachable, waitForHttpHealthy, spec, runtime };
