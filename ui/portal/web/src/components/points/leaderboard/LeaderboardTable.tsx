import { Cell, FormattedNumber, Pagination, SortHeader, Tab, Table, Tabs } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useLeaderboard } from "@left-curve/store";
import { useMemo } from "react";

import type { TableColumn } from "@left-curve/applets-kit";
import type { LeaderboardTimeframe } from "@left-curve/store";
import type React from "react";

import { useUserPoints } from "../useUserPoints";

type LeaderboardRow = {
  ranking: number | null;
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
  const { userIndex, username } = useAccount();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { points: userPoints, volume: userVolume, pnl: userPnl } = useUserPoints();

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

    if (userIndex) {
      if (pinnedUser) {
        result.push({
          ranking: pinnedUser.originalRank,
          user: formatUsername(pinnedUser.username, pinnedUser.user_index),
          volume: Number(pinnedUser.stats.volume),
          pnl: Number(pinnedUser.stats.realized_pnl),
          points: totalPoints(pinnedUser.stats.points),
          isCurrentUser: true,
          isPinned: true,
        });
      } else {
        result.push({
          ranking: null,
          user: formatUsername(username ?? null, userIndex),
          volume: userVolume,
          pnl: userPnl,
          points: userPoints,
          isCurrentUser: true,
          isPinned: true,
        });
      }
    }

    for (const entry of entries) {
      const isCurrentUser = entry.user_index === userIndex;
      result.push({
        ranking: entry.originalRank,
        user: formatUsername(entry.username, entry.user_index),
        volume: Number(entry.stats.volume),
        pnl: Number(entry.stats.realized_pnl),
        points: totalPoints(entry.stats.points),
        isCurrentUser,
        isPinned: false,
      });
    }

    return result;
  }, [entries, pinnedUser, userIndex, username, userVolume, userPnl, userPoints]);

  const columns: TableColumn<LeaderboardRow> = [
    {
      id: "ranking",
      header: m["points.leaderboard.columns.ranking"](),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text
          text={row.original.ranking !== null ? String(row.original.ranking) : "-"}
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
      cell: ({ row }) => (
        <Cell.Text text={<FormattedNumber number={row.original.volume} formatOptions={{ currency: "USD" }} as="span" />} />
      ),
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
      cell: ({ row }) => (
        <Cell.Text text={<FormattedNumber number={row.original.pnl} formatOptions={{ currency: "USD" }} as="span" />} />
      ),
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
        <Cell.Text text={<><FormattedNumber number={row.original.points} formatOptions={{ fractionDigits: 0 }} as="span" /> {m["points.header.points"]()}</>} />
      ),
    },
  ];

  return (
    <div className="p-4 lg:p-8 flex flex-col gap-4 min-h-[45.5rem]">
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
          base: "shadow-none p-0 pt-0 bg-surface-primary-gray",
          row: "border-b border-outline-secondary-gray",
          cell: "px-6 py-4",
        }}
        emptyComponent={
          <div className="flex items-center justify-center py-16">
            <p className="text-ink-tertiary-500 diatype-m-medium">No data available</p>
          </div>
        }
        bottomContent={
          totalPages > 1 ? (
            <Pagination totalPages={totalPages} currentPage={page} onPageChange={setPage} />
          ) : null
        }
      />
    </div>
  );
};
