import { View } from "react-native";
import { GlobalText } from "~/components/foundation";

export default function AuthScreen() {
  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8">
      <GlobalText>Signin</GlobalText>
    </View>
  );
}
