import { useQuery } from "@tanstack/react-query";

import { fetchEpochPoints, type EpochUserStats } from "./pointsApi.js";

export type UseEpochPointsParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  min?: number;
  max?: number;
  enabled?: boolean;
};

export function useEpochPoints(parameters: UseEpochPointsParameters) {
  const { pointsUrl, userIndex, min, max, enabled = true } = parameters;

  const { data: epochPoints, isLoading } = useQuery<[number, EpochUserStats][]>({
    queryKey: ["epochPoints", userIndex, min, max],
    queryFn: () => fetchEpochPoints(pointsUrl, userIndex!, { min, max }),
    enabled: enabled && !!userIndex,
  });

  return { epochPoints, isLoading };
}
