import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { fetchPointsConfig } from "./pointsApi.js";

export type UseBoostedPairsParameters = {
  pointsUrl: string;
  /** Current event epoch from `useCurrentEpoch`. When `null`, no pairs are
   * considered boosted (event not started yet). */
  currentEpoch: number | null;
  enabled?: boolean;
};

/** Parse an epoch-range key as produced by the bot's `EpochRanges` serializer:
 * - `"N"`     → single epoch `[N, N]`
 * - `"N-M"`   → inclusive range `[N, M]`
 * - `"N-"`    → open-ended `[N, +∞]`
 * Returns `null` for malformed keys (defensive — backend is the source of truth). */
function parseRangeKey(key: string): [number, number] | null {
  const trimmed = key.trim();
  const dashIdx = trimmed.indexOf("-");

  if (dashIdx === -1) {
    const n = Number(trimmed);
    return Number.isFinite(n) ? [n, n] : null;
  }

  const fromStr = trimmed.slice(0, dashIdx).trim();
  const toStr = trimmed.slice(dashIdx + 1).trim();
  const from = Number(fromStr);
  if (!Number.isFinite(from)) return null;

  if (toStr === "") return [from, Number.MAX_SAFE_INTEGER];
  const to = Number(toStr);
  if (!Number.isFinite(to)) return null;
  return [from, to];
}

export function useBoostedPairs(parameters: UseBoostedPairsParameters) {
  const { pointsUrl, currentEpoch, enabled = true } = parameters;

  const query = useQuery({
    queryKey: ["pointsConfig"],
    queryFn: () => fetchPointsConfig(pointsUrl),
    enabled,
    staleTime: 60_000,
  });

  const boostByPairId = useMemo<Record<string, string>>(() => {
    if (currentEpoch === null) return {};
    const pairMap = query.data?.boost_config.pair;
    if (!pairMap) return {};

    const out: Record<string, string> = {};
    for (const [pairId, ranges] of Object.entries(pairMap)) {
      for (const [rangeKey, multiplier] of Object.entries(ranges)) {
        const range = parseRangeKey(rangeKey);
        if (!range) continue;
        const [from, to] = range;
        if (currentEpoch < from || currentEpoch > to) continue;
        // Skip multipliers that are <= 1 (no boost). `Udec128_6` is a decimal
        // string like "1.000000" / "2.500000".
        if (Number(multiplier) <= 1) continue;
        out[pairId] = multiplier;
        break;
      }
    }
    return out;
  }, [query.data, currentEpoch]);

  return {
    boostByPairId,
    isLoading: query.isLoading,
  };
}
