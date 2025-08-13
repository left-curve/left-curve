import { Link, Stack } from "expo-router";
import { Text, View } from "react-native";

export default function NotFoundScreen() {
  return (
    <>
      <Stack.Screen options={{ title: "Oops!" }} />
      <View className="h-full w-screen flex flex-col gap-4 items-center justify-center">
        <Link href="/">
          <Text className="text-tertiary-green">Go to home screen!</Text>
        </Link>
      </View>
    </>
  );
}
