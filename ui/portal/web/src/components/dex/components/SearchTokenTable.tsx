import {
  getPerpsPairIdFromPairId,
  useAllPerpsPairStats,
  useConfig,
  useFavPairs,
} from "@left-curve/store";
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
  baseCoin: { symbol: string; logoURI?: string };
  quoteCoin: { symbol: string };
  pairKey: string;
  boostMultiplier?: string;
}> = memo(({ baseCoin, quoteCoin, pairKey, boostMultiplier }) => {
  const { toggleFavPair, hasFavPair } = useFavPairs();
  const isFav = hasFavPair(pairKey);

  return (
    <div className="flex h-full gap-2 diatype-sm-medium justify-start items-center my-auto min-w-fit pr-2">
      <StarToggleButton
        isActive={isFav}
        onToggle={() => toggleFavPair(pairKey)}
        className={isFav ? "text-primitives-warning-500" : "text-fg-secondary-500"}
      />
      <TokenImage src={baseCoin.logoURI} alt={baseCoin.symbol} />
      <p className="whitespace-nowrap">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
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
  onChangePairId: (row: SearchTokenRow) => void;
};

export const SearchTokenTable: React.FC<SearchTokenTableProps> = ({
  classNames,
  data: rows,
  onChangePairId,
}) => {
  const { coins } = useConfig();
  const perpStatsByPairId = useAllPerpsPairStats((s) => s.perpsPairStatsByPairId);
  const getPerpsPairId = useCallback(
    (pairId: SearchTokenRow["pairId"]) => getPerpsPairIdFromPairId(pairId, coins),
    [coins],
  );

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
            baseCoin={row.original.baseCoin}
            quoteCoin={row.original.quoteCoin}
            pairKey={row.original.pairKey}
            boostMultiplier={row.original.boostMultiplier}
          />
        ),
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
        cell: ({ row }) => (
          <PriceCell stats={perpStatsByPairId[getPerpsPairId(row.original.pairId)]} />
        ),
        accessorFn: (row) => {
          const stats = perpStatsByPairId[getPerpsPairId(row.pairId)];
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
        cell: ({ row }) => (
          <ChangeCell stats={perpStatsByPairId[getPerpsPairId(row.original.pairId)]} />
        ),
        accessorFn: (row) => {
          const stats = perpStatsByPairId[getPerpsPairId(row.pairId)];
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
        cell: ({ row }) => (
          <VolumeCell stats={perpStatsByPairId[getPerpsPairId(row.original.pairId)]} />
        ),
        accessorFn: (row) => {
          const stats = perpStatsByPairId[getPerpsPairId(row.pairId)];
          const value = stats?.volume24H;
          return value === undefined ? Number.NEGATIVE_INFINITY : Number(value);
        },
      },
    ],
    [getPerpsPairId, perpStatsByPairId],
  );

  const getRowId = useCallback((row: SearchTokenRow) => row.pairKey, []);

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
        onChangePairId(row.original);
      }}
    />
  );
};
