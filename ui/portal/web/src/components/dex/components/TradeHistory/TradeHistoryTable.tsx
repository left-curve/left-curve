import { IconChevronLeft, IconChevronRight, Table, twMerge } from "@left-curve/applets-kit";
import { useNavigate } from "@tanstack/react-router";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { EmptyPlaceholder } from "../../../foundation/EmptyPlaceholder";

import type { TableColumn } from "@left-curve/applets-kit";
import type { GraphqlQueryResult } from "@left-curve/types";

type TradeHistoryPagination = {
  goNext: () => void;
  goPrev: () => void;
  hasNextPage: boolean;
  hasPreviousPage: boolean;
};

type TradeHistoryTableProps<T extends { blockHeight: number }> = {
  data: GraphqlQueryResult<T> | undefined;
  columns: TableColumn<T>;
  isLoading: boolean;
  pagination?: TradeHistoryPagination;
};

export function TradeHistoryTable<T extends { blockHeight: number }>({
  data,
  columns,
  isLoading,
  pagination,
}: TradeHistoryTableProps<T>) {
  const navigate = useNavigate();
  const hasRows = !!data?.nodes.length;
  const showPagination =
    !!pagination && (hasRows || pagination.hasNextPage || pagination.hasPreviousPage);

  return (
    <Table
      data={data?.nodes || []}
      columns={columns}
      style="simple"
      onRowClick={(row) =>
        navigate({
          to: "/block/$block",
          params: { block: row.original.blockHeight.toString() },
        })
      }
      classNames={{
        row: "h-fit",
        header: "pt-0",
        base: "pb-0 max-h-[31vh] overflow-y-scroll",
        cell: twMerge("diatype-xs-regular py-1", {
          "group-hover:bg-transparent": !data?.nodes.length,
        }),
      }}
      emptyComponent={
        <EmptyPlaceholder
          component={m["dex.protrade.history.noOpenOrders"]()}
          className="h-[3.5rem]"
        />
      }
      bottomContent={
        showPagination ? <ArrowsPagination pagination={pagination} isLoading={isLoading} /> : null
      }
    />
  );
}

const ArrowsPagination: React.FC<{
  pagination: TradeHistoryPagination;
  isLoading: boolean;
}> = ({ pagination, isLoading }) => {
  const { goPrev, goNext, hasPreviousPage, hasNextPage } = pagination;
  const buttonClass =
    "flex items-center justify-center w-7 h-7 rounded-sm text-ink-secondary-blue hover:bg-surface-secondary-blue transition-colors disabled:opacity-40 disabled:hover:bg-transparent disabled:cursor-not-allowed";
  return (
    <div className="flex w-full items-center justify-end gap-1 py-2 px-1">
      <button
        type="button"
        onClick={goPrev}
        disabled={!hasPreviousPage || isLoading}
        aria-label={m["pagination.previous"]()}
        className={buttonClass}
      >
        <IconChevronLeft className="w-5 h-5" />
      </button>
      <button
        type="button"
        onClick={goNext}
        disabled={!hasNextPage || isLoading}
        aria-label={m["pagination.next"]()}
        className={buttonClass}
      >
        <IconChevronRight className="w-5 h-5" />
      </button>
    </div>
  );
};
