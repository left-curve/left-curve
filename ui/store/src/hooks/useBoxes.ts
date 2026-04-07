import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { type BoxesResponse, fetchUserBoxes } from "./pointsApi.js";

export type UseBoxesParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

type NFTRarity = "common" | "uncommon" | "rare" | "epic" | "legendary" | "mythic";

const RARITY_ORDER: NFTRarity[] = ["common", "uncommon", "rare", "epic", "legendary", "mythic"];

export type NFTItem = {
  rarity: NFTRarity;
  quantity: number;
  imageSrc: string;
  frameSrc: string;
};

export function useBoxes(parameters: UseBoxesParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const { data: boxesData = {}, isLoading } = useQuery<BoxesResponse>({
    queryKey: ["boxes", userIndex],
    queryFn: () => fetchUserBoxes(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
  });

  const nfts = useMemo((): NFTItem[] => {
    const counts: Record<string, number> = {};
    for (const loots of Object.values(boxesData)) {
      for (const [loot, info] of Object.entries(loots)) {
        counts[loot] = (counts[loot] ?? 0) + info.opened;
      }
    }
    return RARITY_ORDER.map((rarity) => ({
      rarity,
      quantity: counts[rarity] ?? 0,
      imageSrc: `/images/points/nft/${rarity}.png`,
      frameSrc: `/images/points/nft/frame-${rarity}.png`,
    }));
  }, [boxesData]);

  const unopenedBoxes = useMemo((): Record<string, Record<string, number>> => {
    const result: Record<string, Record<string, number>> = {};
    for (const [chest, loots] of Object.entries(boxesData)) {
      const remaining: Record<string, number> = {};
      for (const [loot, info] of Object.entries(loots)) {
        const rem = info.total - info.opened;
        if (rem > 0) remaining[loot] = rem;
      }
      if (Object.keys(remaining).length > 0) result[chest] = remaining;
    }
    return result;
  }, [boxesData]);

  const unopenedCounts = useMemo((): Record<string, number> => {
    const counts: Record<string, number> = {};
    for (const [chest, loots] of Object.entries(unopenedBoxes)) {
      counts[chest] = Object.values(loots).reduce((sum, n) => sum + n, 0);
    }
    return counts;
  }, [unopenedBoxes]);

  const estimatedVolume = useMemo(() => {
    const thresholds: Record<string, number> = {
      bronze: 25_000,
      silver: 100_000,
      gold: 250_000,
      crystal: 500_000,
    };
    let volume = 0;
    for (const [chest, loots] of Object.entries(boxesData)) {
      const totalBoxes = Object.values(loots).reduce((sum, info) => sum + info.total, 0);
      volume += totalBoxes * (thresholds[chest] ?? 0);
    }
    return volume;
  }, [boxesData]);

  return { boxesData, nfts, unopenedBoxes, unopenedCounts, estimatedVolume, isLoading };
}
