import { Spinner } from "@left-curve/applets-kit";
import { captureException } from "@sentry/react";
import { useEffect } from "react";

import { NotFound } from "./NotFound";

import type React from "react";

type ErrorPageProps = {
  error: Error;
  reset: () => void;
};

const isChunkLoadError = (error: Error): boolean => {
  return (
    error.message.includes("Failed to fetch dynamically imported module") ||
    error.message.includes("Loading chunk") ||
    error.message.includes("Loading CSS chunk") ||
    error.name === "ChunkLoadError"
  );
};

const handleRetry = ({ reset }: { reset: () => void }) => {
  const refreshKey = "chunk_refresh_timestamp";
  const lastRefresh = sessionStorage.getItem(refreshKey);
  const now = Date.now();

  // Only auto-refresh if we haven't refreshed in the last 10 seconds
  if (!lastRefresh || now - Number.parseInt(lastRefresh, 10) > 10000) {
    sessionStorage.setItem(refreshKey, now.toString());
    window.location.reload();
  } else {
    // If we've already tried refreshing, just reset and let the user try again manually
    reset();
  }
};

export const ErrorPage: React.FC<ErrorPageProps> = ({ error, reset }) => {
  useEffect(() => {
    captureException(error);
  }, []);

  if (error instanceof Error && isChunkLoadError(error)) {
    handleRetry({ reset });
    return (
      <div className="flex-1 w-full flex justify-center items-center h-screen">
        <Spinner size="lg" color="pink" />
      </div>
    );
  }

  return <NotFound />;
};
