import { Spinner, isChunkLoadError, reloadOnChunkError } from "@left-curve/applets-kit";
import { captureException } from "@sentry/react";
import { useEffect, useRef } from "react";

import { NotFound } from "./NotFound";

import type React from "react";

type ErrorPageProps = {
  error: Error;
  reset: () => void;
};

export const ErrorPage: React.FC<ErrorPageProps> = ({ error, reset }) => {
  const handled = useRef(false);

  useEffect(() => {
    captureException(error);
  }, []);

  useEffect(() => {
    if (
      error instanceof Error &&
      isChunkLoadError(error) &&
      !reloadOnChunkError() &&
      !handled.current
    ) {
      handled.current = true;
      reset();
    }
  }, [error, reset]);

  if (error instanceof Error && isChunkLoadError(error)) {
    return (
      <div className="flex-1 w-full flex justify-center items-center h-screen">
        <Spinner size="lg" color="pink" />
      </div>
    );
  }

  return <NotFound />;
};
