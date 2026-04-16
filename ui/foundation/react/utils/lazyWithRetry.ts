import { lazy } from "react";
import type React from "react";

export function lazyWithRetry(
  factory: () => Promise<{ default: React.ComponentType<any> }>,
): React.LazyExoticComponent<React.ComponentType<any>> {
  return lazy(() =>
    factory().catch((error) => {
      if (error?.name === "ChunkLoadError" || error?.message?.includes("Loading chunk")) {
        return factory();
      }
      throw error;
    }),
  );
}
