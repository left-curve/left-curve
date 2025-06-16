import { Cell, Table, type TableColumn } from "@left-curve/applets-kit";
import type React from "react";
import { mockPoolsInfo, type PoolInfo } from "~/mock";

export const PoolsTable: React.FC = () => {
  const columns: TableColumn<PoolInfo> = [
    {
      header: "Vault",
      cell: ({ row }) => <Cell.Assets assets={row.original.pairs} />,
    },
    {
      header: "APR",
      cell: () => <Cell.Text text="TBD" />,
    },
    {
      header: "My Position",
      cell: ({ row }) => <Cell.Text text={row.original.userPosition} />,
    },
    {
      header: "TVL",
      cell: ({ row }) => <Cell.Text text={row.original.tvl} />,
    },
    {
      header: "Manage",
      cell: () => <Cell.Action action={() => console.log("Manage Pool")} label="Manage" />,
    },
  ];

  return (
    <div className="flex w-full p-4">
      <Table data={mockPoolsInfo} columns={columns} />
    </div>
  );
};
