import { GlobalText } from "@left-curve/foundation-app";
import { View } from "react-native";

export default function AuthScreen() {
  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8">
      <GlobalText>Signin</GlobalText>
    </View>
  );
}
