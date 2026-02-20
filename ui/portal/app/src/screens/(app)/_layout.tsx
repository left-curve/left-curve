import { Stack } from "expo-router";
import { View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { Menu } from "~/components/foundation";

export default function AppLayout() {
  const insets = useSafeAreaInsets();
  return (
    <View className="flex-1 bg-surface-primary-rice">
      <View style={{ paddingBottom: insets.bottom + 80 }} className="flex-1">
        <Stack>
          <Stack.Screen name="index" options={{ headerShown: false }} />
          <Stack.Screen name="convert" options={{ headerShown: false }} />
          <Stack.Screen name="settings" options={{ headerShown: false }} />
          <Stack.Screen name="tx/[txHash]" options={{ headerShown: false }} />
          <Stack.Screen name="block/[block]" options={{ headerShown: false }} />
          <Stack.Screen name="account/[address]" options={{ headerShown: false }} />
          <Stack.Screen name="contract/[address]" options={{ headerShown: false }} />
        </Stack>
      </View>
      <Menu />
    </View>
  );
}
