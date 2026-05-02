import { useMemo } from "react";
import { View, Text } from "react-native";
import { Decimal } from "@left-curve/dango/utils";
import { useAccount, usePerpsVaultUserShares } from "@left-curve/store";
import { Card, FormattedNumber, Skeleton } from "../components";
import { VaultList } from "./VaultList";

function useTotalDeposited(): { value: Decimal; isLoading: boolean } {
  const { userSharesValue, vaultState } = usePerpsVaultUserShares();

  const value = useMemo(() => Decimal(userSharesValue || "0"), [userSharesValue]);

  return { value, isLoading: !vaultState };
}

export function EarnScreen() {
  const { account } = useAccount();
  const { value: totalDeposited, isLoading } = useTotalDeposited();
  const isConnected = !!account;

  return (
    <View className="flex-1 py-8 px-4">
      <View className="w-full max-w-[960px] mx-auto flex flex-col gap-6">
        <View className="flex flex-col gap-2">
          <Text className="text-fg-primary font-display text-[28px] font-semibold tracking-tight">
            Earn
          </Text>
          <Text className="text-fg-secondary text-[13px] leading-relaxed">
            Deposit into vaults to earn yield on your assets.
          </Text>
        </View>

        <Card className="flex flex-row items-center justify-between p-5">
          <View className="flex flex-col gap-1">
            <Text className="text-fg-tertiary text-[11px] uppercase tracking-wide font-medium">
              Total Deposited
            </Text>
            {isLoading ? (
              <Skeleton width={100} height={24} />
            ) : isConnected ? (
              <FormattedNumber
                value={totalDeposited.toString()}
                className="text-[20px] font-semibold tracking-tight"
                formatOptions={{ currency: "USD" }}
              />
            ) : (
              <Text className="text-fg-quaternary text-[20px] font-semibold tracking-tight tabular-nums">
                --
              </Text>
            )}
          </View>
        </Card>

        <View className="flex flex-col gap-3">
          <Text className="text-fg-primary text-[16px] font-semibold tracking-tight">Vaults</Text>
          <VaultList />
        </View>
      </View>
    </View>
  );
}
