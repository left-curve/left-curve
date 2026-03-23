import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { fetchCampaigns, fetchUserOats } from "./pointsApi.js";

type OATType = "supporter" | "wizard" | "trader" | "hurrah";

const FALLBACK_CAMPAIGN_MAP: Record<number, OATType> = {
  1: "supporter",
  2: "wizard",
  3: "trader",
  4: "hurrah",
};

const campaignNameToOatType = (name: string): OATType | null => {
  const lower = name.toLowerCase();
  if (lower.includes("supporter")) return "supporter";
  if (lower.includes("wizard")) return "wizard";
  if (lower.includes("trader")) return "trader";
  if (lower.includes("hurrah")) return "hurrah";
  return null;
};

export type OATStatus = {
  type: OATType;
  isLocked: boolean;
};

export type UseOatsParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

export function useOats(parameters: UseOatsParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const { data: registeredOats = [], isLoading: isLoadingOats } = useQuery({
    queryKey: ["oats", userIndex],
    queryFn: () => fetchUserOats(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
  });

  const { data: campaigns, isLoading: isLoadingCampaigns } = useQuery({
    queryKey: ["campaigns"],
    queryFn: () => fetchCampaigns(pointsUrl),
    enabled,
  });

  const campaignMap = useMemo((): Record<number, OATType> => {
    if (!campaigns) return FALLBACK_CAMPAIGN_MAP;
    const map: Record<number, OATType> = {};
    for (const [name, id] of campaigns) {
      const oatType = campaignNameToOatType(name);
      if (oatType) map[id] = oatType;
    }
    return Object.keys(map).length > 0 ? map : FALLBACK_CAMPAIGN_MAP;
  }, [campaigns]);

  const registeredCampaigns = useMemo(
    () => new Set(registeredOats.map((o) => o.collection_id)),
    [registeredOats],
  );

  const oatStatuses = useMemo(
    (): OATStatus[] =>
      Object.entries(campaignMap).map(([campaignId, oatType]) => ({
        type: oatType,
        isLocked: !registeredCampaigns.has(Number(campaignId)),
      })),
    [campaignMap, registeredCampaigns],
  );

  const isLoading = isLoadingOats || isLoadingCampaigns;

  return { oatStatuses, registeredOats, isLoading, oatCount: registeredOats.length };
}
