import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Decimal } from "@left-curve/dango/utils";
import { usePublicClient } from "./usePublicClient.js";

import type { VaultSnapshot } from "@left-curve/dango/types";

export type VaultPerformancePoint = {
  date: string;
  timestamp: number;
  sharePrice: number;
  dailyChange: number;
};

export type VaultPerformancePeriod = "7D" | "30D" | "90D";

const PERIOD_DAYS: Record<VaultPerformancePeriod, number> = {
  "7D": 7,
  "30D": 30,
  "90D": 90,
};

function snapshotsToPerformance(
  snapshots: Record<string, VaultSnapshot>,
): VaultPerformancePoint[] {
  const entries = Object.entries(snapshots).sort(
    ([a], [b]) => Number(a) - Number(b),
  );

  return entries.map(([ts, snapshot], index) => {
    const sharePrice = Decimal(snapshot.equity).div(snapshot.share_supply).toNumber();
    let dailyChange = 0;

    if (index > 0) {
      const prevSnapshot = entries[index - 1][1];
      const prevPrice = Decimal(prevSnapshot.equity).div(prevSnapshot.share_supply).toNumber();
      dailyChange = prevPrice > 0 ? ((sharePrice - prevPrice) / prevPrice) * 100 : 0;
    }

    // Timestamp is in "seconds.nanoseconds" format (e.g. "1732770602.144737024")
    const dateMs = Math.floor(Number(ts) * 1000);

    return {
      date: new Date(dateMs).toISOString(),
      timestamp: dateMs,
      sharePrice,
      dailyChange,
    };
  });
}

export type UseVaultSnapshotsParameters = {
  period?: VaultPerformancePeriod;
  enabled?: boolean;
};

export function useVaultSnapshots(parameters: UseVaultSnapshotsParameters = {}) {
  const { period = "30D", enabled = true } = parameters;
  const client = usePublicClient();

  const days = PERIOD_DAYS[period];
  const min = useMemo(() => {
    return Math.floor((Date.now() - days * 24 * 60 * 60 * 1000) / 1000);
  }, [days]);

  return useQuery({
    queryKey: ["vault-snapshots", period],
    queryFn: async () => {
      const snapshots = await client.getVaultSnapshots({ min });
      return snapshotsToPerformance(snapshots);
    },
    enabled,
    staleTime: 60_000,
    retry: 2,
  });
}
