import { useTheme } from "./hooks/useTheme";

import { ExpoRoot, useRouter } from "expo-router";
import { ctx } from "expo-router/_ctx";
import { DangoStoreProvider } from "@left-curve/store";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";

import { config } from "~/store";
import { SafeAreaProvider, SafeAreaView } from "react-native-safe-area-context";
import { AppProvider, twMerge } from "@left-curve/foundation";

import type { ToastController } from "@left-curve/foundation";

const queryClient = new QueryClient();

export const App: React.FC = () => {
  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ExpoRoot
          context={ctx}
          wrapper={({ children }) => {
            const { theme } = useTheme();
            const { navigate } = useRouter();

            return (
              <AppProvider toast={{} as ToastController} navigate={(to) => navigate(to)}>
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
              </AppProvider>
            );
          }}
        />
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
