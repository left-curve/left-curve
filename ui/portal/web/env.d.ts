/// <reference types="@rsbuild/core/types" />

interface ImportMetaEnv {
  readonly GIT_COMMIT: string;
  readonly CONFIG_ENVIRONMENT: string;
  readonly PUBLIC_SWAPPER_INTEGRATOR_ID?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
