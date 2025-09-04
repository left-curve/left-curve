import { useNavigate } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Cell, CursorPagination, Table } from "@left-curve/applets-kit";

import type { TableColumn } from "@left-curve/applets-kit";
import type { IndexedTransaction } from "@left-curve/dango/types";

type TransactionsTableProps = {
  transactions?: IndexedTransaction[];
  pagination?: {
    isLoading: boolean;
    goNext: () => void;
    goPrev: () => void;
    hasNextPage: boolean;
    hasPreviousPage: boolean;
  };
};

export const TransactionsTable: React.FC<TransactionsTableProps> = ({
  transactions,
  pagination,
}) => {
  const navigate = useNavigate();

  const columns: TableColumn<IndexedTransaction> = [
    {
      header: "Hash",
      cell: ({ row }) => (
        <Cell.TxHash
          hash={row.original.hash}
          navigate={() => navigate({ to: `/tx/${row.original.hash}` })}
        />
      ),
    },
    {
      header: "Block",
      cell: ({ row }) => (
        <Cell.BlockHeight
          blockHeight={row.original.blockHeight}
          navigate={() => navigate({ to: `/block/${row.original.blockHeight}` })}
        />
      ),
    },
    {
      header: "Age",
      cell: ({ row }) => <Cell.Age date={row.original.createdAt} addSuffix />,
    },
    {
      header: "Sender",
      cell: ({ row }) => (
        <Cell.Sender sender={row.original.sender} navigate={(url) => navigate({ to: url })} />
      ),
    },
    {
      header: "Actions",
      cell: ({ row }) => <Cell.TxMessages messages={row.original.messages} />,
    },
    {
      header: "Result",
      cell: ({ row }) => {
        const { hasSucceeded, messages } = row.original;

        return (
          <Cell.TxResult
            className="justify-end"
            isSuccess={hasSucceeded}
            text={m["explorer.txs.result"]({ result: String(hasSucceeded) })}
            total={messages.length}
          />
        );
      },
    },
  ];

  if (!transactions?.length) return null;

  return (
    <Table
      data={transactions}
      columns={columns}
      bottomContent={
        pagination ? (
          <CursorPagination
            {...pagination}
            className="flex w-full justify-end gap-2"
            nextLabel={m["pagination.next"]()}
            previousLabel={m["pagination.previous"]()}
          />
        ) : null
      }
    />
  );
};
