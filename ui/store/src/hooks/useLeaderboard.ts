import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import { fetchLeaderboard, type LeaderboardEntry } from "./pointsApi.js";

export type LeaderboardSort = "points" | "pnl" | "volume";
export type LeaderboardTimeframe = null | 1 | 2 | 4;

export type LeaderboardEntryWithRank = LeaderboardEntry & { originalRank: number };

export type UseLeaderboardParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

const PAGE_SIZE = 10;

export function useLeaderboard(parameters: UseLeaderboardParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;

  const [sort, setSort] = useState<LeaderboardSort>("points");
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("desc");
  const [timeframe, setTimeframe] = useState<LeaderboardTimeframe>(null);
  const [page, setPage] = useState(1);

  const leaderboardQuery = useQuery({
    queryKey: ["leaderboard", sort, timeframe],
    queryFn: () =>
      fetchLeaderboard(pointsUrl, {
        sort,
        timeframe: timeframe ?? undefined,
      }),
    enabled,
  });

  const { entries, pinnedUser, totalPages } = useMemo(() => {
    const raw = leaderboardQuery.data ?? [];

    const ranked = raw.map((e, i) => ({ ...e, originalRank: i + 1 }));

    const pinnedUser = ranked.find((e) => e.user_index === userIndex) ?? null;
    const filtered = userIndex ? ranked.filter((e) => e.user_index !== userIndex) : ranked;

    const sorted = sortDirection === "asc" ? [...filtered].reverse() : filtered;

    const start = (page - 1) * PAGE_SIZE;
    const entries = sorted.slice(start, start + PAGE_SIZE);
    const totalPages = Math.ceil(sorted.length / PAGE_SIZE);

    return { entries, pinnedUser, totalPages };
  }, [leaderboardQuery.data, userIndex, sortDirection, page]);

  const handleSortChange = (newSort: LeaderboardSort) => {
    if (newSort === sort) {
      setSortDirection((d) => (d === "desc" ? "asc" : "desc"));
    } else {
      setSort(newSort);
      setSortDirection("desc");
    }
    setPage(1);
  };

  const handleTimeframeChange = (tf: LeaderboardTimeframe) => {
    setTimeframe(tf);
    setPage(1);
  };

  return {
    entries,
    pinnedUser,
    sort,
    sortDirection,
    timeframe,
    page,
    totalPages,
    isLoading: leaderboardQuery.isLoading,
    setPage,
    handleSortChange,
    handleTimeframeChange,
  };
}
