import {
  allPairStatsStore,
  allPerpsPairStatsStore,
  useFavPairs,
  TradePairStore,
} from "@left-curve/store";
import {
  Badge,
  Cell,
  FormattedNumber,
  IconEmptyStar,
  IconStar,
  PairStatValue,
  SortHeader,
  Table,
} from "@left-curve/applets-kit";
import { memo, useMemo } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { TableHeaderContext, TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type React from "react";

import type { SearchTokenRow } from "./SearchToken";

const TokenImage = memo(({ src, alt }: { src?: string; alt: string }) => (
  <img src={src} alt={alt} className="w-5 h-5 flex-shrink-0" />
));

const PerpsPairNameWithFav: React.FC<{
  baseCoin: { symbol: string; logoURI?: string };
  quoteCoin: { symbol: string };
  pairKey: string;
}> = memo(({ baseCoin, quoteCoin, pairKey }) => {
  const { toggleFavPair, hasFavPair } = useFavPairs();
  const isFav = hasFavPair(pairKey);

  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto min-w-fit pr-2">
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          toggleFavPair(pairKey);
        }}
        className="focus:outline-none flex-shrink-0"
      >
        {isFav ? (
          <IconStar className="w-4 h-4 text-fg-primary-700" />
        ) : (
          <IconEmptyStar className="w-4 h-4 text-fg-primary-700" />
        )}
      </button>
      <TokenImage src={baseCoin.logoURI} alt={baseCoin.symbol} />
      <p className="whitespace-nowrap">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
      <Badge text="Perp" color="green" size="s" />
    </div>
  );
});

function useRowPairStats(row: SearchTokenRow) {
  const statsByPair = allPairStatsStore((s) => s.pairStatsByKey);
  const perpStatsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);

  return row.mode === "perps"
    ? perpStatsByPairId[getPerpsPairId(row.pairId)]
    : statsByPair[`${row.pairId.baseDenom}:${row.pairId.quoteDenom}`];
}

const PriceCell = memo(({ row }: { row: SearchTokenRow }) => {
  const stats = useRowPairStats(row);

  return (
    <Cell.Text
      text={
        <FormattedNumber
          number={stats?.currentPrice ?? "0"}
          formatOptions={{ currency: "USD" }}
          as="span"
        />
      }
    />
  );
});

const ChangeCell = memo(({ row }: { row: SearchTokenRow }) => {
  const stats = useRowPairStats(row);

  return (
    <div className="flex flex-col gap-1">
      <PairStatValue
        kind="priceChange24h"
        value={stats?.priceChange24H}
        className="diatype-xs-medium"
        as="span"
      />
    </div>
  );
});

const VolumeCell = memo(({ row }: { row: SearchTokenRow }) => {
  const stats = useRowPairStats(row);

  return (
    <div className="flex flex-col gap-1">
      <PairStatValue
        kind="volume24h"
        value={stats?.volume24H}
        className="diatype-xs-medium"
        align="end"
        as="span"
      />
    </div>
  );
});

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
  const { favPairs } = useFavPairs();

  const statsByPair = allPairStatsStore((s) => s.pairStatsByKey);
  const perpStatsByPairId = allPerpsPairStatsStore((s) => s.perpsPairStatsByPairId);

  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);

  const data = useMemo(() => [...rows], [rows, favPairs]);

  const getPairStats = (row: SearchTokenRow) => {
    return row.mode === "perps"
      ? perpStatsByPairId[getPerpsPairId(row.pairId)]
      : statsByPair[`${row.pairId.baseDenom}:${row.pairId.quoteDenom}`];
  };

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
        return <Cell.PairNameWithFav type="Spot" pairId={row.original.pairId} />;
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
      cell: ({ row }) => <PriceCell row={row.original} />,
      accessorFn: (row) => Number(getPairStats(row)?.currentPrice ?? 0),
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
      cell: ({ row }) => <ChangeCell row={row.original} />,
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
      cell: ({ row }) => <VolumeCell row={row.original} />,
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
      onRowPointerDown={(row) => {
        onChangePairId(row.original);
      }}
    />
  );
};
