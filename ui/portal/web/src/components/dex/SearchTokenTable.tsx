import { useConfig, useFavPairs, usePrices } from "@left-curve/store";
import { useMemo, useCallback } from "react";
import { Cell, SortHeader, Table, useApp, useTableSort } from "@left-curve/applets-kit";

import type { SortKeys, TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type { PairId, PairUpdate } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";

const SearchTokenTableContainer: React.FC<PropsWithChildren> = ({ children }) => <>{children}</>;

type SearchTokenTableProps = {
  classNames?: TableClassNames;
  data: PairUpdate[];
  searchText?: string;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
};

type SortBy = "pairName" | "price" | "change24h" | "volume" | string;

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
  const { hasFavPair } = useFavPairs();

  const sortKeys = useMemo<SortKeys<PairUpdate, SortBy>>(
    () => ({
      pairName: (row) => {
        const base = coins.byDenom[row.baseDenom]?.symbol ?? row.baseDenom;
        const quote = coins.byDenom[row.quoteDenom]?.symbol ?? row.quoteDenom;
        return `${base}-${quote}`.toUpperCase();
      },
      price: (row) => getPrice(1, row.baseDenom, { format: false }),
      change24h: () => 0,
      volume: () => 0,
    }),
    [coins.byDenom, getPrice],
  );

  const groupFavs = useCallback(
    (row: PairUpdate) => hasFavPair({ baseDenom: row.baseDenom, quoteDenom: row.quoteDenom }),
    [hasFavPair],
  );

  const { sortedData, sortKey, sortDir, toggleSortDir } = useTableSort<PairUpdate, SortBy>({
    data,
    sortKeys,
    initialKey: "pairName",
    initialDir: "asc",
    groupFirst: groupFavs,
  });

  const sortState = {
    sortKey,
    sortDir,
    onClick: toggleSortDir,
  } as const;

  const columns: TableColumn<PairUpdate> = [
    {
      id: "pairName",
      header: () => <SortHeader label="Name" key="pairName" {...sortState} />,
      cell: ({ row }) => {
        const pair = { baseDenom: row.original.baseDenom, quoteDenom: row.original.quoteDenom };
        return <Cell.PairNameWithFav type="Spot" pairId={pair} />;
      },
      filterFn: (row, _, value) => {
        const v = String(value ?? "").toUpperCase();
        const baseCoin = coins.byDenom[row.original.baseDenom];
        const quoteCoin = coins.byDenom[row.original.quoteDenom];
        return (
          (baseCoin?.symbol ?? "").toUpperCase().includes(v) ||
          (quoteCoin?.symbol ?? "").toUpperCase().includes(v)
        );
      },
    },
    {
      id: "price",
      header: () => <SortHeader label="Price" key="price" {...sortState} />,
      cell: ({ row }) => <Cell.Text text={getPrice(1, row.original.baseDenom, { format: true })} />,
    },
    {
      id: "change24h",
      header: () => <SortHeader label="24h Change" key="change24h" {...sortState} />,
      cell: () => <Cell.Text text="-" />,
    },
    {
      id: "volume",
      header: () => <SortHeader label="Volume" key="volume" {...sortState} />,
      cell: () => <Cell.Text text="-" />,
    },
  ];

  const columnFilters = useMemo(() => [{ id: "pairName", value: searchText }], [searchText]);

  return (
    <Table
      data={sortedData}
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
