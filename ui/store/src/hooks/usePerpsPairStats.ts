import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { create } from "zustand";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";

import { Decimal } from "@left-curve/dango/utils";

import type { PerpsPairStats } from "@left-curve/dango/types";

export type NormalizedPerpsPairStats = {
  pairId: string;
  currentPrice: string | null;
  price24HAgo: string | null;
  volume24H: string;
  priceChange24H: string | null;
};

export type UsePerpsPairStatsParameters = {
  pairId: string;
  enabled?: boolean;
};

export type UseAllPerpsPairStatsParameters = {
  enabled?: boolean;
};

type AllPerpsPairStatsStoreState = {
  perpsPairStats: NormalizedPerpsPairStats[];
  perpsPairStatsByPairId: Record<string, NormalizedPerpsPairStats>;
  setPerpsPairStats: (stats: NormalizedPerpsPairStats[]) => void;
};

export const allPerpsPairStatsStore = create<AllPerpsPairStatsStoreState>((set) => ({
  perpsPairStats: [],
  perpsPairStatsByPairId: {},
  setPerpsPairStats: (perpsPairStats) =>
    set({
      perpsPairStats,
      perpsPairStatsByPairId: Object.fromEntries(
        perpsPairStats.map((stats) => [stats.pairId, stats]),
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

function normalizePerpsPairStats(stats: PerpsPairStats): NormalizedPerpsPairStats {
  const currentPrice = asDecimal(stats.currentPrice);
  const price24HAgo = asDecimal(stats.price24HAgo);

  const priceChangeFromBackend = asDecimal(stats.priceChange24H);
  const priceChangeComputed =
    currentPrice && price24HAgo && !price24HAgo.isZero()
      ? currentPrice.minus(price24HAgo).div(price24HAgo).mul(100)
      : null;

  return {
    pairId: stats.pairId,
    currentPrice: currentPrice?.toString() ?? null,
    price24HAgo: price24HAgo?.toString() ?? null,
    volume24H: stats.volume24H,
    priceChange24H: (priceChangeFromBackend ?? priceChangeComputed)?.toString() ?? null,
  };
}

export function usePerpsPairStats(parameters: UsePerpsPairStatsParameters) {
  const { pairId, enabled = true } = parameters;
  const client = usePublicClient();

  return useQuery({
    enabled: enabled && !!pairId,
    queryKey: ["perps_pair_stats", pairId],
    queryFn: async () => {
      const stats = await client.getPerpsPairStats({ pairId });

      if (!stats) return null;

      return normalizePerpsPairStats(stats);
    },
  });
}

export function useAllPerpsPairStats(parameters: UseAllPerpsPairStatsParameters = {}): void {
  const { enabled = true } = parameters;
  const { subscriptions } = useConfig();

  useEffect(() => {
    if (!enabled) return;
    const unsubscribe = subscriptions.subscribe("allPerpsPairStats", {
      listener: ({ allPerpsPairStats }) =>
        allPerpsPairStatsStore
          .getState()
          .setPerpsPairStats(allPerpsPairStats.map(normalizePerpsPairStats)),
    });
    return () => unsubscribe();
  }, [enabled, subscriptions]);
}
