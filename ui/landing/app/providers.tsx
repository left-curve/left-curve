"use client";

import { GrunnectProvider } from "@leftcurve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { config } from "~/grug.config";

import type React from "react";

const queryClient = new QueryClient();

export interface ProvidersProps {
  children: React.ReactNode;
}

export function Providers({ children }: ProvidersProps) {
  return (
    <GrunnectProvider config={config}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </GrunnectProvider>
  );
}
