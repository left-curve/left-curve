import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { type AttackCompensation, fetchTotalUsersWithPoints, fetchUserStats } from "./pointsApi.js";

export type UsePointsParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

const parseUdec = (value: string): number => Number(value);

export function usePoints(parameters: UsePointsParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const userStatsQuery = useQuery({
    queryKey: ["userStats", userIndex],
    queryFn: () => fetchUserStats(pointsUrl, userIndex!),
    enabled: enabled && !!userIndex,
  });

  const totalUsersQuery = useQuery({
    queryKey: ["totalUsersWithPoints"],
    queryFn: () => fetchTotalUsersWithPoints(pointsUrl),
    enabled,
  });

  const derived = useMemo(() => {
    const data = userStatsQuery.data;
    const totalUsers = totalUsersQuery.data ?? 0;

    const lpPoints = parseUdec(data?.stats.points.vault ?? "0");
    const tradingPoints = parseUdec(data?.stats.points.perps ?? "0");
    const referralPoints = parseUdec(data?.stats.points.referral ?? "0");
    const points = lpPoints + tradingPoints + referralPoints;
    const volume = parseUdec(data?.stats.volume ?? "0");
    const pnl = parseUdec(data?.stats.realized_pnl ?? "0");
    const rank = data?.rank ?? 0;

    const percentile =
      rank > 0 && totalUsers > 0 ? Math.min(((totalUsers - rank + 1) / totalUsers) * 100, 100) : 0;

    const compensation: AttackCompensation | undefined = data?.compensation;

    return {
      points,
      lpPoints,
      tradingPoints,
      referralPoints,
      volume,
      pnl,
      rank,
      percentile,
      compensation,
    };
  }, [userStatsQuery.data, totalUsersQuery.data]);

  return {
    ...derived,
    isLoading: userStatsQuery.isLoading || totalUsersQuery.isLoading,
  };
}
