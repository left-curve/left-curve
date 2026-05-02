import { useState } from "react";
import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Button, Input, Spinner } from "../components";

type CreateAccountStepProps = {
  readonly identifier: string | undefined;
  readonly referrer: number | undefined;
  readonly onReferrerChange: (referrer: number | undefined) => void;
  readonly onContinue: () => void;
  readonly onBack: () => void;
  readonly isPending: boolean;
};

const truncateIdentifier = (id: string): string =>
  id.length > 20 ? `${id.slice(0, 6)}...${id.slice(-4)}` : id;

export function CreateAccountStep({
  identifier,
  referrer,
  onReferrerChange,
  onContinue,
  onBack,
  isPending,
}: CreateAccountStepProps) {
  const [showAdvanced, setShowAdvanced] = useState(false);

  const referrerFromQuery = (() => {
    if (typeof window === "undefined") return undefined;
    const ref = new URLSearchParams(window.location.search).get("ref");
    if (!ref) return undefined;
    const parsed = Number.parseInt(ref, 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
  })();

  const isReferrerLocked = referrerFromQuery !== undefined;

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
          Create your account
        </Text>
        <Text
          className={twMerge(
            "font-text text-[13px]",
            "text-fg-secondary text-center",
            "max-w-[300px]",
          )}
        >
          {identifier
            ? `No account found for ${truncateIdentifier(identifier)}. We'll create one for you.`
            : "Setting up your new account."}
        </Text>
      </View>

      <View className="flex flex-col gap-4">
        <Button variant="primary" size="lg" onPress={onContinue} disabled={isPending}>
          {isPending ? (
            <View className="flex-row items-center gap-2">
              <Spinner size="sm" className="text-btn-primary-fg" />
              <Text className="font-medium text-btn-primary-fg text-[14px]">Creating...</Text>
            </View>
          ) : (
            <Text className="font-medium text-btn-primary-fg text-[14px]">Continue</Text>
          )}
        </Button>

        {showAdvanced && (
          <Input
            label="Referral Code"
            value={referrer?.toString() ?? ""}
            onChangeText={(val) => {
              if (isReferrerLocked) return;
              const parsed = val ? Number.parseInt(val, 10) : undefined;
              onReferrerChange(parsed && Number.isFinite(parsed) ? parsed : undefined);
            }}
            placeholder="Enter referral code"
            keyboardType="number-pad"
            disabled={isReferrerLocked}
          />
        )}

        <Pressable onPress={() => setShowAdvanced((prev) => !prev)} className="self-center">
          <Text className="text-fg-secondary text-[12px] font-medium">
            {showAdvanced ? "Hide" : "Advanced Options"}{" "}
            <Text
              style={
                {
                  transform: showAdvanced ? "rotate(180deg)" : "none",
                } as never
              }
            >
              {"\u25BE"}
            </Text>
          </Text>
        </Pressable>
      </View>

      <View className="items-center">
        <Pressable onPress={onBack} disabled={isPending}>
          <Text className="text-fg-tertiary text-[12px] font-medium">Back</Text>
        </Pressable>
      </View>
    </View>
  );
}
