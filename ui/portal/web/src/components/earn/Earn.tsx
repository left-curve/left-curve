import type { PropsWithChildren } from "react";

import { m } from "~/paraglide/messages";

import { mockPoolsInfo, type PoolInfo } from "~/mock";
import type { TableColumn } from "@left-curve/applets-kit";

import { StrategyCard } from "@left-curve/applets-kit";
import { Cell, Table } from "@left-curve/applets-kit";

const EarnContainer: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

const EarnHeader: React.FC = () => {
  return (
    <div className="flex flex-col items-center justify-center pb-6 text-center">
      <img
        src="/images/emojis/detailed/pig.svg"
        alt="pig-detailed"
        className="w-[148px] h-[148px]"
      />
      <h1 className="exposure-h1-italic text-gray-900">{m["earn.title"]()}</h1>
      <p className="text-gray-500 diatype-lg-medium">{m["earn.description"]()}</p>
    </div>
  );
};

const EarnPoolsCards: React.FC = () => {
  return (
    <div className="flex gap-4 scrollbar-none justify-start lg:justify-between p-4 overflow-x-auto overflow-y-visible">
      <StrategyCard />
      <StrategyCard />
      <StrategyCard />
      <StrategyCard />
    </div>
  );
};

const EarnUserPoolsTable: React.FC = () => {
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

export const Earn = Object.assign(EarnContainer, {
  Header: EarnHeader,
  PoolsCards: EarnPoolsCards,
  UserPoolsTable: EarnUserPoolsTable,
});
