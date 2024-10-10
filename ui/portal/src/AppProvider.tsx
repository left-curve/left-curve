import { GrunnectProvider } from "@leftcurve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { config } from "../grunnect.config";

const queryClient = new QueryClient();

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <GrunnectProvider config={config}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </GrunnectProvider>
  );
};
