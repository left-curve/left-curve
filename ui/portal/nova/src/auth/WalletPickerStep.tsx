import { useState } from "react";
import { Image, Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useConnectors } from "@left-curve/store";
import { Button, Spinner } from "../components";

const EXCLUDED_CONNECTOR_TYPES: ReadonlySet<string> = new Set(["passkey", "session", "privy"]);

type WalletPickerStepProps = {
  readonly onSelectWallet: (connectorId: string) => void;
  readonly onBack: () => void;
  readonly isPending: boolean;
};

export function WalletPickerStep({ onSelectWallet, onBack, isPending }: WalletPickerStepProps) {
  const connectors = useConnectors();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const walletConnectors = connectors.filter((c) => !EXCLUDED_CONNECTOR_TYPES.has(c.type));

  const handleSelect = (connectorId: string) => {
    setSelectedId(connectorId);
    onSelectWallet(connectorId);
  };

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
          Connect Wallet
        </Text>
        <Text className={twMerge("font-text text-[13px]", "text-fg-secondary text-center")}>
          {walletConnectors.length > 0
            ? "Select a wallet to continue."
            : "No wallets detected. Install a browser wallet extension."}
        </Text>
      </View>

      {walletConnectors.length > 0 && (
        <View className="flex flex-col gap-2">
          {walletConnectors.map((connector) => {
            const isThisLoading = isPending && selectedId === connector.id;
            const isOtherLoading = isPending && selectedId !== connector.id;

            return (
              <Button
                key={connector.id}
                variant="secondary"
                size="lg"
                onPress={() => handleSelect(connector.id)}
                disabled={isOtherLoading}
              >
                <View className="flex-row items-center gap-3 flex-1">
                  {connector.icon && (
                    <Image
                      source={{ uri: connector.icon }}
                      style={{ width: 20, height: 20, borderRadius: 4 }}
                      accessibilityLabel={connector.name}
                    />
                  )}
                  <Text className="font-medium text-fg-primary text-[14px] flex-1">
                    {connector.name}
                  </Text>
                  {isThisLoading && <Spinner size="sm" className="text-fg-tertiary" />}
                </View>
              </Button>
            );
          })}
        </View>
      )}

      <View className="items-center">
        <Pressable onPress={onBack} disabled={isPending}>
          <Text className="text-fg-tertiary text-[12px] font-medium">Back</Text>
        </Pressable>
      </View>
    </View>
  );
}
