import { useQuery } from "@tanstack/react-query";

import { fetchWeeklyPoints, type WeeklyPointsResponse } from "./pointsApi.js";

export type UseWeeklyPointsParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  min?: number;
  max?: number;
  enabled?: boolean;
};

export function useWeeklyPoints(parameters: UseWeeklyPointsParameters) {
  const { pointsUrl, userIndex, min, max, enabled = true } = parameters;

  const { data: weeklyPoints, isLoading } = useQuery<WeeklyPointsResponse>({
    queryKey: ["weeklyPoints", userIndex, min, max],
    queryFn: () => fetchWeeklyPoints(pointsUrl, userIndex!, { min, max }),
    enabled: enabled && !!userIndex,
  });

  return { weeklyPoints, isLoading };
}
