import { lazy } from "react";
import type React from "react";

const REFRESH_KEY = "chunk_refresh_timestamp";
const REFRESH_COOLDOWN_MS = 10_000;

export function isChunkLoadError(error: Error): boolean {
  return (
    error.message.includes("Failed to fetch dynamically imported module") ||
    error.message.includes("Loading chunk") ||
    error.message.includes("Loading CSS chunk") ||
    error.name === "ChunkLoadError"
  );
}

export function reloadOnChunkError(): boolean {
  const lastRefresh = sessionStorage.getItem(REFRESH_KEY);
  const now = Date.now();

  if (!lastRefresh || now - Number.parseInt(lastRefresh, 10) > REFRESH_COOLDOWN_MS) {
    sessionStorage.setItem(REFRESH_KEY, now.toString());
    window.location.reload();
    return true;
  }
  return false;
}

export function lazyWithRetry(
  factory: () => Promise<{ default: React.ComponentType<any> }>,
): React.LazyExoticComponent<React.ComponentType<any>> {
  return lazy(() =>
    factory().catch((error) => {
      if (error instanceof Error && isChunkLoadError(error)) {
        if (reloadOnChunkError()) {
          return new Promise(() => {});
        }
      }
      throw error;
    }),
  );
}
