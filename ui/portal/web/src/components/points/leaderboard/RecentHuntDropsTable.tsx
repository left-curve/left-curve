import { Cell, IconStar, Table } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  type HuntedLatestEntry,
  type HuntedLoot,
  useHuntedLatest,
  useHuntedMultipliers,
} from "@left-curve/store";

import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";

import type { Decimal } from "@left-curve/utils";

import { formatUsername } from "./utils";

type RewardMeta = {
  label: () => string;
  // Hard-coded literal strings so Tailwind's JIT picks them up.
  colorClass: string;
  withStar: boolean;
};

const REWARD_META: Record<HuntedLoot, RewardMeta> = {
  pearl_dango: {
    label: () => m["points.leaderboard.recentDrops.rewards.pearl_dango"](),
    colorClass: "text-utility-blue-600",
    withStar: true,
  },
  silver_shell: {
    label: () => m["points.leaderboard.recentDrops.rewards.silver_shell"](),
    colorClass: "text-utility-green-500",
    withStar: false,
  },
  bronze_shell: {
    label: () => m["points.leaderboard.recentDrops.rewards.bronze_shell"](),
    colorClass: "text-utility-error-400",
    withStar: false,
  },
  golden_shell: {
    label: () => m["points.leaderboard.recentDrops.rewards.golden_shell"](),
    colorClass: "text-utility-warning-500",
    withStar: false,
  },
};

function formatBoost(multiplier: InstanceType<typeof Decimal> | null): string {
  if (!multiplier) return "—";
  const pct = multiplier.minus(1).times(100);
  // Trim trailing zeros — "+50%" rather than "+50.00%".
  return `+${pct.toFixed(2).replace(/\.?0+$/, "")}%`;
}

/**
 * `grug_types::Timestamp` serializes as `"seconds.nanoseconds"` (e.g.
 * `"1732770602.144737024"`). `new Date(...)` on that string yields Invalid Date,
 * so we parse it as a float and multiply by 1000 to feed Date(ms).
 */
function parseGrugTimestamp(raw: string): Date | null {
  const seconds = Number.parseFloat(raw);
  if (!Number.isFinite(seconds)) return null;
  return new Date(seconds * 1000);
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
        const meta = REWARD_META[row.original.loot];
        return (
          <div className={`flex items-center gap-1 diatype-m-medium ${meta.colorClass}`}>
            <span>{meta.label()}</span>
            {meta.withStar ? <IconStar className="w-4 h-4" /> : null}
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
      cell: ({ row }) => {
        const date = parseGrugTimestamp(row.original.block_timestamp);
        return (
          <div className="flex justify-end">
            {date ? <Cell.Age date={date} addSuffix /> : <Cell.Text text="—" />}
          </div>
        );
      },
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
