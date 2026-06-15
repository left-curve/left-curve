import { useAllPerpsPairStats, useFavPairs } from "@left-curve/store";
import {
  Badge,
  Cell,
  FormattedNumber,
  IconFlame,
  PairStatValue,
  SortHeader,
  StarToggleButton,
  Table,
  Tooltip,
} from "@left-curve/applets-kit";
import { memo, useCallback, useMemo } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Image } from "~/components/foundation/Image";

import type { TableHeaderContext, TableClassNames, TableColumn } from "@left-curve/applets-kit";
import type React from "react";

import type { NormalizedPerpsPairStats } from "@left-curve/store";
import type { SearchTokenRow } from "./SearchToken";

const TokenImage = memo(({ src, alt }: { src?: string; alt: string }) => (
  <Image src={src} alt={alt} className="w-5 h-5 flex-shrink-0" />
));

// `Udec128_6` arrives as a stringified decimal like "2.000000". Trim trailing
// zeros (and the dangling dot) for display: "2.000000" → "2", "2.500000" → "2.5".
const formatMultiplier = (raw: string) => raw.replace(/\.?0+$/, "") || raw;

const PerpsPairNameWithFav: React.FC<{
  pair: SearchTokenRow["pair"];
  boostMultiplier?: string;
}> = memo(({ pair, boostMultiplier }) => {
  const { toggleFavPair, hasFavPair } = useFavPairs();
  const isFav = hasFavPair(pair.ticker);

  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto min-w-fit pr-2">
      <StarToggleButton
        isActive={isFav}
        onToggle={() => toggleFavPair(pair.ticker)}
        className={isFav ? "text-primitives-warning-500" : "text-fg-secondary-500"}
      />
      <TokenImage src={pair.logoURI} alt={pair.base.symbol} />
      <p className="whitespace-nowrap">{pair.ticker}</p>
      <Badge text="Perp" color="green" size="s" />
      {boostMultiplier ? (
        <Tooltip
          className="min-w-0 rounded-md"
          content={
            <div className="diatype-sm-regular text-primitives-gray-dark-200">
              {`${formatMultiplier(boostMultiplier)}x points`}
            </div>
          }
        >
          <IconFlame className="text-primitives-red-light-500 w-4 h-4" />
        </Tooltip>
      ) : null}
    </div>
  );
});

const PriceCell = memo(({ stats }: { stats?: NormalizedPerpsPairStats }) => {
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

const ChangeCell = memo(({ stats }: { stats?: NormalizedPerpsPairStats }) => {
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

const VolumeCell = memo(({ stats }: { stats?: NormalizedPerpsPairStats }) => {
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
  onChangePair: (row: SearchTokenRow) => void;
};

export const SearchTokenTable: React.FC<SearchTokenTableProps> = ({
  classNames,
  data: rows,
  onChangePair,
}) => {
  const perpStatsByPairId = useAllPerpsPairStats((s) => s.perpsPairStatsByPairId);

  const columns = useMemo<TableColumn<SearchTokenRow>>(
    () => [
      {
        id: "isFavorite",
        accessorFn: (row) => row.isFavorite,
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
        cell: ({ row }) => (
          <PerpsPairNameWithFav
            pair={row.original.pair}
            boostMultiplier={row.original.boostMultiplier}
          />
        ),
        filterFn: (row, _, value) => {
          const v = String(value ?? "").toUpperCase();
          return (
            row.original.pair.ticker.toUpperCase().includes(v) ||
            row.original.pair.name.toUpperCase().includes(v)
          );
        },
        accessorFn: (row) => row.pair.ticker,
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
        cell: ({ row }) => <PriceCell stats={perpStatsByPairId[row.original.pair.id]} />,
        accessorFn: (row) => {
          const stats = perpStatsByPairId[row.pair.id];
          return Number(stats?.currentPrice ?? 0);
        },
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
        cell: ({ row }) => <ChangeCell stats={perpStatsByPairId[row.original.pair.id]} />,
        accessorFn: (row) => {
          const stats = perpStatsByPairId[row.pair.id];
          const value = stats?.priceChange24H;
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
        cell: ({ row }) => <VolumeCell stats={perpStatsByPairId[row.original.pair.id]} />,
        accessorFn: (row) => {
          const stats = perpStatsByPairId[row.pair.id];
          const value = stats?.volume24H;
          return value === undefined ? Number.NEGATIVE_INFINITY : Number(value);
        },
      },
    ],
    [perpStatsByPairId],
  );

  const getRowId = useCallback((row: SearchTokenRow) => row.pair.ticker, []);

  return (
    <Table
      data={rows}
      columns={columns}
      style="simple"
      classNames={classNames}
      getRowId={getRowId}
      initialSortState={{
        fixed: [{ id: "isFavorite", desc: true }],
        variable: [{ id: "pairName", desc: false }],
      }}
      initialColumnVisibility={{ isFavorite: false }}
      onRowPointerDown={(row) => {
        onChangePair(row.original);
      }}
    />
  );
};
