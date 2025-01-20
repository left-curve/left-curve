import { GrunnectProvider } from "@left-curve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { config } from "../grunnect.config";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 0,
    },
  },
});

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <GrunnectProvider config={config}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </GrunnectProvider>
  );
};
