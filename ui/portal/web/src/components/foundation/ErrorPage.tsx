import { Spinner, isChunkLoadError, reloadOnChunkError } from "@left-curve/applets-kit";
import { captureException } from "@sentry/react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";

import { NotFound } from "./NotFound";

import type React from "react";

type ErrorPageProps = {
  error: Error;
  reset: () => void;
};

export const ErrorPage: React.FC<ErrorPageProps> = ({ error, reset }) => {
  const navigate = useNavigate();
  const isChunkError = error instanceof Error && isChunkLoadError(error);
  const handledChunkError = useRef(false);

  const { data: isChainRunning, isFetched } = useQuery({
    queryKey: ["error_page_chain_status"],
    enabled: !isChunkError,
    queryFn: async () => {
      try {
        const response = await fetch(window.dango.urls.upUrl);
        if (!response.ok) return false;
        const { is_running } = await response.json();
        return Boolean(is_running);
      } catch {
        return false;
      }
    },
  });

  useEffect(() => {
    if (isChunkError || !isFetched) return;
    if (isChainRunning) captureException(error);
    else navigate({ to: "/maintenance" });
  }, [error, isChunkError, isFetched, isChainRunning, navigate]);

  useEffect(() => {
    if (!isChunkError || handledChunkError.current) return;
    handledChunkError.current = true;
    if (!reloadOnChunkError()) {
      reset();
    }
  }, [isChunkError, reset]);

  if (isChunkError || !isFetched || !isChainRunning) {
    return (
      <div className="flex-1 w-full flex justify-center items-center h-screen">
        <Spinner size="lg" color="pink" />
      </div>
    );
  }

  return <NotFound />;
};
