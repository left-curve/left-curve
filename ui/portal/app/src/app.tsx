import { useTheme } from "./hooks/useTheme";

import { ctx } from "expo-router/_ctx";
import { ExpoRoot } from "expo-router";
import { DangoStoreProvider } from "@left-curve/store";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { SafeAreaProvider, SafeAreaView } from "react-native-safe-area-context";

import { config } from "~/store";
import { twMerge } from "@left-curve/foundation";

const queryClient = new QueryClient();

export const App: React.FC = () => {
  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ExpoRoot
          context={ctx}
          wrapper={({ children }) => {
            const { theme } = useTheme();
            return (
              <SafeAreaProvider>
                <SafeAreaView
                  className={twMerge(
                    theme,
                    "flex-1 bg-surface-primary-rice text-primary-900 diatype-m-medium relative",
                  )}
                >
                  {children}
                </SafeAreaView>
              </SafeAreaProvider>
            );
          }}
        />
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
