import { Stack } from "expo-router";
import { config } from "~/store";
import { DangoStoreProvider } from "@left-curve/store";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { ThemeProvider } from "~/hooks/useTheme";
import { Layout } from "~/components";

const queryClient = new QueryClient();

export default function RootLayout() {
  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>
          <SafeAreaProvider>
            <Layout>
              <Stack>
                <Stack.Screen name="index" options={{ headerShown: false }} />
                <Stack.Screen name="+not-found" />
              </Stack>
            </Layout>
          </SafeAreaProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </DangoStoreProvider>
  );
}
