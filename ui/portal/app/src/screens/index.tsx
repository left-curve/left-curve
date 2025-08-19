import { Text, View } from "react-native";
import { useInputs, twMerge } from "@left-curve/applets-kit";

export default function HomeScreen() {
  const { inputs } = useInputs();
  return (
    <View className={twMerge("flex-1 items-center justify-center bg-white", "bg-red-500")}>
      <Text className="text-xl font-bold text-tertiary-green">Welcome to Dango!</Text>
    </View>
  );
}
