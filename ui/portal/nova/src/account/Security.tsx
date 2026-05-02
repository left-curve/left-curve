import { View, Text } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { decodeBase64, encodeHex } from "@left-curve/dango/encoding";
import { useAccount, useSigningClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { Button, Card, Chip, Divider, Skeleton } from "../components";

const KEY_TYPE_LABELS: Record<string, string> = {
  secp256r1: "WebAuthn",
  secp256k1: "Secp256k1",
  ethereum: "Ethereum",
  SECP256R1: "WebAuthn",
  SECP256K1: "Secp256k1",
  ETHEREUM: "Ethereum",
};

const KEY_TYPE_SHORT: Record<string, string> = {
  secp256r1: "WA",
  secp256k1: "SK",
  ethereum: "ET",
  SECP256R1: "WA",
  SECP256K1: "SK",
  ETHEREUM: "ET",
};

function KeyRow({
  keyData,
  isCurrentKey,
}: {
  keyData: {
    keyHash: string;
    keyType: string;
    publicKey: string;
    createdAt?: string;
  };
  isCurrentKey: boolean;
}) {
  const isEthereumKey = keyData.keyType === "ETHEREUM" || keyData.keyType === "ethereum";
  const keyRepresentation = isEthereumKey
    ? keyData.publicKey
    : `0x${encodeHex(decodeBase64(keyData.publicKey))}`;
  const truncatedKey = `${keyRepresentation.slice(0, 10)}...${keyRepresentation.slice(-6)}`;

  return (
    <View className="flex flex-row items-center py-3 px-1">
      <View className="flex flex-row items-center gap-3 flex-1">
        <View
          className={twMerge(
            "w-9 h-9 rounded-field items-center justify-center",
            isCurrentKey ? "bg-up-bg" : "bg-bg-sunk",
          )}
        >
          <Text
            className={twMerge(
              "text-[11px] font-semibold",
              isCurrentKey ? "text-up" : "text-fg-tertiary",
            )}
          >
            {KEY_TYPE_SHORT[keyData.keyType] ?? "??"}
          </Text>
        </View>

        <View className="flex flex-col gap-0.5">
          <View className="flex flex-row items-center gap-2">
            <Text className="text-fg-primary text-[13px] font-medium">{truncatedKey}</Text>
            <Chip variant={isCurrentKey ? "up" : "default"}>
              {isCurrentKey ? "Active" : "Inactive"}
            </Chip>
          </View>
          <Text className="text-fg-tertiary text-[11px]">
            {KEY_TYPE_LABELS[keyData.keyType] ?? keyData.keyType}
          </Text>
        </View>
      </View>

      {!isCurrentKey && (
        <Button variant="ghost" size="sm">
          <Text className="text-down text-[12px]">Remove</Text>
        </Button>
      )}
    </View>
  );
}

export function Security() {
  const { userIndex, keyHash: currentKeyHash, isConnected } = useAccount();
  const { data: signingClient } = useSigningClient();

  const { data: keys = [], isPending } = useQuery({
    enabled: !!signingClient && !!userIndex,
    queryKey: ["user_keys", userIndex],
    queryFn: async () => await signingClient?.getUserKeys({ userIndex: userIndex! }),
  });

  return (
    <View className="flex flex-col gap-4">
      <View className="flex flex-row items-center justify-between">
        <Text className="text-fg-primary text-[20px] font-display font-semibold tracking-tight">
          Security
        </Text>
        <Button variant="primary" size="default">
          <Text className="text-btn-primary-fg text-[13px]">Add Key</Text>
        </Button>
      </View>

      <Card className="p-5">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
          Connected keys
        </Text>
        <Text className="text-fg-tertiary text-[12px] mt-1">
          Manage the signing keys associated with your account.
        </Text>

        <View className="flex flex-col mt-4">
          {isPending ? (
            <View className="flex flex-col gap-3">
              <Skeleton height={56} />
              <Skeleton height={56} />
            </View>
          ) : !isConnected ? (
            <View className="p-4 bg-bg-sunk rounded-field items-center">
              <Text className="text-fg-tertiary text-[13px]">
                Connect your account to view keys
              </Text>
            </View>
          ) : keys.length === 0 ? (
            <View className="p-4 bg-bg-sunk rounded-field items-center">
              <Text className="text-fg-tertiary text-[13px]">No keys found</Text>
            </View>
          ) : (
            keys.map((keyData, i) => (
              <View key={keyData.keyHash}>
                {i > 0 && <Divider />}
                <KeyRow keyData={keyData} isCurrentKey={keyData.keyHash === currentKeyHash} />
              </View>
            ))
          )}
        </View>
      </Card>

      <Card className="p-5">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
          Spending limits
        </Text>
        <Text className="text-fg-tertiary text-[12px] mt-1">
          Configure per-key spending limits for additional protection.
        </Text>
        <View className="mt-4 p-4 bg-bg-sunk rounded-field items-center">
          <Text className="text-fg-tertiary text-[13px]">No spending limits configured</Text>
        </View>
      </Card>
    </View>
  );
}
