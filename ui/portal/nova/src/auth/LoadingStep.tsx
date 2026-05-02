import { Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Spinner } from "../components";

export function LoadingStep() {
  return (
    <View className="flex flex-col gap-6 px-6 pb-6 pt-2">
      <View className="flex flex-col gap-2 items-center">
        <Text
          className={twMerge(
            "font-display font-bold",
            "text-[20px] text-fg-primary",
            "text-center",
          )}
        >
          Signing you in
        </Text>
        <Text className={twMerge("font-text text-[13px]", "text-fg-secondary text-center")}>
          Hold tight while we set things up.
        </Text>
      </View>

      <View className="items-center justify-center py-6">
        <Spinner size="lg" className="text-accent" />
      </View>
    </View>
  );
}
