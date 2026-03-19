import {
  useAllPairStats,
  useFavPairs,
  usePrices,
  perpsOrderBookStore,
  tradePairStore,
} from "@left-curve/store";
import {
  Badge,
  Cell,
  IconEmptyStar,
  IconStar,
  PairStatValue,
  SortHeader,
  Table,
  useApp,
} from "@left-curve/applets-kit";
import { useMemo } from "react";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { TableHeaderContext, TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type React from "react";

import type { SearchTokenRow } from "./SearchToken";

const PerpsPairNameWithFav: React.FC<{
  baseCoin: { symbol: string; logoURI?: string };
  quoteCoin: { symbol: string };
  pairKey: string;
}> = ({ baseCoin, quoteCoin, pairKey }) => {
  const { toggleFavPair, hasFavPair } = useFavPairs();
  const isFav = hasFavPair(pairKey);

  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto">
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          toggleFavPair(pairKey);
        }}
        className="focus:outline-none"
      >
        {isFav ? (
          <IconStar className="w-4 h-4 text-fg-primary-700" />
        ) : (
          <IconEmptyStar className="w-4 h-4 text-fg-primary-700" />
        )}
      </button>
      <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="w-5 h-5" />
      <p className="min-w-[4.5rem]">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
      <Badge text="Perp" color="green" size="s" />
    </div>
  );
};

type SearchTokenTableProps = {
  classNames?: TableClassNames;
  data: SearchTokenRow[];
  onChangePairId: (row: SearchTokenRow) => void;
};

export const SearchTokenTable: React.FC<SearchTokenTableProps> = ({
  classNames,
  data: rows,
  onChangePairId,
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const { favPairs } = useFavPairs();
  const { statsByPair } = useAllPairStats();

  const activePerpsPrice = perpsOrderBookStore((s) => s.currentPrice);
  const activePairId = tradePairStore((s) => s.pairId);
  const activeMode = tradePairStore((s) => s.mode);

  const data = useMemo(() => [...rows], [rows, favPairs]);

  const getPairStats = (row: SearchTokenRow) =>
    statsByPair[`${row.pairId.baseDenom}:${row.pairId.quoteDenom}`];

  const columns: TableColumn<SearchTokenRow> = [
    {
      id: "isFavorite",
      accessorFn: (row) => favPairs.includes(row.pairKey),
    },
    {
      id: "pairName",
      header: (ctx: TableHeaderContext<SearchTokenRow>) => (
        <SortHeader
          label={m["dex.protrade.searchPairTable.name"]()}
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: ({ row }) => {
        if (row.original.mode === "perps") {
          return (
            <PerpsPairNameWithFav
              baseCoin={row.original.baseCoin}
              quoteCoin={row.original.quoteCoin}
              pairKey={row.original.pairKey}
            />
          );
        }
        const pair = {
          baseDenom: row.original.pairId.baseDenom,
          quoteDenom: row.original.pairId.quoteDenom,
        };
        return <Cell.PairNameWithFav type="Spot" pairId={pair} />;
      },
      filterFn: (row, _, value) => {
        const v = String(value ?? "").toUpperCase();
        return (
          row.original.baseCoin.symbol.toUpperCase().includes(v) ||
          row.original.quoteCoin.symbol.toUpperCase().includes(v)
        );
      },
      accessorFn: (row) => row.pairKey,
    },
    {
      id: "price",
      header: (ctx: TableHeaderContext<SearchTokenRow>) => (
        <SortHeader
          label={m["dex.protrade.searchPairTable.price"]()}
          sorted={ctx.column.getIsSorted()}
          toggleSort={ctx.column.toggleSorting}
        />
      ),
      cell: ({ row }) => {
        if (
          row.original.mode === "perps" &&
          activeMode === "perps" &&
          row.original.pairId.baseDenom === activePairId.baseDenom &&
          activePerpsPrice !== "0"
        ) {
          return <Cell.Text text={formatNumber(activePerpsPrice, formatNumberOptions)} />;
        }
        return <Cell.Text text={getPrice(1, row.original.baseCoin.denom, { format: true })} />;
      },
      accessorFn: (row) => getPrice(1, row.baseCoin.denom, { format: false }),
    },
    {
      id: "change24h",
      header: (ctx: TableHeaderContext<SearchTokenRow>) => (
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
      header: (ctx: TableHeaderContext<SearchTokenRow>) => (
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
      onRowClick={(row) => {
        onChangePairId(row.original);
      }}
    />
  );
};
