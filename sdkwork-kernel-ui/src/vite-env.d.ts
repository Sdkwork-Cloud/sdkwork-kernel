/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_KERNEL_API_URL?: string;
  readonly VITE_KERNEL_ACCESS_TOKEN?: string;
  readonly VITE_KERNEL_TENANT_ID?: string;
  readonly VITE_KERNEL_USER_ID?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
