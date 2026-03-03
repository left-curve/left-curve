import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { type BoxReward, fetchUserBoxes } from "./pointsApi.js";

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
};

export function useBoxes(parameters: UseBoxesParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const { data: boxes = [], isLoading } = useQuery({
    queryKey: ["boxes", userIndex],
    queryFn: () => fetchUserBoxes(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
  });

  const nfts = useMemo((): NFTItem[] => {
    const counts: Record<string, number> = {};
    for (const box of boxes) {
      if (box.opened) {
        const rarity = box.loot.toLowerCase();
        counts[rarity] = (counts[rarity] ?? 0) + 1;
      }
    }
    return RARITY_ORDER.map((rarity) => ({
      rarity,
      quantity: counts[rarity] ?? 0,
      imageSrc: `/images/points/nft/${rarity}.png`,
    }));
  }, [boxes]);

  const unopenedBoxes = useMemo(() => {
    const grouped: Record<string, BoxReward[]> = {};
    for (const box of boxes) {
      if (!box.opened) {
        const chest = box.chest.toLowerCase();
        grouped[chest] = [...(grouped[chest] ?? []), box];
      }
    }
    return grouped;
  }, [boxes]);

  const estimatedVolume = useMemo(() => {
    const thresholds: Record<string, number> = {
      bronze: 25_000,
      silver: 100_000,
      gold: 250_000,
      crystal: 500_000,
    };
    let volume = 0;
    for (const box of boxes) {
      const chest = box.chest.toLowerCase();
      volume += thresholds[chest] ?? 0;
    }
    return volume;
  }, [boxes]);

  return { boxes, nfts, unopenedBoxes, estimatedVolume, isLoading };
}
