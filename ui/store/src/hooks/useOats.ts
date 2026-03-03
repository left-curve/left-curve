import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { fetchUserOats } from "./pointsApi.js";

type OATType = "supporter" | "wizard" | "trader" | "hurrah";

const CAMPAIGN_MAP: Record<number, OATType> = {
  1: "supporter",
  2: "wizard",
  3: "trader",
  4: "hurrah",
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

  const { data: registeredOats = [], isLoading } = useQuery({
    queryKey: ["oats", userIndex],
    queryFn: () => fetchUserOats(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
  });

  const registeredCampaigns = useMemo(
    () => new Set(registeredOats.map((o) => o.collection_id)),
    [registeredOats],
  );

  const oatStatuses = useMemo(
    (): OATStatus[] =>
      Object.entries(CAMPAIGN_MAP).map(([campaignId, oatType]) => ({
        type: oatType,
        isLocked: !registeredCampaigns.has(Number(campaignId)),
      })),
    [registeredCampaigns],
  );

  return { oatStatuses, registeredOats, isLoading, oatCount: registeredOats.length };
}
