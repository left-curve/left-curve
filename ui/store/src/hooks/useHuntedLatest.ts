import { Decimal } from "@left-curve/utils";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useCallback } from "react";

import {
  type HuntedLatestEntry,
  type HuntedLoot,
  fetchHuntedLatest,
  fetchPointsConfig,
} from "./pointsApi.js";

export type UseHuntedLatestParameters = {
  pointsUrl: string;
  limit?: number;
  enabled?: boolean;
  refetchInterval?: number | false;
};

const DEFAULT_REFETCH_INTERVAL = 60_000;
const DEFAULT_PAGE_SIZE = 20;

export function useHuntedLatest(parameters: UseHuntedLatestParameters) {
  const {
    pointsUrl,
    limit = DEFAULT_PAGE_SIZE,
    enabled = true,
    refetchInterval = DEFAULT_REFETCH_INTERVAL,
  } = parameters;

  const query = useInfiniteQuery({
    queryKey: ["hunted-latest", limit],
    queryFn: ({ pageParam }) => fetchHuntedLatest(pointsUrl, { limit, start_after: pageParam }),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (lastPage) =>
      lastPage.length === limit ? lastPage[lastPage.length - 1].block_height : undefined,
    enabled,
    refetchInterval,
  });

  return {
    pages: (query.data?.pages ?? []) as HuntedLatestEntry[][],
    hasNextPage: query.hasNextPage,
    fetchNextPage: query.fetchNextPage,
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    isFetchingNextPage: query.isFetchingNextPage,
    isError: query.isError,
  };
}

export type UseHuntedMultipliersParameters = {
  pointsUrl: string;
  enabled?: boolean;
};

export function useHuntedMultipliers(parameters: UseHuntedMultipliersParameters) {
  const { pointsUrl, enabled = true } = parameters;

  const query = useQuery({
    queryKey: ["points-config"],
    queryFn: () => fetchPointsConfig(pointsUrl),
    enabled,
    staleTime: 5 * 60_000,
  });

  const huntedMap = query.data?.boost_config?.hunted;

  const resolveMultiplier = useCallback(
    (loot: HuntedLoot, epoch: number): InstanceType<typeof Decimal> | null => {
      const ranges = huntedMap?.[loot];
      if (!ranges) return null;
      for (const [rangeKey, value] of Object.entries(ranges)) {
        if (epochInRange(epoch, rangeKey)) {
          try {
            return Decimal(value);
          } catch {
            return null;
          }
        }
      }
      return null;
    },
    [huntedMap],
  );

  return {
    resolveMultiplier,
    isLoading: query.isLoading,
  };
}

function epochInRange(epoch: number, rangeKey: string): boolean {
  const trimmed = rangeKey.trim();
  if (trimmed === "") return false;

  if (trimmed.includes("-")) {
    const [startRaw, endRaw] = trimmed.split("-", 2);
    const start = Number.parseInt(startRaw, 10);
    if (!Number.isFinite(start)) return false;
    if (endRaw === "" || endRaw === undefined) return epoch >= start;
    const end = Number.parseInt(endRaw, 10);
    if (!Number.isFinite(end)) return false;
    return epoch >= start && epoch <= end;
  }

  const single = Number.parseInt(trimmed, 10);
  return Number.isFinite(single) && epoch === single;
}
