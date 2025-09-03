import { Stack } from "expo-router";
import { View } from "react-native";
import { Menu } from "~/components/foundation";

export default function AppLayout() {
  return (
    <View className="flex-1 bg-surface-primary-rice">
      <Stack>
        <Stack.Screen name="index" options={{ headerShown: false }} />
        <Stack.Screen name="swap" options={{ headerShown: false }} />
      </Stack>
      <Menu />
    </View>
  );
}
