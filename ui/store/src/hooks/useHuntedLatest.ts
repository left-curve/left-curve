import { Decimal } from "@left-curve/utils";
import { useQuery } from "@tanstack/react-query";
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
  /** Override polling interval. Defaults to 60s for the "Live" feel. */
  refetchInterval?: number | false;
};

const DEFAULT_REFETCH_INTERVAL = 60_000;

export function useHuntedLatest(parameters: UseHuntedLatestParameters) {
  const {
    pointsUrl,
    limit,
    enabled = true,
    refetchInterval = DEFAULT_REFETCH_INTERVAL,
  } = parameters;

  const query = useQuery({
    queryKey: ["hunted-latest", limit ?? null],
    queryFn: () => fetchHuntedLatest(pointsUrl, { limit }),
    enabled,
    refetchInterval,
  });

  return {
    entries: query.data ?? ([] as HuntedLatestEntry[]),
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    isError: query.isError,
  };
}

export type UseHuntedMultipliersParameters = {
  pointsUrl: string;
  enabled?: boolean;
};

/**
 * Reads `/config -> boost_config.hunted` and exposes a resolver that returns
 * the multiplier (as `Decimal`) for a given loot variant and epoch. Keys in
 * the config map are epoch ranges using the formats `"N"`, `"N-M"` or `"N-"`.
 */
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
