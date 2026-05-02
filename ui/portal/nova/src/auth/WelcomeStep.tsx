import { Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Button } from "../components";

type WelcomeStepProps = {
  readonly email: string;
  readonly onEmailChange: (email: string) => void;
  readonly onContinueEmail: () => void;
  readonly onConnectPasskey: () => void;
  readonly onConnectWallet: () => void;
  readonly onConnectPrivy: () => void;
  readonly isPending: boolean;
};

export function WelcomeStep({
  onContinueEmail,
  onConnectPasskey,
  onConnectWallet,
  onConnectPrivy,
  isPending,
}: WelcomeStepProps) {
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
          Welcome to Dango
        </Text>
        <Text
          className={twMerge(
            "font-text text-[13px]",
            "text-fg-secondary text-center",
            "max-w-[300px]",
          )}
        >
          Trade perpetuals, earn yield, and manage your portfolio.
        </Text>
      </View>

      <View className="flex flex-col gap-3">
        <Button variant="primary" size="lg" onPress={onContinueEmail} disabled={isPending}>
          <Text className="font-medium text-btn-primary-fg text-[14px]">Continue with Email</Text>
        </Button>

        {/* Divider */}
        <View className="flex-row items-center gap-3">
          <View className="flex-1 h-px bg-border-subtle" />
          <Text className="text-fg-quaternary text-[11px] font-medium tracking-wide uppercase">
            or
          </Text>
          <View className="flex-1 h-px bg-border-subtle" />
        </View>

        <View className="flex flex-col gap-2">
          <Button variant="secondary" size="lg" onPress={onConnectPrivy} disabled={isPending}>
            <Text className="text-fg-primary text-[14px] font-medium">Continue with Google</Text>
          </Button>

          <Button variant="secondary" size="lg" onPress={onConnectPasskey} disabled={isPending}>
            <Text className="text-fg-primary text-[14px] font-medium">Connect with Passkey</Text>
          </Button>
        </View>
      </View>

      <View className="items-center">
        <Button variant="ghost" size="sm" onPress={onConnectWallet} disabled={isPending}>
          <Text className="text-fg-tertiary text-[12px] font-medium">Connect Wallet</Text>
        </Button>
      </View>
    </View>
  );
}
