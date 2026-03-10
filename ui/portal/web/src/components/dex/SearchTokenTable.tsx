import { useAllPairStats, useConfig, useFavPairs, usePrices } from "@left-curve/store";
import { Cell, PairStatValue, SortHeader, Table, useApp } from "@left-curve/applets-kit";

import type { TableHeaderContext, TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type { PairId, PairUpdate } from "@left-curve/dango/types";
import type React from "react";
import { useMemo, type PropsWithChildren } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

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
  const { statsByPair } = useAllPairStats();

  const data = useMemo(() => [...pairs], [pairs, favPairs]);
  const getPairStats = (row: PairUpdate) => statsByPair[`${row.baseDenom}:${row.quoteDenom}`];

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
          label={m["dex.protrade.searchPairTable.name"]()}
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
          label={m["dex.protrade.searchPairTable.price"]()}
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
          label={m["dex.protrade.searchPairTable.24hChange"]()}
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: ({ row }) => {
        const stats = getPairStats(row.original);
        return (
          <div className="flex flex-col gap-1">
            <PairStatValue
              kind="priceChange24h"
              value={stats?.priceChange24H}
              formatOptions={{ ...formatNumberOptions, maximumTotalDigits: 6 }}
              className="diatype-xs-medium"
              as="span"
            />
          </div>
        );
      },
      accessorFn: (row) => {
        const value = getPairStats(row)?.priceChange24H;
        return value === null || value === undefined ? Number.NEGATIVE_INFINITY : Number(value);
      },
    },
    {
      id: "volume",
      header: (ctx: TableHeaderContext<PairUpdate>) => (
        <SortHeader
          label={m["dex.protrade.searchPairTable.volume"]()}
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
          className="ml-auto w-full justify-end"
        />
      ),
      cell: ({ row }) => {
        const value = getPairStats(row.original)?.volume24H;
        return (
          <div className="flex flex-col gap-1">
            <PairStatValue
              kind="volume24h"
              value={value}
              formatOptions={{ ...formatNumberOptions, maximumTotalDigits: 5 }}
              className="diatype-xs-medium"
              align="end"
              as="span"
            />
          </div>
        );
      },
      accessorFn: (row) => {
        const value = getPairStats(row)?.volume24H;
        return value === undefined ? Number.NEGATIVE_INFINITY : Number(value);
      },
    },
  ];

  return (
    <Table
      data={data}
      columns={columns}
      style="simple"
      classNames={classNames}
      initialSortState={{
        fixed: [{ id: "isFavorite", desc: true }],
        variable: [{ id: "pairName", desc: false }],
      }}
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
