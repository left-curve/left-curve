import { Button, Cell, Table } from "@left-curve/applets-kit";

import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";

type PointsHistoryRow = {
  activity: string;
  date: string;
  points: number;
};

const mockPointsHistory: PointsHistoryRow[] = [
  { activity: "Provide Liquidity", date: "Jan 5, 2025", points: 15 },
  { activity: "Provide Liquidity", date: "Jan 5, 2025", points: 37 },
  { activity: "Provide Liquidity", date: "Jan 12, 2025", points: 12 },
  { activity: "Provide Liquidity", date: "Jan 19, 2025", points: 55 },
  { activity: "Provide Liquidity", date: "Jan 26, 2025", points: 8 },
  { activity: "Provide Liquidity", date: "Feb 2, 2025", points: 29 },
  { activity: "Provide Liquidity", date: "Feb 9, 2025", points: 41 },
  { activity: "Referral", date: "Feb 16, 2025", points: 19 },
  { activity: "Swap", date: "Feb 23, 2025", points: 63 },
  { activity: "Provide Liquidity", date: "Mar 2, 2025", points: 3 },
];

export const PointsProfileTable: React.FC = () => {
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

  return (
    <Table
      data={mockPointsHistory}
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
