import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { create } from "zustand";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";

import { Decimal, formatUnits } from "@left-curve/dango/utils";

import type { PairStats } from "@left-curve/dango/types";
import type { AnyCoin } from "../types/coin.js";

export type NormalizedPairStats = {
  baseDenom: string;
  quoteDenom: string;
  currentPrice: string | null;
  price24HAgo: string | null;
  volume24H: string;
  priceChange24H: string | null;
};

export type UsePairStatsParameters = {
  baseDenom: string;
  quoteDenom: string;
  enabled?: boolean;
};

export type UseAllPairStatsParameters = {
  enabled?: boolean;
};

const toPairKey = (baseDenom: string, quoteDenom: string) => `${baseDenom}:${quoteDenom}`;

type AllPairStatsStoreState = {
  pairStats: NormalizedPairStats[];
  pairStatsByKey: Record<string, NormalizedPairStats>;
  setPairStats: (pairStats: NormalizedPairStats[]) => void;
};

export const allPairStatsStore = create<AllPairStatsStoreState>((set) => ({
  pairStats: [],
  pairStatsByKey: {},
  setPairStats: (pairStats) =>
    set({
      pairStats,
      pairStatsByKey: Object.fromEntries(
        pairStats.map((stats) => [toPairKey(stats.baseDenom, stats.quoteDenom), stats]),
      ),
    }),
}));

function asDecimal(value: string | null | undefined) {
  if (!value) return null;

  try {
    return Decimal(value);
  } catch {
    return null;
  }
}

function normalizePairStats(
  pairStats: PairStats,
  coinsByDenom: Record<string, AnyCoin>,
): NormalizedPairStats {
  const baseCoin = coinsByDenom[pairStats.baseDenom];
  const quoteCoin = coinsByDenom[pairStats.quoteDenom];

  const decimalsFactor =
    baseCoin && quoteCoin ? Decimal(10).pow(baseCoin.decimals - quoteCoin.decimals) : null;

  const currentPriceRaw = asDecimal(pairStats.currentPrice);
  const price24HAgoRaw = asDecimal(pairStats.price24HAgo);
  const currentPrice =
    currentPriceRaw && decimalsFactor ? currentPriceRaw.mul(decimalsFactor) : null;
  const price24HAgo = price24HAgoRaw && decimalsFactor ? price24HAgoRaw.mul(decimalsFactor) : null;

  const priceChangeFromBackend = asDecimal(pairStats.priceChange24H);
  const priceChangeComputed =
    currentPrice && price24HAgo && !price24HAgo.isZero()
      ? currentPrice.minus(price24HAgo).div(price24HAgo).mul(100)
      : null;

  return {
    baseDenom: pairStats.baseDenom,
    quoteDenom: pairStats.quoteDenom,
    currentPrice: currentPrice?.toString() ?? null,
    price24HAgo: price24HAgo?.toString() ?? null,
    volume24H: quoteCoin
      ? formatUnits(pairStats.volume24H, quoteCoin.decimals)
      : pairStats.volume24H,
    priceChange24H: (priceChangeFromBackend ?? priceChangeComputed)?.toString() ?? null,
  };
}

export function usePairStats(parameters: UsePairStatsParameters) {
  const { baseDenom, quoteDenom, enabled = true } = parameters;
  const client = usePublicClient();
  const { coins } = useConfig();

  return useQuery({
    enabled,
    queryKey: ["pair_stats", baseDenom, quoteDenom],
    queryFn: async () => {
      const pairStats = await client.getPairStats({ baseDenom, quoteDenom });

      if (!pairStats) return null;

      return normalizePairStats(pairStats, coins.byDenom);
    },
  });
}

export function useAllPairStats(parameters: UseAllPairStatsParameters = {}): void {
  const { enabled = true } = parameters;
  const { coins, subscriptions } = useConfig();

  useEffect(() => {
    if (!enabled) return;
    const unsubscribe = subscriptions.subscribe("allPairStats", {
      listener: ({ allPairStats }) =>
        allPairStatsStore
          .getState()
          .setPairStats(allPairStats.map((stats) => normalizePairStats(stats, coins.byDenom))),
    });
    return () => unsubscribe();
  }, [enabled, subscriptions, coins.byDenom]);
}

export { toPairKey };
