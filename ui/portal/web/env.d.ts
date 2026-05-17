/// <reference types="@rsbuild/core/types" />

interface ImportMetaEnv {
  readonly GIT_COMMIT: string;
  readonly CONFIG_ENVIRONMENT: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
