/// <reference types="@rsbuild/core/types" />

import type { HyperlaneConfig } from "@left-curve/types";

declare global {
  interface ImportMetaEnv {
    readonly GIT_COMMIT: string;
    readonly CONFIG_ENVIRONMENT: string;
    readonly HYPERLANE_CONFIG: HyperlaneConfig;
    readonly PUBLIC_SWAPPER_INTEGRATOR_ID?: string;
  }

  interface ImportMeta {
    readonly env: ImportMetaEnv;
  }
}
