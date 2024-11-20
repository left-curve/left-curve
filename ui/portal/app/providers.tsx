"use client";

import { GrunnectProvider } from "@leftcurve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { NuqsAdapter } from "nuqs/adapters/next/app";
import { config } from "../grunnect.config";

import "@leftcurve/types/window";
import { AuthProvider } from "./providers/AuthProvider";

const queryClient = new QueryClient();

export interface ProvidersProps {
  children: React.ReactNode;
}

export function Providers({ children }: ProvidersProps) {
  return (
    <GrunnectProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <NuqsAdapter>{children}</NuqsAdapter>
        </AuthProvider>
      </QueryClientProvider>
    </GrunnectProvider>
  );
}
