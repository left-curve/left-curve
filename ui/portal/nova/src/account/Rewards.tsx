import { View, Text } from "react-native";
import { useAccount, usePoints } from "@left-curve/store";
import { Card, Chip, Divider, Skeleton } from "../components";

function formatPoints(value: number): string {
  return value.toLocaleString("en-US", { maximumFractionDigits: 0 });
}

export function Rewards() {
  const { userIndex, isConnected } = useAccount();
  const pointsUrl = typeof window !== "undefined" ? (window.dango?.urls?.pointsUrl ?? "") : "";

  const { points, tradingPoints, lpPoints, referralPoints, isLoading } = usePoints({
    pointsUrl,
    userIndex,
  });

  const breakdown = [
    { label: "Trading points", value: tradingPoints, type: "earn" as const },
    { label: "LP points", value: lpPoints, type: "earn" as const },
    { label: "Referral points", value: referralPoints, type: "earn" as const },
  ];

  return (
    <View className="flex flex-col gap-4">
      <Text className="text-fg-primary text-[20px] font-display font-semibold tracking-tight">
        Rewards
      </Text>

      <View className="grid grid-cols-1 md:grid-cols-2 gap-3" style={{ display: "grid" as never }}>
        <Card className="p-5">
          <Text className="text-fg-tertiary text-[11px] uppercase tracking-wide font-medium">
            Total points
          </Text>
          {isLoading ? (
            <Skeleton height={32} width={120} className="mt-1.5" />
          ) : (
            <Text className="text-fg-primary font-semibold text-[28px] tracking-tight tabular-nums mt-1.5">
              {isConnected ? formatPoints(points) : "\u2014"}
            </Text>
          )}
          <Text className="text-fg-tertiary text-[12px] mt-1">Lifetime accumulated</Text>
        </Card>

        <Card className="p-5">
          <Text className="text-fg-tertiary text-[11px] uppercase tracking-wide font-medium">
            Claimable
          </Text>
          <View className="flex flex-row items-center justify-between mt-1.5">
            <Text className="text-fg-tertiary font-semibold text-[28px] tracking-tight tabular-nums">
              {"\u2014"}
            </Text>
          </View>
          <Text className="text-fg-tertiary text-[12px] mt-1">Claiming not yet available</Text>
        </Card>
      </View>

      <Card className="p-5">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
          How to earn
        </Text>
        <View className="flex flex-col gap-3 mt-3">
          {[
            { label: "Trade on spot or perps", reward: "1 pt / $1 volume" },
            { label: "Provide liquidity", reward: "2 pts / $1 TVL / day" },
            { label: "Refer friends", reward: "200 pts per referral" },
            { label: "Daily check-in", reward: "50 pts / day" },
          ].map((item) => (
            <View
              key={item.label}
              className="flex flex-row items-center justify-between py-2 px-3 bg-bg-sunk rounded-field"
            >
              <Text className="text-fg-primary text-[13px]">{item.label}</Text>
              <Text className="text-accent text-[12px] font-medium">{item.reward}</Text>
            </View>
          ))}
        </View>
      </Card>

      <Card className="overflow-hidden">
        <View className="px-4 py-3">
          <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
            Points breakdown
          </Text>
        </View>

        {isLoading ? (
          <View className="flex flex-col gap-2 p-4">
            <Skeleton height={44} />
            <Skeleton height={44} />
            <Skeleton height={44} />
          </View>
        ) : !isConnected ? (
          <View className="p-8 items-center">
            <Text className="text-fg-tertiary text-[13px]">
              Connect your account to view points
            </Text>
          </View>
        ) : (
          breakdown.map((entry, i) => (
            <View key={entry.label}>
              {i > 0 && <Divider className="mx-4" />}
              <View className="flex flex-row items-center h-11 px-4 hover:bg-bg-sunk transition-[background] duration-150 ease-[var(--ease)]">
                <Text className="flex-1 text-fg-primary text-[13px]">{entry.label}</Text>
                <View className="w-[80px] flex flex-row justify-center">
                  <Chip variant="up">Earned</Chip>
                </View>
                <Text className="w-[100px] text-right text-[13px] font-medium tabular-nums text-up">
                  +{formatPoints(entry.value)}
                </Text>
              </View>
            </View>
          ))
        )}
      </Card>
    </View>
  );
}
