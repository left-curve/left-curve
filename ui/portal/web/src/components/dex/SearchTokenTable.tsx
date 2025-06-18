import { useConfig, usePrices } from "@left-curve/store";
import { useMemo } from "react";

import { Cell, Table } from "@left-curve/applets-kit";

import type { TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type { PairId, PairUpdate } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";
import { useApp } from "~/hooks/useApp";

const SearchTokenTableContainer: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type SearchTokenTableProps = {
  classNames?: TableClassNames;
  data: PairUpdate[];
  searchText?: string;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
};

const SearchTokenSpotTable: React.FC<SearchTokenTableProps> = ({
  classNames,
  data,
  searchText,
  onChangePairId,
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { coins } = useConfig();
  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const columns: TableColumn<PairUpdate> = [
    {
      id: "pairName",
      header: "Name",
      cell: ({ row }) => (
        <Cell.PairName
          type="Spot"
          pairId={{ baseDenom: row.original.baseDenom, quoteDenom: row.original.quoteDenom }}
        />
      ),
      filterFn: (row, _, value) => {
        const baseCoin = coins[row.original.baseDenom];
        const quoteCoin = coins[row.original.quoteDenom];

        return baseCoin.symbol.includes(value) || quoteCoin.symbol.includes(value);
      },
    },
    {
      header: "Price",
      cell: ({ row }) => <Cell.Text text={getPrice(1, row.original.baseDenom, { format: true })} />,
    },
    {
      header: "24h Change",
      cell: ({ row }) => <Cell.Text text="-" />,
    },
    {
      header: "Volume",
      cell: ({ row }) => <Cell.Text text="-" />,
    },
  ];

  const columnFilters = useMemo(() => [{ id: "pairName", value: searchText }], [searchText]);

  return (
    <Table
      data={data}
      columns={columns}
      style="simple"
      classNames={classNames}
      columnFilters={columnFilters}
      onRowClick={(row) =>
        onChangePairId({
          baseDenom: row.original.baseDenom,
          quoteDenom: row.original.quoteDenom,
        })
      }
    />
  );
};

export const SearchTokenTable = Object.assign(SearchTokenTableContainer, {
  Spot: SearchTokenSpotTable,
});
