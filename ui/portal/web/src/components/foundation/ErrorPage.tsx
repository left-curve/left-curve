import { Spinner, isChunkLoadError, reloadOnChunkError } from "@left-curve/applets-kit";
import { captureException } from "@sentry/react";
import { useEffect } from "react";

import { NotFound } from "./NotFound";

import type React from "react";

type ErrorPageProps = {
  error: Error;
  reset: () => void;
};

export const ErrorPage: React.FC<ErrorPageProps> = ({ error, reset }) => {
  useEffect(() => {
    captureException(error);
  }, []);

  if (error instanceof Error && isChunkLoadError(error)) {
    if (!reloadOnChunkError()) {
      reset();
    }
    return (
      <div className="flex-1 w-full flex justify-center items-center h-screen">
        <Spinner size="lg" color="pink" />
      </div>
    );
  }

  return <NotFound />;
};
