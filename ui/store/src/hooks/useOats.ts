import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { fetchCampaigns, fetchUserOats } from "./pointsApi.js";

type OATType = "supporter" | "wizard" | "trader" | "hurrah";

const OAT_ORDER: OATType[] = ["supporter", "wizard", "trader", "hurrah"];

const FALLBACK_CAMPAIGN_MAP: Record<number, OATType> = {
  1: "supporter",
  2: "wizard",
  3: "trader",
  4: "hurrah",
};

/** Points boost percentage per OAT */
const OAT_POINTS_BOOST = 100;

/** OAT validity duration in seconds (4 weeks, aligned with backend oat_window) */
const OAT_VALIDITY_DURATION_SECONDS = 28 * 24 * 60 * 60;

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
  /** Unix timestamp (seconds) when this OAT expires, undefined if not registered */
  expiresAt?: number;
  /** Points boost percentage for this OAT */
  pointsBoost: number;
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

  const registeredOatsByCampaign = useMemo(() => {
    const map = new Map<number, { registeredAt: number }>();
    for (const oat of registeredOats) {
      map.set(oat.collection_id, { registeredAt: Number.parseFloat(String(oat.registered_at)) });
    }
    return map;
  }, [registeredOats]);

  const oatStatuses = useMemo((): OATStatus[] => {
    const statuses = Object.entries(campaignMap).map(([campaignId, oatType]) => {
      const registered = registeredOatsByCampaign.get(Number(campaignId));
      return {
        type: oatType,
        isLocked: !registered,
        expiresAt: registered ? registered.registeredAt + OAT_VALIDITY_DURATION_SECONDS : undefined,
        pointsBoost: OAT_POINTS_BOOST,
      };
    });
    return statuses.sort((a, b) => OAT_ORDER.indexOf(a.type) - OAT_ORDER.indexOf(b.type));
  }, [campaignMap, registeredOatsByCampaign]);

  const isLoading = isLoadingOats || isLoadingCampaigns;

  return { oatStatuses, registeredOats, isLoading, oatCount: registeredOats.length };
}
