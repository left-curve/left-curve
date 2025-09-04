import { View } from "react-native";
import { Landing } from "~/components/Landing/Landing";

export default function HomeScreen() {
  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8">
      <Landing>
        <Landing.Overview />
      </Landing>
    </View>
  );
}
