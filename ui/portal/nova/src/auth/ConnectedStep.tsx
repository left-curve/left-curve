import { Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Button } from "../components";

import type { Username } from "@left-curve/dango/types";

type ConnectedStepProps = {
  readonly username: Username;
  readonly onDisconnect: () => void;
  readonly onClose: () => void;
};

export function ConnectedStep({ username, onDisconnect, onClose }: ConnectedStepProps) {
  return (
    <View className="flex flex-col gap-6 px-6 pb-6 pt-2">
      <View className="flex flex-col gap-3 items-center">
        {/* Success avatar */}
        <View
          className={twMerge(
            "w-14 h-14 items-center justify-center",
            "rounded-full",
            "bg-accent-bg",
          )}
        >
          <Text className="text-accent text-[22px] font-bold">
            {username.charAt(0).toUpperCase()}
          </Text>
        </View>

        <Text
          className={twMerge(
            "font-display font-bold",
            "text-[20px] text-fg-primary",
            "text-center",
          )}
        >
          Connected
        </Text>
        <Text
          className={twMerge(
            "font-text text-[13px]",
            "text-fg-secondary text-center",
            "max-w-[280px]",
          )}
        >
          Signed in as
        </Text>
        <View
          className={twMerge(
            "px-3 py-1.5",
            "rounded-chip",
            "bg-bg-tint",
            "border border-border-subtle",
          )}
        >
          <Text className="text-fg-primary text-[13px] font-medium">{username}</Text>
        </View>
      </View>

      <View className="flex flex-col gap-2">
        <Button variant="primary" size="lg" onPress={onClose}>
          <Text className="font-medium text-btn-primary-fg text-[14px]">Continue</Text>
        </Button>

        <Button variant="ghost" size="lg" onPress={onDisconnect}>
          <Text className="font-medium text-down text-[14px]">Disconnect</Text>
        </Button>
      </View>
    </View>
  );
}
