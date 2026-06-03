import { useQuery } from "@tanstack/react-query";

import {
  type BoostersResponse,
  type HuntedLoot,
  type OatEntry,
  fetchBoosters,
} from "./pointsApi.js";

export type UseBoostersParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

const HUNTED_RANK: Record<HuntedLoot, 0 | 1 | 2 | 3> = {
  bronze_shell: 0,
  silver_shell: 1,
  golden_shell: 2,
  pearl_dango: 3,
};

export type HuntedBooster = {
  loot: HuntedLoot;
  epoch: number;
  multiplier: string;
  rank: 0 | 1 | 2 | 3;
};

export type UseBoostersReturnType = {
  oats: OatEntry[];
  huntedBoosters: HuntedBooster[];
  isLoading: boolean;
};

const EMPTY_RETURN: UseBoostersReturnType = {
  oats: [],
  huntedBoosters: [],
  isLoading: false,
};

export function useBoosters(parameters: UseBoostersParameters): UseBoostersReturnType {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const { data, isLoading } = useQuery<BoostersResponse, Error, UseBoostersReturnType>({
    queryKey: ["boosters", userIndex],
    queryFn: () => fetchBoosters(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
    select: (raw): UseBoostersReturnType => ({
      oats: raw.oats,
      huntedBoosters: raw.hunted_loots
        .map((row) => ({
          loot: row.loot,
          epoch: row.epoch,
          multiplier: row.multiplier,
          rank: HUNTED_RANK[row.loot],
        }))
        // Newest epoch first, then highest tier first within an epoch.
        .sort((a, b) => b.epoch - a.epoch || b.rank - a.rank),
      isLoading: false,
    }),
  });

  return data ? { ...data, isLoading } : { ...EMPTY_RETURN, isLoading };
}
