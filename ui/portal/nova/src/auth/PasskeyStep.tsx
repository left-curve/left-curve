import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Button, Spinner } from "../components";

type PasskeyStepProps = {
  readonly onCreatePasskey: () => void;
  readonly onUseExisting: () => void;
  readonly onBack: () => void;
  readonly isCreating: boolean;
  readonly isLogging: boolean;
};

export function PasskeyStep({
  onCreatePasskey,
  onUseExisting,
  onBack,
  isCreating,
  isLogging,
}: PasskeyStepProps) {
  const isBusy = isCreating || isLogging;

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
          Passkey Authentication
        </Text>
        <Text
          className={twMerge(
            "font-text text-[13px]",
            "text-fg-secondary text-center",
            "max-w-[300px]",
          )}
        >
          Use your device's biometric or hardware key to sign in securely.
        </Text>
      </View>

      {/* Passkey icon */}
      <View
        className={twMerge(
          "items-center justify-center",
          "h-24 mx-4",
          "rounded-card",
          "bg-bg-sunk",
        )}
      >
        {isBusy ? (
          <Spinner size="lg" className="text-accent" />
        ) : (
          <Text className="text-[40px] text-fg-quaternary">{"\uD83D\uDD12"}</Text>
        )}
      </View>

      <View className="flex flex-col gap-3">
        <Button variant="primary" size="lg" onPress={onUseExisting} disabled={isBusy}>
          {isLogging ? (
            <View className="flex-row items-center gap-2">
              <Spinner size="sm" className="text-btn-primary-fg" />
              <Text className="font-medium text-btn-primary-fg text-[14px]">Signing in...</Text>
            </View>
          ) : (
            <Text className="font-medium text-btn-primary-fg text-[14px]">
              Use Existing Passkey
            </Text>
          )}
        </Button>

        <Button variant="secondary" size="lg" onPress={onCreatePasskey} disabled={isBusy}>
          {isCreating ? (
            <View className="flex-row items-center gap-2">
              <Spinner size="sm" className="text-fg-primary" />
              <Text className="font-medium text-fg-primary text-[14px]">Creating...</Text>
            </View>
          ) : (
            <Text className="font-medium text-fg-primary text-[14px]">Create New Passkey</Text>
          )}
        </Button>
      </View>

      <View className="items-center">
        <Pressable onPress={onBack} disabled={isBusy}>
          <Text className="text-fg-tertiary text-[12px] font-medium">Back</Text>
        </Pressable>
      </View>
    </View>
  );
}
