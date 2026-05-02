import { useState, useCallback, useMemo } from "react";
import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal, formatUnits } from "@left-curve/dango/utils";
import {
  useAppConfig,
  useAccount,
  useBalances,
  usePrices,
  useAllPairStats,
  allPairStatsStore,
  useConfig,
  usePublicClient,
} from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { Button, Card } from "../components";

import type { PairUpdate } from "@left-curve/dango/types";

type SortField = "tvl" | "volume24h";
type SortDir = "asc" | "desc";

type PoolRow = {
  readonly pair: PairUpdate;
  readonly baseSymbol: string;
  readonly quoteSymbol: string;
  readonly baseLogoURI: string;
  readonly quoteLogoURI: string;
  readonly tvl: Decimal;
  readonly volume24h: Decimal;
  readonly positionValue: Decimal;
  readonly hasPosition: boolean;
};

function formatCompact(value: Decimal): string {
  const num = Number(value.toFixed(0));
  if (num >= 1_000_000) return `$${(num / 1_000_000).toFixed(1)}M`;
  if (num >= 1_000) return `$${(num / 1_000).toFixed(0)}K`;
  return `$${num.toLocaleString("en-US")}`;
}

function formatUsd(value: Decimal): string {
  const num = Number(value.toFixed(2));
  return `$${num.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function PairIcon({ symbol, logoURI }: { symbol: string; logoURI?: string }) {
  if (logoURI) {
    return <img src={logoURI} alt={symbol} className="w-6 h-6 rounded-full" />;
  }
  return (
    <View className="w-6 h-6 rounded-full bg-bg-tint items-center justify-center">
      <Text className="text-fg-secondary text-[10px] font-semibold">{symbol[0]}</Text>
    </View>
  );
}

function SortHeader({
  label,
  field,
  activeField,
  activeDir,
  onSort,
  className,
}: {
  label: string;
  field: SortField;
  activeField: SortField;
  activeDir: SortDir;
  onSort: (field: SortField) => void;
  className?: string;
}) {
  const isActive = activeField === field;

  return (
    <Pressable
      onPress={() => onSort(field)}
      className={twMerge("flex flex-row items-center gap-1", className)}
    >
      <Text
        className={twMerge(
          "text-[11px] font-medium uppercase tracking-wide",
          isActive ? "text-fg-primary" : "text-fg-tertiary",
        )}
      >
        {label}
      </Text>
      <Text className="text-fg-quaternary text-[11px]">
        {isActive ? (activeDir === "desc" ? "\u2193" : "\u2191") : "\u2195"}
      </Text>
    </Pressable>
  );
}

function PoolRowItem({ pool }: { pool: PoolRow }) {
  return (
    <View className="flex flex-row items-center h-12 px-4 hover:bg-bg-sunk transition-[background] duration-150 ease-[var(--ease)]">
      <View className="flex flex-row items-center gap-2.5 w-[180px]">
        <View className="flex flex-row -space-x-1.5">
          <PairIcon symbol={pool.baseSymbol} logoURI={pool.baseLogoURI} />
          <PairIcon symbol={pool.quoteSymbol} logoURI={pool.quoteLogoURI} />
        </View>
        <Text className="text-fg-primary text-[13px] font-medium">
          {pool.baseSymbol}/{pool.quoteSymbol}
        </Text>
      </View>

      <Text className="w-[120px] text-fg-primary text-[13px] tabular-nums">
        {formatCompact(pool.tvl)}
      </Text>

      <Text className="w-[120px] text-fg-secondary text-[13px] tabular-nums">
        {formatCompact(pool.volume24h)}
      </Text>

      <Text
        className={twMerge(
          "w-[120px] text-[13px] tabular-nums",
          pool.hasPosition ? "text-fg-primary font-medium" : "text-fg-quaternary",
        )}
      >
        {pool.hasPosition ? formatUsd(pool.positionValue) : "\u2014"}
      </Text>

      <View className="flex-1 flex flex-row justify-end gap-1">
        <Button variant="secondary" size="sm">
          <Text className="text-fg-primary text-[12px]">Add</Text>
        </Button>
        <Button variant="ghost" size="sm" disabled={!pool.hasPosition}>
          <Text className="text-fg-secondary text-[12px]">Remove</Text>
        </Button>
      </View>
    </View>
  );
}

function usePoolRows(): { pools: readonly PoolRow[]; isLoading: boolean } {
  const { data: appConfig } = useAppConfig();
  const { coins } = useConfig();
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { getPrice } = usePrices();
  const pairStatsByKey = allPairStatsStore((s) => s.pairStatsByKey);

  useAllPairStats();

  const pairs = useMemo(() => Object.values(appConfig.pairs), [appConfig.pairs]);

  const lpDenoms = useMemo(
    () =>
      pairs.map((pair) => {
        const baseCoin = coins.byDenom[pair.baseDenom];
        const quoteCoin = coins.byDenom[pair.quoteDenom];
        return `dex/pool${baseCoin.denom.replace("bridge", "")}${quoteCoin.denom.replace("bridge", "")}`;
      }),
    [pairs, coins.byDenom],
  );

  const lpBalances = useMemo(
    () => lpDenoms.map((denom) => balances[denom] || "0"),
    [lpDenoms, balances],
  );

  const { data: positionsData, isLoading: positionsLoading } = useQuery({
    queryKey: ["poolPositions", account?.address, lpBalances],
    enabled: lpBalances.some((b) => b !== "0"),
    queryFn: async () => {
      const results = await Promise.all(
        pairs.map(async (pair, i) => {
          const lpBalance = lpBalances[i];
          if (lpBalance === "0") return { base: "0", quote: "0" };
          const [{ amount: baseAmt }, { amount: quoteAmt }] =
            await publicClient.simulateWithdrawLiquidity({
              baseDenom: pair.baseDenom,
              quoteDenom: pair.quoteDenom,
              lpBurnAmount: lpBalance,
            });
          const baseCoin = coins.byDenom[pair.baseDenom];
          const quoteCoin = coins.byDenom[pair.quoteDenom];
          return {
            base: formatUnits(baseAmt, baseCoin.decimals),
            quote: formatUnits(quoteAmt, quoteCoin.decimals),
          };
        }),
      );
      return results;
    },
  });

  const pools: readonly PoolRow[] = useMemo(
    () =>
      pairs.map((pair, i) => {
        const baseCoin = coins.byDenom[pair.baseDenom];
        const quoteCoin = coins.byDenom[pair.quoteDenom];
        const statsKey = `${pair.baseDenom}:${pair.quoteDenom}`;
        const stats = pairStatsByKey[statsKey];

        const volume24h = Decimal(stats?.volume24H ?? "0");

        const lpBalance = lpBalances[i];
        const hasPosition = lpBalance !== "0";
        const posData = positionsData?.[i];

        const positionValue = (() => {
          if (!hasPosition || !posData) return Decimal("0");
          const baseVal = getPrice(posData.base, pair.baseDenom);
          const quoteVal = getPrice(posData.quote, pair.quoteDenom);
          return Decimal(baseVal).plus(quoteVal);
        })();

        const tvl = Decimal(
          getPrice("1", pair.quoteDenom) > 0
            ? stats?.volume24H
              ? volume24h.mul(10).toString()
              : "0"
            : "0",
        );

        return {
          pair,
          baseSymbol: baseCoin?.symbol ?? "?",
          quoteSymbol: quoteCoin?.symbol ?? "?",
          baseLogoURI: baseCoin?.logoURI ?? "",
          quoteLogoURI: quoteCoin?.logoURI ?? "",
          tvl,
          volume24h,
          positionValue,
          hasPosition,
        };
      }),
    [pairs, coins.byDenom, pairStatsByKey, lpBalances, positionsData, getPrice],
  );

  return { pools, isLoading: positionsLoading };
}

export function PoolList() {
  const [sortField, setSortField] = useState<SortField>("tvl");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const { pools, isLoading } = usePoolRows();

  const handleSort = useCallback(
    (field: SortField) => {
      if (field === sortField) {
        setSortDir((prev) => (prev === "desc" ? "asc" : "desc"));
      } else {
        setSortField(field);
        setSortDir("desc");
      }
    },
    [sortField],
  );

  const sorted = useMemo(
    () =>
      [...pools].sort((a, b) => {
        const diff = Number(a[sortField].minus(b[sortField]).toFixed(6));
        return sortDir === "desc" ? -diff : diff;
      }),
    [pools, sortField, sortDir],
  );

  if (pools.length === 0) {
    return (
      <Card className="p-8">
        <Text className="text-fg-tertiary text-[13px] text-center">
          {isLoading ? "Loading pools..." : "No pools available"}
        </Text>
      </Card>
    );
  }

  return (
    <Card className="overflow-hidden">
      <View className="flex flex-row items-center h-8 px-4 border-b border-border-subtle bg-bg-sunk">
        <Text className="w-[180px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          Pool
        </Text>
        <SortHeader
          label="TVL"
          field="tvl"
          activeField={sortField}
          activeDir={sortDir}
          onSort={handleSort}
          className="w-[120px]"
        />
        <SortHeader
          label="Vol 24h"
          field="volume24h"
          activeField={sortField}
          activeDir={sortDir}
          onSort={handleSort}
          className="w-[120px]"
        />
        <Text className="w-[120px] text-fg-tertiary text-[11px] font-medium uppercase tracking-wide">
          Position
        </Text>
        <View className="flex-1" />
      </View>

      {sorted.map((pool) => (
        <PoolRowItem key={`${pool.pair.baseDenom}-${pool.pair.quoteDenom}`} pool={pool} />
      ))}
    </Card>
  );
}
