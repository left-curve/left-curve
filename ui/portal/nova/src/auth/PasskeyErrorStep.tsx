import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Button, Spinner } from "../components";

type PasskeyErrorStepProps = {
  readonly onCreateAccount: () => void;
  readonly onBack: () => void;
  readonly isPending: boolean;
};

export function PasskeyErrorStep({ onCreateAccount, onBack, isPending }: PasskeyErrorStepProps) {
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
          No account found
        </Text>
        <Text
          className={twMerge(
            "font-text text-[13px]",
            "text-fg-secondary text-center",
            "max-w-[300px]",
          )}
        >
          No account is associated with this passkey. Create a new account to get started.
        </Text>
      </View>

      <View
        className={twMerge(
          "items-center justify-center",
          "h-16 mx-4",
          "rounded-card",
          "bg-down-bg",
        )}
      >
        <Text className="text-[24px] text-down">{"\u2717"}</Text>
      </View>

      <View className="flex flex-col gap-3">
        <Button variant="primary" size="lg" onPress={onCreateAccount} disabled={isPending}>
          {isPending ? (
            <View className="flex-row items-center gap-2">
              <Spinner size="sm" className="text-btn-primary-fg" />
              <Text className="font-medium text-btn-primary-fg text-[14px]">Creating...</Text>
            </View>
          ) : (
            <Text className="font-medium text-btn-primary-fg text-[14px]">Create Account</Text>
          )}
        </Button>
      </View>

      <View className="items-center">
        <Pressable onPress={onBack} disabled={isPending}>
          <Text className="text-fg-tertiary text-[12px] font-medium">Back</Text>
        </Pressable>
      </View>
    </View>
  );
}
