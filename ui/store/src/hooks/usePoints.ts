import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { fetchLeaderboard, fetchUserPoints, type LeaderboardEntry } from "./pointsApi.js";

export type UsePointsParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

const parseUdec = (value: string): number => Number(value);

const findUserRank = (
  leaderboard: Record<string, LeaderboardEntry> | undefined,
  userIndex: number | undefined,
): number => {
  if (!leaderboard || !userIndex) return 0;
  for (const [rankStr, entry] of Object.entries(leaderboard)) {
    if (entry.user_index === userIndex) return Number(rankStr);
  }
  return 0;
};

export function usePoints(parameters: UsePointsParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const pointsQuery = useQuery({
    queryKey: ["points", userIndex],
    queryFn: () => fetchUserPoints(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
  });

  const leaderboardQuery = useQuery({
    queryKey: ["leaderboard"],
    queryFn: () => fetchLeaderboard(pointsUrl),
    enabled,
  });

  const derived = useMemo(() => {
    const data = pointsQuery.data;
    const leaderboard = leaderboardQuery.data;

    const lpPoints = parseUdec(data?.vault ?? "0");
    const tradingPoints = parseUdec(data?.perps ?? "0") + parseUdec(data?.trades ?? "0");
    const referralPoints = parseUdec(data?.referral ?? "0");
    const points = lpPoints + tradingPoints + referralPoints;

    const rank = findUserRank(leaderboard, userIndex);

    const totalEntries = leaderboard ? Object.keys(leaderboard).length : 0;
    const percentile =
      rank > 0 && totalEntries > 0
        ? Math.min(((totalEntries - rank + 1) / totalEntries) * 100, 100)
        : 0;

    return { points, lpPoints, tradingPoints, referralPoints, rank, percentile };
  }, [pointsQuery.data, leaderboardQuery.data, userIndex]);

  return {
    ...derived,
    isLoading: pointsQuery.isLoading || leaderboardQuery.isLoading,
  };
}
