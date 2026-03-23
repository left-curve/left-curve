import { Cell, Table } from "@left-curve/applets-kit";
import { useAccount, useWeeklyPoints } from "@left-curve/store";
import { useMemo } from "react";

import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";

type PointsHistoryRow = {
  activity: string;
  date: string;
  points: number;
};

// TODO: make configurable or fetch from backend
const EVENT_START_EPOCH = 1735689600; // Jan 1, 2025 00:00:00 UTC
const SECONDS_PER_WEEK = 604_800;

const SOURCE_LABELS: Record<string, string> = {
  vault: "Provide Liquidity",
  perps: "Trade",
  referral: "Referral",
};

const formatWeekDate = (weekNumber: number): string => {
  const timestamp = EVENT_START_EPOCH + weekNumber * SECONDS_PER_WEEK;
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
};

export const PointsProfileTable: React.FC = () => {
  const { userIndex } = useAccount();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { weeklyPoints, isLoading } = useWeeklyPoints({ pointsUrl, userIndex });

  const rows = useMemo((): PointsHistoryRow[] => {
    if (!weeklyPoints) return [];
    const result: PointsHistoryRow[] = [];
    for (const [weekStr, points] of Object.entries(weeklyPoints)) {
      const weekNumber = Number(weekStr);
      const date = formatWeekDate(weekNumber);
      for (const [source, label] of Object.entries(SOURCE_LABELS)) {
        const value = Number(points[source as keyof typeof points] ?? "0");
        if (value > 0) {
          result.push({ activity: label, date, points: value });
        }
      }
    }
    return result.sort((a, b) => b.date.localeCompare(a.date));
  }, [weeklyPoints]);

  const columns: TableColumn<PointsHistoryRow> = [
    {
      header: "Action",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900" text={row.original.activity} />
      ),
    },
    {
      header: "Date",
      cell: ({ row }) => <Cell.Text className="text-ink-primary-900" text={row.original.date} />,
    },
    {
      header: "Points",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900" text={`${row.original.points} XPoints`} />
      ),
    },
  ];

  if (isLoading) {
    return (
      <div className="px-6 py-8 text-center text-ink-tertiary-500 diatype-m-regular">
        Loading point history...
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="px-6 py-8 text-center text-ink-tertiary-500 diatype-m-regular">
        No point history yet
      </div>
    );
  }

  return (
    <Table
      data={rows}
      columns={columns}
      style="simple"
      classNames={{
        base: "rounded-none bg-surface-primary-rice p-0",
        header: "hidden",
        cell: "px-6 py-4",
        row: "border-b border-outline-secondary-gray last:border-b-0",
      }}
    />
  );
};
