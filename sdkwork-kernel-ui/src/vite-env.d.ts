/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_KERNEL_API_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
