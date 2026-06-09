import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";
import { usePublicClient } from "./usePublicClient.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { Decimal } from "@left-curve/utils";

import type { PerpsPairStats } from "@left-curve/types";
import type { Config } from "../types/store.js";
import type { LiveResourceSnapshot } from "../live/types.js";

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

export type AllPerpsPairStatsSnapshot = LiveResourceSnapshot & {
  perpsPairStats: NormalizedPerpsPairStats[];
  perpsPairStatsByPairId: Record<string, NormalizedPerpsPairStats>;
};

const ALL_PERPS_PAIR_STATS_HTTP_INTERVAL = 5_000;

type AllPerpsPairStatsResourceParams = {
  chainId: Config["chain"]["id"];
  subscriptions: Config["subscriptions"];
};

const initialAllPerpsPairStatsSnapshot: AllPerpsPairStatsSnapshot = {
  status: "idle",
  error: null,
  perpsPairStats: [],
  perpsPairStatsByPairId: {},
};

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

function buildStatsByPairId(stats: NormalizedPerpsPairStats[]) {
  return Object.fromEntries(stats.map((stats) => [stats.pairId, stats]));
}

function equalNormalizedStats(previous: NormalizedPerpsPairStats, next: NormalizedPerpsPairStats) {
  return (
    previous.pairId === next.pairId &&
    previous.currentPrice === next.currentPrice &&
    previous.price24HAgo === next.price24HAgo &&
    previous.volume24H === next.volume24H &&
    previous.priceChange24H === next.priceChange24H
  );
}

function equalAllPerpsPairStatsSnapshot(
  previous: AllPerpsPairStatsSnapshot,
  next: AllPerpsPairStatsSnapshot,
) {
  if (previous.status !== next.status || previous.error !== next.error) return false;
  if (previous.perpsPairStats.length !== next.perpsPairStats.length) return false;

  for (let index = 0; index < previous.perpsPairStats.length; index += 1) {
    if (!equalNormalizedStats(previous.perpsPairStats[index], next.perpsPairStats[index])) {
      return false;
    }
  }

  return true;
}

const allPerpsPairStatsResource = createLiveResource<
  AllPerpsPairStatsResourceParams,
  AllPerpsPairStatsSnapshot
>({
  name: "allPerpsPairStats",
  cache: "keep",
  getKey: ({ chainId }) => `allPerpsPairStats:${chainId}`,
  getInitialSnapshot: () => initialAllPerpsPairStatsSnapshot,
  equal: equalAllPerpsPairStatsSnapshot,
  start: ({ subscriptions }, { emit, error }) =>
    subscriptions.subscribe("allPerpsPairStats", {
      params: {
        httpInterval: ALL_PERPS_PAIR_STATS_HTTP_INTERVAL,
      },
      listener: ({ allPerpsPairStats }) => {
        const perpsPairStats = allPerpsPairStats.map(normalizePerpsPairStats);
        emit({
          status: "ready",
          error: null,
          perpsPairStats,
          perpsPairStatsByPairId: buildStatsByPairId(perpsPairStats),
        });
      },
      onError: error,
    }),
});

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

export function useAllPerpsPairStats<Selection>(
  selector: (snapshot: AllPerpsPairStatsSnapshot) => Selection,
  parameters: UseAllPerpsPairStatsParameters = {},
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection {
  const { enabled = true } = parameters;
  const config = useConfig();

  return useLiveResource({
    resource: allPerpsPairStatsResource,
    params: {
      chainId: config.chain.id,
      subscriptions: config.subscriptions,
    },
    enabled,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}
