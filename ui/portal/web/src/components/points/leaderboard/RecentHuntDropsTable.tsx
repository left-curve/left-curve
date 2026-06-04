import { Cell, IconStar, Table, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  type HuntedLatestEntry,
  type HuntedLoot,
  useHuntedLatest,
  useHuntedMultipliers,
} from "@left-curve/store";

import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";

import { type Decimal, formatUsername } from "@left-curve/utils";

const REWARD_META: Record<HuntedLoot, { color: string; star?: boolean }> = {
  pearl_dango: { color: "text-utility-blue-600", star: true },
  silver_shell: { color: "text-utility-green-500" },
  bronze_shell: { color: "text-utility-error-400" },
  golden_shell: { color: "text-utility-warning-500" },
};

const REWARD_LABELS: Record<HuntedLoot, () => string> = {
  pearl_dango: m["points.leaderboard.recentDrops.rewards.pearl_dango"],
  silver_shell: m["points.leaderboard.recentDrops.rewards.silver_shell"],
  bronze_shell: m["points.leaderboard.recentDrops.rewards.bronze_shell"],
  golden_shell: m["points.leaderboard.recentDrops.rewards.golden_shell"],
};

function formatBoost(multiplier: InstanceType<typeof Decimal> | null): string {
  if (!multiplier) return "—";
  const pct = multiplier.minus(1).times(100);
  return `+${pct.toFixed(2).replace(/\.?0+$/, "")}%`;
}

export const RecentHuntDropsTable: React.FC = () => {
  const pointsUrl = window.dango.urls.pointsUrl;
  const { entries, isLoading, isFetching } = useHuntedLatest({ pointsUrl });
  const { resolveMultiplier } = useHuntedMultipliers({ pointsUrl });

  const columns: TableColumn<HuntedLatestEntry> = [
    {
      id: "user",
      header: m["points.leaderboard.recentDrops.columns.user"](),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text text={formatUsername(row.original.username, row.original.user_index)} />
      ),
    },
    {
      id: "rewardType",
      header: m["points.leaderboard.recentDrops.columns.rewardType"](),
      enableSorting: false,
      cell: ({ row }) => {
        const { color, star } = REWARD_META[row.original.loot];
        return (
          <div className={twMerge("flex items-center gap-1 diatype-m-medium", color)}>
            <span>{REWARD_LABELS[row.original.loot]()}</span>
            {star ? <IconStar className="w-4 h-4" /> : null}
          </div>
        );
      },
    },
    {
      id: "boost",
      header: m["points.leaderboard.recentDrops.columns.boost"](),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text text={formatBoost(resolveMultiplier(row.original.loot, row.original.epoch))} />
      ),
    },
    {
      id: "dropped",
      header: () => (
        <span className="ml-auto block w-full text-right">
          {m["points.leaderboard.recentDrops.columns.dropped"]()}
        </span>
      ),
      enableSorting: false,
      cell: ({ row }) => (
        <div className="flex justify-end">
          <Cell.Age
            date={new Date(Number.parseFloat(row.original.block_timestamp) * 1000)}
            addSuffix
          />
        </div>
      ),
    },
  ];

  return (
    <div className="p-4">
      <Table
        data={entries}
        columns={columns}
        style="default"
        isLoading={(isLoading || isFetching) && entries.length === 0}
        classNames={{
          base: "shadow-none p-0 pt-0 bg-surface-primary-gray",
          row: "border-b border-outline-secondary-gray",
          cell: "px-6 py-4",
        }}
        emptyComponent={
          <div className="flex items-center justify-center py-16">
            <p className="text-ink-tertiary-500 diatype-m-medium">
              {m["points.leaderboard.recentDrops.empty"]()}
            </p>
          </div>
        }
      />
    </div>
  );
};
