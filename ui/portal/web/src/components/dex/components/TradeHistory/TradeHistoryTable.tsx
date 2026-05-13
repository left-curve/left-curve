import { Table, twMerge } from "@left-curve/applets-kit";
import { useNavigate } from "@tanstack/react-router";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { EmptyPlaceholder } from "../../../foundation/EmptyPlaceholder";

import type { TableColumn } from "@left-curve/applets-kit";
import type { GraphqlQueryResult } from "@left-curve/dango/types";

type TradeHistoryTableProps<T extends { blockHeight: number }> = {
  data: GraphqlQueryResult<T> | undefined;
  columns: TableColumn<T>;
  isLoading: boolean;
};

export function TradeHistoryTable<T extends { blockHeight: number }>({
  data,
  columns,
  isLoading,
}: TradeHistoryTableProps<T>) {
  const navigate = useNavigate();

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
    />
  );
}
