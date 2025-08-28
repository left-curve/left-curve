import { DangoStoreProvider } from "@left-curve/store";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { ThemeProvider } from "~/hooks/useTheme";
import { config } from "~/store";

import type { PropsWithChildren } from "react";
import { StatusBar } from "react-native";

const queryClient = new QueryClient();

export const AppProviders: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>
          <SafeAreaProvider>
            <StatusBar />
            {children}
          </SafeAreaProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
