/// <reference types="@rsbuild/core/types" />

import type { HyperlaneConfig } from "@left-curve/types";

declare global {
  interface ImportMetaEnv {
    readonly GIT_COMMIT: string;
    readonly CONFIG_ENVIRONMENT: string;
    readonly HYPERLANE_CONFIG: HyperlaneConfig;
  }

  interface ImportMeta {
    readonly env: ImportMetaEnv;
  }
}
