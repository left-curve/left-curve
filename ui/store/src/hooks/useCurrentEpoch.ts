import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { fetchCurrentEpoch } from "./pointsApi.js";

export type UseCurrentEpochParameters = {
  pointsUrl: string;
  enabled?: boolean;
};

export function useCurrentEpoch(parameters: UseCurrentEpochParameters) {
  const { pointsUrl, enabled = true } = parameters;

  const query = useQuery({
    queryKey: ["currentEpoch"],
    queryFn: () => fetchCurrentEpoch(pointsUrl),
    enabled,
    refetchInterval: 60000,
  });

  const derived = useMemo(() => {
    const data = query.data;
    if (!data) return { isStarted: false, currentEpoch: null, endDate: null, startsAt: null };

    if (data.status === "not_started") {
      return {
        isStarted: false as const,
        currentEpoch: null,
        endDate: null,
        startsAt: data.starts_at,
      };
    }

    const remainingSeconds = Math.floor(Number(data.remaining));
    const endDate = new Date(Date.now() + remainingSeconds * 1000);

    return {
      isStarted: true as const,
      currentEpoch: data.current_epoch,
      endDate,
      startsAt: null,
    };
  }, [query.data]);

  return {
    ...derived,
    isLoading: query.isLoading,
    refetch: query.refetch,
  };
}
