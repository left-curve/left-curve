import { Cell, Pagination, SortHeader, Tab, Table, Tabs } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useLeaderboard } from "@left-curve/store";
import { useMemo } from "react";

import type { TableColumn } from "@left-curve/applets-kit";
import type { LeaderboardTimeframe } from "@left-curve/store";
import type React from "react";

import { useUserPoints } from "../useUserPoints";

type LeaderboardRow = {
  ranking: number;
  user: string;
  volume: number;
  pnl: number;
  points: number;
  isCurrentUser: boolean;
  isPinned: boolean;
};

const TIMEFRAME_TABS: { label: string; value: LeaderboardTimeframe }[] = [
  { label: m["points.leaderboard.timeframes.allTime"](), value: null },
  { label: m["points.leaderboard.timeframes.oneWeek"](), value: 1 },
  { label: m["points.leaderboard.timeframes.twoWeeks"](), value: 2 },
  { label: m["points.leaderboard.timeframes.oneMonth"](), value: 4 },
];

function totalPoints(points: { vault: string; perps: string; referral: string }): number {
  return Number(points.vault) + Number(points.perps) + Number(points.referral);
}

function formatUsername(username: string | null, userIndex: number): string {
  if (!username) return m["points.leaderboard.userFallback"]({ index: String(userIndex) });
  const match = username.match(/^user_(\d+)$/);
  if (match) return m["points.leaderboard.userFallback"]({ index: match[1] });
  return username;
}

export const LeaderboardTable: React.FC = () => {
  const { userIndex } = useAccount();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { rank: userRank } = useUserPoints();

  const {
    entries,
    pinnedUser,
    sort,
    sortDirection,
    timeframe,
    page,
    totalPages,
    isLoading,
    setPage,
    handleSortChange,
    handleTimeframeChange,
  } = useLeaderboard({ pointsUrl, userIndex });

  const rows = useMemo((): LeaderboardRow[] => {
    const result: LeaderboardRow[] = [];

    if (pinnedUser && userIndex) {
      result.push({
        ranking: userRank,
        user: formatUsername(pinnedUser.username, pinnedUser.user_index),
        volume: Number(pinnedUser.stats.volume),
        pnl: Number(pinnedUser.stats.realized_pnl),
        points: totalPoints(pinnedUser.stats.points),
        isCurrentUser: true,
        isPinned: true,
      });
    }

    for (const entry of entries) {
      result.push({
        ranking: entry.originalRank,
        user: formatUsername(entry.username, entry.user_index),
        volume: Number(entry.stats.volume),
        pnl: Number(entry.stats.realized_pnl),
        points: totalPoints(entry.stats.points),
        isCurrentUser: false,
        isPinned: false,
      });
    }

    return result;
  }, [entries, pinnedUser, userIndex, userRank]);

  const columns: TableColumn<LeaderboardRow> = [
    {
      id: "ranking",
      header: m["points.leaderboard.columns.ranking"](),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text
          text={String(row.original.ranking)}
          className={row.original.isPinned ? "font-bold" : ""}
        />
      ),
    },
    {
      id: "user",
      header: m["points.leaderboard.columns.user"](),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text
          text={
            row.original.isCurrentUser
              ? `${row.original.user} ${m["points.leaderboard.you"]()}`
              : row.original.user
          }
          className={row.original.isCurrentUser ? "font-bold" : ""}
        />
      ),
    },
    {
      id: "volume",
      header: () => (
        <SortHeader
          label={m["points.leaderboard.columns.volume"]()}
          sorted={sort === "volume" ? sortDirection : false}
          toggleSort={() => handleSortChange("volume")}
        />
      ),
      enableSorting: false,
      cell: ({ row }) => <Cell.Text text={`$${row.original.volume.toLocaleString()}`} />,
    },
    {
      id: "pnl",
      header: () => (
        <SortHeader
          label={m["points.leaderboard.columns.pnl"]()}
          sorted={sort === "pnl" ? sortDirection : false}
          toggleSort={() => handleSortChange("pnl")}
        />
      ),
      enableSorting: false,
      cell: ({ row }) => <Cell.Text text={`$${row.original.pnl.toLocaleString()}`} />,
    },
    {
      id: "points",
      header: () => (
        <SortHeader
          label={m["points.leaderboard.columns.points"]()}
          sorted={sort === "points" ? sortDirection : false}
          toggleSort={() => handleSortChange("points")}
          className="ml-auto w-full justify-end"
        />
      ),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text text={`${row.original.points.toLocaleString()} ${m["points.header.points"]()}`} />
      ),
    },
  ];

  return (
    <div className="p-4 lg:p-8 flex flex-col gap-4">
      <Tabs
        layoutId="leaderboard-timeframe-tabs"
        selectedTab={String(timeframe ?? "all")}
        onTabChange={(v) => {
          const tf = v === "all" ? null : (Number(v) as LeaderboardTimeframe);
          handleTimeframeChange(tf);
        }}
      >
        {TIMEFRAME_TABS.map(({ label, value }) => (
          <Tab key={label} title={String(value ?? "all")}>
            {label}
          </Tab>
        ))}
      </Tabs>

      <Table
        data={rows}
        columns={columns}
        style="default"
        isLoading={isLoading}
        classNames={{
          row: "border-b border-outline-secondary-gray",
          cell: "px-6 py-4",
        }}
        bottomContent={
          totalPages > 1 ? (
            <Pagination totalPages={totalPages} currentPage={page} onPageChange={setPage} />
          ) : null
        }
      />
    </div>
  );
};
