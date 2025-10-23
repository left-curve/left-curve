import { useConfig, useFavPairs, usePrices } from "@left-curve/store";
import { Cell, SortHeader, Table, useApp } from "@left-curve/applets-kit";

import type { TableHeaderContext, TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type { PairId, PairUpdate } from "@left-curve/dango/types";
import type React from "react";
import { useMemo, type PropsWithChildren } from "react";

const SearchTokenTableContainer: React.FC<PropsWithChildren> = ({ children }) => <>{children}</>;

type SearchTokenTableProps = {
  classNames?: TableClassNames;
  data: PairUpdate[];
  searchText?: string;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
};

const SearchTokenSpotTable: React.FC<SearchTokenTableProps> = ({
  classNames,
  data: pairs,
  searchText,
  onChangePairId,
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { coins } = useConfig();
  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const { favPairs } = useFavPairs();

  const data = useMemo(() => [...pairs], [pairs, favPairs]);

  const columns: TableColumn<PairUpdate> = [
    {
      id: "isFavorite",
      accessorFn: (row) =>
        favPairs.includes(
          `${coins.byDenom[row.baseDenom].symbol}-${coins.byDenom[row.quoteDenom].symbol}`,
        ),
    },
    {
      id: "pairName",
      header: (ctx: TableHeaderContext<PairUpdate>) => (
        <SortHeader
          label="Name"
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: ({ row }) => {
        const pair = { baseDenom: row.original.baseDenom, quoteDenom: row.original.quoteDenom };
        return <Cell.PairNameWithFav type="Spot" pairId={pair} />;
      },
      filterFn: (row, _, value) => {
        const v = String(value ?? "").toUpperCase();
        const baseCoin = coins.byDenom[row.original.baseDenom];
        const quoteCoin = coins.byDenom[row.original.quoteDenom];
        return (
          baseCoin.symbol.toUpperCase().includes(v) || quoteCoin.symbol.toUpperCase().includes(v)
        );
      },
      accessorFn: (row) => {
        const baseCoin = coins.byDenom[row.baseDenom];
        const quoteCoin = coins.byDenom[row.quoteDenom];
        return `${baseCoin.symbol}-${quoteCoin.symbol}`;
      },
    },
    {
      id: "price",
      header: (ctx: TableHeaderContext<PairUpdate>) => (
        <SortHeader
          label="Price"
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: ({ row }) => <Cell.Text text={getPrice(1, row.original.baseDenom, { format: true })} />,
      accessorFn: (row) => getPrice(1, row.baseDenom, { format: false }),
    },
    {
      id: "change24h",
      header: (ctx: TableHeaderContext<PairUpdate>) => (
        <SortHeader
          label="24h Change"
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: () => <Cell.Text text="-" />,
    },
    {
      id: "volume",
      header: (ctx: TableHeaderContext<PairUpdate>) => (
        <SortHeader
          label="Volume"
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: () => <Cell.Text text="-" />,
    },
  ];

  return (
    <Table
      data={data}
      columns={columns}
      style="simple"
      classNames={classNames}
      initialSortState={[
        { id: "isFavorite", desc: true },
        { id: "pairName", desc: false },
      ]}
      initialColumnVisibility={{ isFavorite: false }}
      columnFilters={[{ id: "pairName", value: searchText }]}
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
