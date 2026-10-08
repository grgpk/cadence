/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** The API origin the browser calls. Empty in production, where it is the same origin. */
  readonly VITE_API_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
