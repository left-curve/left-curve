import { useConfig } from "./useConfig.js";
import { createLiveResource } from "../live/createLiveResource.js";
import { useLiveResource } from "../live/useLiveResource.js";

import { Decimal, shallowEqual } from "@left-curve/utils";

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

export type UseAllPerpsPairStatsParameters = {
  enabled?: boolean;
};

export type UsePerpsPairStatsByPairIdParameters = {
  pairId: string;
  enabled?: boolean;
};

export type AllPerpsPairStatsSnapshot = LiveResourceSnapshot & {
  perpsPairStats: NormalizedPerpsPairStats[];
  perpsPairStatsByPairId: Record<string, NormalizedPerpsPairStats>;
};

const ALL_PERPS_PAIR_STATS_HTTP_INTERVAL = 5_000;

type AllPerpsPairStatsResourceParams = {
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

function normalizeAllPerpsPairStats(
  allPerpsPairStats: PerpsPairStats[],
  previousByPairId: Record<string, NormalizedPerpsPairStats>,
) {
  const perpsPairStats = allPerpsPairStats.map((stats) => {
    const next = normalizePerpsPairStats(stats);
    const previous = previousByPairId[next.pairId];
    return shallowEqual(previous, next) ? previous : next;
  });

  return {
    perpsPairStats,
    perpsPairStatsByPairId: buildStatsByPairId(perpsPairStats),
  };
}

function equalAllPerpsPairStatsSnapshot(
  previous: AllPerpsPairStatsSnapshot,
  next: AllPerpsPairStatsSnapshot,
) {
  if (previous.status !== next.status || previous.error !== next.error) return false;
  if (previous.perpsPairStats.length !== next.perpsPairStats.length) return false;

  for (let index = 0; index < previous.perpsPairStats.length; index += 1) {
    if (!shallowEqual(previous.perpsPairStats[index], next.perpsPairStats[index])) {
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
  getKey: () => "allPerpsPairStats",
  getInitialSnapshot: () => initialAllPerpsPairStatsSnapshot,
  equal: equalAllPerpsPairStatsSnapshot,
  start: ({ subscriptions }, { emit, error }) => {
    let previousByPairId: Record<string, NormalizedPerpsPairStats> = {};

    return subscriptions.subscribe("allPerpsPairStats", {
      params: {
        httpInterval: ALL_PERPS_PAIR_STATS_HTTP_INTERVAL,
      },
      listener: ({ allPerpsPairStats }) => {
        const nextStats = normalizeAllPerpsPairStats(allPerpsPairStats, previousByPairId);
        previousByPairId = nextStats.perpsPairStatsByPairId;

        emit({
          status: "ready",
          error: null,
          ...nextStats,
        });
      },
      onError: error,
    });
  },
});

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
      subscriptions: config.subscriptions,
    },
    enabled,
    selector,
    equalityFn,
    restartToken: config.subscriptions,
  });
}

export function usePerpsPairStatsByPairId(
  parameters: UsePerpsPairStatsByPairIdParameters,
): NormalizedPerpsPairStats | null {
  const { pairId, enabled = true } = parameters;

  return useAllPerpsPairStats(
    (snapshot) => snapshot.perpsPairStatsByPairId[pairId] ?? null,
    { enabled },
    shallowEqual,
  );
}
