/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_SDKWORK_KERNEL_DEPLOYMENT_PROFILE?: string;
  readonly VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_HTTP_URL?: string;
  readonly VITE_SDKWORK_KERNEL_APPLICATION_PUBLIC_WEBSOCKET_URL?: string;
  readonly VITE_SDKWORK_KERNEL_PLATFORM_API_GATEWAY_HTTP_URL?: string;
  readonly VITE_KERNEL_API_URL?: string;
  readonly VITE_KERNEL_TENANT_ID?: string;
  readonly VITE_KERNEL_USER_ID?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
