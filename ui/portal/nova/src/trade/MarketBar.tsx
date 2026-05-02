import { useMemo } from "react";
import { View, Text } from "react-native";
import { twMerge, useCountdown } from "@left-curve/foundation";
import { Decimal } from "@left-curve/dango/utils";
import {
  useCurrentPrice,
  useTradeCoins,
  allPerpsPairStatsStore,
  perpsPairStateStore,
  TradePairStore,
  perpsStateStore,
  usePerpsParam,
} from "@left-curve/store";
import { Card, FormattedNumber, Skeleton } from "../components";

type KpiItemProps = {
  readonly label: string;
  readonly children: React.ReactNode;
};

function KpiItem({ label, children }: KpiItemProps) {
  return (
    <View className="flex flex-col gap-0.5">
      <Text className="text-[10px] text-fg-tertiary tracking-wide uppercase">{label}</Text>
      {typeof children === "string" ? (
        <Text className="text-[13px] font-medium font-mono tabular-nums text-fg-primary">
          {children}
        </Text>
      ) : (
        children
      )}
    </View>
  );
}

function KpiSkeleton() {
  return (
    <View className="flex flex-col gap-1">
      <Skeleton width={48} height={10} />
      <Skeleton width={72} height={16} />
    </View>
  );
}

export function MarketBar() {
  const { baseCoin, quoteCoin } = useTradeCoins();
  const { currentPrice, previousPrice } = useCurrentPrice();
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);
  const mode = TradePairStore((s) => s.mode);

  const perpsPairId = getPerpsPairId();
  const statsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);
  const pairStats = statsByPairId[perpsPairId];

  const pairState = perpsPairStateStore((s) => s.pairState);
  const perpsState = perpsStateStore((s) => s.state);

  const markPrice = currentPrice ? Decimal(currentPrice) : null;
  const isPositiveChange =
    previousPrice && currentPrice ? Decimal(previousPrice).lte(currentPrice) : true;

  const { change24h, changeDelta } = useMemo(() => {
    if (!pairStats?.priceChange24H) return { change24h: null, changeDelta: null };
    const change = Decimal(pairStats.priceChange24H);
    const delta =
      pairStats.currentPrice && pairStats.price24HAgo
        ? Decimal(pairStats.currentPrice).minus(Decimal(pairStats.price24HAgo))
        : null;
    return { change24h: change, changeDelta: delta };
  }, [pairStats]);

  const volume24h = pairStats?.volume24H ?? null;

  const { totalOiUsd } = useMemo(() => {
    if (!pairState || !currentPrice) return { totalOiUsd: null };
    const price = Decimal(currentPrice);
    const longOi = Decimal(pairState.longOi);
    const shortOi = Decimal(pairState.shortOi);
    return { totalOiUsd: longOi.mul(price).plus(shortOi.mul(price)) };
  }, [pairState, currentPrice]);

  const { data: perpsParam } = usePerpsParam();

  const { fundingPct, isPositiveFunding, countdownEndTime } = useMemo(() => {
    if (!pairState?.fundingRate) {
      return { fundingPct: null, isPositiveFunding: true, countdownEndTime: undefined };
    }
    const rate = Decimal(pairState.fundingRate);
    const endTime =
      perpsState?.lastFundingTime && perpsParam?.fundingPeriod
        ? Number(perpsState.lastFundingTime) * 1000 + perpsParam.fundingPeriod * 1000
        : undefined;
    return {
      fundingPct: rate.mul(100).div(24),
      isPositiveFunding: rate.gte(0),
      countdownEndTime: endTime,
    };
  }, [pairState, perpsState, perpsParam]);

  const countdown = useCountdown({
    date: countdownEndTime,
    showLeadingZeros: true,
  });

  const pairLabel =
    mode === "perps" ? `${baseCoin.symbol}-PERP` : `${baseCoin.symbol}/${quoteCoin.symbol}`;

  return (
    <Card className="flex flex-row items-stretch p-0 overflow-visible relative">
      <View className="flex flex-row items-center gap-2.5 px-3.5 py-2.5 border-r border-border-subtle min-w-[220px] shrink-0">
        <View className="w-8 h-8 rounded-full bg-accent-bg items-center justify-center shrink-0">
          <Text className="text-accent font-bold text-[13px] font-display">
            {baseCoin.symbol[0]}
          </Text>
        </View>
        <View className="flex flex-col gap-0.5 min-w-0">
          <View className="flex flex-row items-center gap-1.5">
            <Text className="text-fg-primary font-semibold text-[14px] tracking-tight whitespace-nowrap">
              {pairLabel}
            </Text>
            <Text className="text-fg-tertiary text-[11px] font-normal">
              {"\u00B7"} {quoteCoin.symbol}
            </Text>
          </View>
          <Text className="text-fg-tertiary text-[11px] whitespace-nowrap">
            {mode === "perps" ? `Perpetual ${"\u00B7"} 100x max` : "Spot"}
          </Text>
        </View>
      </View>

      <View className="flex flex-row items-center gap-7 px-6 flex-1 overflow-auto">
        <View className="flex flex-col gap-0.5 min-w-[130px]">
          <Text className="text-[10px] text-fg-tertiary tracking-wide uppercase">Mark</Text>
          {markPrice ? (
            <FormattedNumber
              value={markPrice.toString()}
              formatOptions={{
                currency: "USD",
                fractionDigits: markPrice.lt(Decimal("10")) ? 4 : 2,
              }}
              colorSign={isPositiveChange !== undefined}
              className={twMerge(
                "text-[16px] font-medium",
                isPositiveChange ? "text-up" : "text-down",
              )}
            />
          ) : (
            <Skeleton width={96} height={20} />
          )}
        </View>

        {change24h ? (
          <KpiItem label="24h Change">
            <View className="flex flex-row items-center gap-1">
              <View className="flex flex-row items-baseline">
                <FormattedNumber
                  value={change24h.toString()}
                  formatOptions={{ fractionDigits: 2 }}
                  sign
                  colorSign
                  className="text-[13px] font-medium"
                />
                <Text
                  className={twMerge(
                    "text-[13px] font-medium font-mono tabular-nums",
                    change24h.gte(Decimal("0")) ? "text-up" : "text-down",
                  )}
                >
                  %
                </Text>
              </View>
              {changeDelta && (
                <FormattedNumber
                  value={changeDelta.toString()}
                  formatOptions={{ fractionDigits: 2 }}
                  sign
                  className="text-fg-tertiary text-[12px] font-normal"
                />
              )}
            </View>
          </KpiItem>
        ) : (
          <KpiSkeleton />
        )}

        {volume24h ? (
          <KpiItem label="24h Volume">
            <FormattedNumber
              value={volume24h}
              formatOptions={{ currency: "USD" }}
              className="text-fg-primary text-[13px] font-medium"
            />
          </KpiItem>
        ) : (
          <KpiSkeleton />
        )}

        {totalOiUsd ? (
          <KpiItem label="Open Interest">
            <FormattedNumber
              value={totalOiUsd.toString()}
              formatOptions={{ currency: "USD", fractionDigits: 0 }}
              className="text-fg-primary text-[13px] font-medium"
            />
          </KpiItem>
        ) : (
          <KpiSkeleton />
        )}

        <View className="flex flex-col gap-0.5">
          <Text className="text-[10px] text-fg-tertiary tracking-wide uppercase">Funding / 1h</Text>
          {fundingPct ? (
            <View className="flex flex-row items-baseline">
              <FormattedNumber
                value={fundingPct.toString()}
                formatOptions={{ fractionDigits: 4 }}
                colorSign
                className={twMerge(
                  "text-[13px] font-medium",
                  isPositiveFunding ? "text-up" : "text-down",
                )}
              />
              <Text
                className={twMerge(
                  "text-[13px] font-medium font-mono tabular-nums",
                  isPositiveFunding ? "text-up" : "text-down",
                )}
              >
                %
              </Text>
            </View>
          ) : (
            <Skeleton width={64} height={16} />
          )}
        </View>

        <KpiItem label="Countdown">
          {countdownEndTime
            ? `${countdown.hours}:${countdown.minutes}:${countdown.seconds}`
            : "--:--:--"}
        </KpiItem>
      </View>
    </Card>
  );
}
