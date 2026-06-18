import path from 'node:path';
import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

const rootDir = path.dirname(fileURLToPath(import.meta.url));

const packageAlias = (packageDir: string, entry = 'index.ts') =>
  path.resolve(rootDir, 'packages', packageDir, 'src', entry);

function resolveUiDevServer() {
  const bind = process.env.SDKWORK_KERNEL_UI_DEV_BIND ?? '127.0.0.1:5179';
  const separatorIndex = bind.lastIndexOf(':');
  if (separatorIndex <= 0) {
    return { host: '127.0.0.1', port: 5179 };
  }
  const host = bind.slice(0, separatorIndex);
  const port = Number.parseInt(bind.slice(separatorIndex + 1), 10);
  return {
    host: host || '127.0.0.1',
    port: Number.isFinite(port) ? port : 5179
  };
}

const uiDevServer = resolveUiDevServer();

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@sdkwork/kernel-ui-agent': packageAlias('sdkwork-kernel-ui-agent', 'index.tsx'),
      '@sdkwork/kernel-ui-code': packageAlias('sdkwork-kernel-ui-code', 'index.tsx'),
      '@sdkwork/kernel-ui-commons': packageAlias('sdkwork-kernel-ui-commons', 'index.tsx'),
      '@sdkwork/kernel-ui-core': packageAlias('sdkwork-kernel-ui-core'),
      '@sdkwork/kernel-ui-permissions': packageAlias('sdkwork-kernel-ui-permissions', 'index.tsx'),
      '@sdkwork/kernel-ui-services': packageAlias('sdkwork-kernel-ui-services'),
      '@sdkwork/kernel-ui-terminal': packageAlias('sdkwork-kernel-ui-terminal', 'index.tsx'),
      '@sdkwork/kernel-ui-telemetry': packageAlias('sdkwork-kernel-ui-telemetry', 'index.tsx'),
      '@sdkwork/kernel-ui-types': packageAlias('sdkwork-kernel-ui-types'),
      '@sdkwork/kernel-ui-workspace': packageAlias('sdkwork-kernel-ui-workspace', 'index.tsx')
    }
  },
  server: {
    host: uiDevServer.host,
    port: uiDevServer.port
  }
});
