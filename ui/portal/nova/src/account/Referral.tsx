import { View, Text } from "react-native";
import { Decimal } from "@left-curve/dango/utils";
import {
  useAccount,
  useReferralData,
  useRefereeStats,
  useReferralSettings,
  getReferralCode,
  getReferralLink,
} from "@left-curve/store";
import { Button, Card, Skeleton } from "../components";

type DecimalValue = ReturnType<typeof Decimal>;

function formatUsd(value: DecimalValue, decimals = 2): string {
  const num = Number(value.toFixed(decimals));
  return `$${num.toLocaleString("en-US", { minimumFractionDigits: decimals, maximumFractionDigits: decimals })}`;
}

function StatCard({
  label,
  value,
  isLoading,
}: {
  label: string;
  value: string;
  isLoading: boolean;
}) {
  return (
    <View className="flex flex-col gap-1">
      <Text className="text-fg-tertiary text-[11px] uppercase tracking-wide font-medium">
        {label}
      </Text>
      {isLoading ? (
        <Skeleton height={28} width={80} />
      ) : (
        <Text className="text-fg-primary text-[20px] font-semibold tracking-tight tabular-nums">
          {value}
        </Text>
      )}
    </View>
  );
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text);
}

export function Referral() {
  const { userIndex, isConnected } = useAccount();

  const { referralData, isLoading: dataLoading } = useReferralData({ userIndex });
  const { settings, isLoading: settingsLoading } = useReferralSettings({ userIndex });
  const { referees, isLoading: refereesLoading } = useRefereeStats({
    referrerIndex: userIndex,
    enabled: isConnected,
  });

  const isLoading = dataLoading || settingsLoading;
  const isReferrer = isConnected && !settingsLoading && settings != null;

  const referralCode = getReferralCode(userIndex);
  const shareLink = getReferralLink(userIndex);

  const totalReferees = referralData?.refereeCount ?? 0;
  const totalEarned = Decimal(referralData?.commissionEarnedFromReferees ?? "0");

  return (
    <View className="flex flex-col gap-4">
      <Text className="text-fg-primary text-[20px] font-display font-semibold tracking-tight">
        Referral
      </Text>

      <Card className="p-5">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
          Your referral code
        </Text>
        <Text className="text-fg-tertiary text-[12px] mt-1">
          Share your code and earn a share of referred trading fees.
        </Text>

        {!isConnected ? (
          <View className="mt-4 p-4 bg-bg-sunk rounded-field items-center">
            <Text className="text-fg-tertiary text-[13px]">
              Connect your account to get a referral code
            </Text>
          </View>
        ) : (
          <>
            <View className="flex flex-row items-center gap-3 mt-4">
              <View className="flex-1 flex flex-row items-center h-10 px-4 bg-bg-sunk rounded-field">
                <Text className="text-fg-primary font-mono text-[14px] font-semibold tracking-wide flex-1">
                  {referralCode || "\u2014"}
                </Text>
              </View>
              <Button
                variant="secondary"
                size="default"
                onPress={() => copyToClipboard(referralCode)}
              >
                <Text className="text-fg-primary text-[13px]">Copy</Text>
              </Button>
            </View>

            <View className="flex flex-row items-center gap-3 mt-3">
              <View className="flex-1 flex flex-row items-center h-10 px-4 bg-bg-sunk rounded-field overflow-hidden">
                <Text className="text-fg-secondary font-mono text-[12px] flex-1" numberOfLines={1}>
                  {shareLink || "\u2014"}
                </Text>
              </View>
              <Button variant="secondary" size="default" onPress={() => copyToClipboard(shareLink)}>
                <Text className="text-fg-primary text-[13px]">Share</Text>
              </Button>
            </View>
          </>
        )}
      </Card>

      <View className="grid grid-cols-1 md:grid-cols-3 gap-3" style={{ display: "grid" as never }}>
        <Card className="p-5">
          <StatCard label="Total referred" value={String(totalReferees)} isLoading={isLoading} />
        </Card>
        <Card className="p-5">
          <StatCard label="Total earned" value={formatUsd(totalEarned)} isLoading={isLoading} />
        </Card>
        <Card className="p-5">
          <StatCard
            label="Commission rate"
            value={
              isReferrer && settings?.commissionRate
                ? `${(Number(settings.commissionRate) * 100).toFixed(1)}%`
                : "\u2014"
            }
            isLoading={isLoading}
          />
        </Card>
      </View>

      <Card className="overflow-hidden">
        <View className="px-4 py-3">
          <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
            Referred users
          </Text>
        </View>

        <View className="flex flex-row items-center h-8 px-4 border-t border-border-subtle bg-bg-sunk">
          <Text className="w-[180px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
            Address
          </Text>
          <Text className="w-[140px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
            User
          </Text>
          <Text className="w-[140px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
            Volume
          </Text>
          <Text className="flex-1 text-fg-tertiary text-[11px] font-medium uppercase tracking-wide text-right">
            Commission
          </Text>
        </View>

        {refereesLoading ? (
          <View className="flex flex-col gap-2 p-4">
            <Skeleton height={44} />
            <Skeleton height={44} />
          </View>
        ) : referees.length === 0 ? (
          <View className="p-8 items-center">
            <Text className="text-fg-tertiary text-[13px]">No referred users yet</Text>
          </View>
        ) : (
          referees.map((referee) => (
            <View
              key={referee.userIndex}
              className="flex flex-row items-center h-11 px-4 hover:bg-bg-sunk transition-[background] duration-150 ease-[var(--ease)]"
            >
              <Text className="w-[180px] text-fg-secondary text-[13px] font-mono">
                #{referee.userIndex}
              </Text>
              <Text className="w-[140px] text-fg-tertiary text-[13px]">
                user_{referee.userIndex}
              </Text>
              <Text className="w-[140px] text-fg-secondary text-[13px] tabular-nums">
                {formatUsd(Decimal(referee.volume ?? "0"))}
              </Text>
              <Text className="flex-1 text-up text-[13px] font-medium tabular-nums text-right">
                +{formatUsd(Decimal(referee.commissionEarned ?? "0"))}
              </Text>
            </View>
          ))
        )}
      </Card>
    </View>
  );
}
