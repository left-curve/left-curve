import { Stack } from "expo-router";
import { AppProviders } from "~/app.providers";
import { AppTheme } from "~/app.theme";

export default function RootLayout() {
  return (
    <AppProviders>
      <AppTheme>
        <Stack>
          <Stack.Screen name="(app)" options={{ headerShown: false }} />
          <Stack.Screen name="(auth)" options={{ headerShown: false }} />
          <Stack.Screen
            name="search"
            options={{ headerShown: false, presentation: "containedModal" }}
          />
        </Stack>
      </AppTheme>
    </AppProviders>
  );
}
