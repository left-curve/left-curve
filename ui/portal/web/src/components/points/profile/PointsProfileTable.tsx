import { Button, Cell, Pagination, SortHeader, Table, useApp } from "@left-curve/applets-kit";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useEpochPoints } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useCallback, useMemo, useState } from "react";
import { useUserPoints } from "../useUserPoints";

import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";

type EpochHistoryRow = {
  epoch: number;
  epochLabel: string;
  dateRange: string;
  dateTimestamp: number;
  points: number;
};

type SortKey = "date" | "points";
type SortDir = "asc" | "desc";

const formatEpochDateRange = (startedAt: string, endedAt: string): string => {
  const start = new Date(Number.parseFloat(startedAt) * 1000);
  const end = new Date(Number.parseFloat(endedAt) * 1000);
  const dateOpts: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" };
  const timeOpts: Intl.DateTimeFormatOptions = {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  };
  const startDate = start.toLocaleDateString("en-US", dateOpts);
  const startTime = start.toLocaleTimeString("en-US", timeOpts);
  const endDate = end.toLocaleDateString("en-US", dateOpts);
  const endTime = end.toLocaleTimeString("en-US", timeOpts);
  return `${startDate} · ${startTime} – ${endDate} · ${endTime}`;
};

const PAGE_SIZE = 10;

export const PointsProfileTable: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { userIndex } = useAccount();
  const navigate = useNavigate();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { epochPoints, isLoading } = useEpochPoints({ pointsUrl, userIndex });
  const { compensation } = useUserPoints();
  const [page, setPage] = useState(1);
  const [sortKey, setSortKey] = useState<SortKey>("date");
  const [sortDir, setSortDir] = useState<SortDir>("desc");

  const allRows = useMemo((): EpochHistoryRow[] => {
    const rows: EpochHistoryRow[] = [];

    if (compensation) {
      const compensationPoints = Number(compensation.vault) + Number(compensation.unrealized);
      if (compensationPoints > 0) {
        rows.push({
          epoch: 0,
          epochLabel: m["points.profile.epochLabel"]({ number: "0" }),
          dateRange: m["points.profile.compensation"](),
          dateTimestamp: 0,
          points: compensationPoints,
        });
      }
    }

    if (epochPoints) {
      for (const [epoch, epochStats] of epochPoints) {
        const vault = Number(epochStats.stats.points.vault);
        const perps = Number(epochStats.stats.points.perps);
        const referral = Number(epochStats.stats.points.referral);
        const total = vault + perps + referral;
        if (total > 0) {
          rows.push({
            epoch,
            epochLabel: m["points.profile.epochLabel"]({ number: String(epoch) }),
            dateRange: formatEpochDateRange(epochStats.started_at, epochStats.ended_at),
            dateTimestamp: Number.parseFloat(epochStats.started_at),
            points: total,
          });
        }
      }
    }

    return rows;
  }, [epochPoints, compensation]);

  const sortedRows = useMemo(() => {
    const accessor =
      sortKey === "date"
        ? (r: EpochHistoryRow) => r.dateTimestamp
        : (r: EpochHistoryRow) => r.points;
    const sorted = [...allRows].sort((a, b) => accessor(a) - accessor(b));
    return sortDir === "desc" ? sorted.reverse() : sorted;
  }, [allRows, sortKey, sortDir]);

  const paginatedRows = useMemo(() => {
    const start = (page - 1) * PAGE_SIZE;
    return sortedRows.slice(start, start + PAGE_SIZE);
  }, [sortedRows, page]);

  const totalPages = Math.ceil(sortedRows.length / PAGE_SIZE);

  const handleSort = useCallback(
    (key: SortKey) => {
      if (key === sortKey) {
        setSortDir((d) => (d === "desc" ? "asc" : "desc"));
      } else {
        setSortKey(key);
        setSortDir("desc");
      }
      setPage(1);
    },
    [sortKey],
  );

  const columns: TableColumn<EpochHistoryRow> = [
    {
      id: "epoch",
      header: m["points.profile.columns.epoch"](),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900" text={row.original.epochLabel} />
      ),
    },
    {
      id: "date",
      header: () => (
        <SortHeader
          label={m["points.profile.columns.date"]()}
          sorted={sortKey === "date" ? sortDir : false}
          toggleSort={() => handleSort("date")}
        />
      ),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900" text={row.original.dateRange} />
      ),
    },
    {
      id: "points",
      header: () => (
        <SortHeader
          label={m["points.profile.columns.points"]()}
          sorted={sortKey === "points" ? sortDir : false}
          toggleSort={() => handleSort("points")}
          className="ml-auto w-full justify-end"
        />
      ),
      enableSorting: false,
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900"
          text={m["points.profile.xPoints"]({ points: formatNumber(row.original.points, formatNumberOptions) })}
        />
      ),
    },
  ];

  if (isLoading) {
    return (
      <div className="px-6 py-8 text-center text-ink-tertiary-500 diatype-m-regular">
        {m["points.profile.loading"]()}
      </div>
    );
  }

  return (
    <Table
      data={paginatedRows}
      columns={columns}
      style="default"
      emptyComponent={
        <div className="px-6 py-8 text-center text-ink-tertiary-500 diatype-m-regular">
          {m["points.profile.noHistory"]()}
        </div>
      }
      classNames={{
        base: "p-0 p-4",
        cell: "px-6 py-4",
        row: "border-b border-outline-secondary-gray last:border-b-0",
      }}
      bottomContent={
        <div>
          {totalPages > 1 ? (
            <Pagination totalPages={totalPages} currentPage={page} onPageChange={setPage} />
          ) : null}
          {allRows.length === 0 && (
            <div className="px-6 py-4 flex items-center justify-center">
              <Button onClick={() => navigate({ to: "/trade" })}>
                {m["points.profile.getStarted"]()}
              </Button>
            </div>
          )}
        </div>
      }
    />
  );
};
