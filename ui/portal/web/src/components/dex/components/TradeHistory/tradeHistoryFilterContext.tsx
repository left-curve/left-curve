import { createContext } from "@left-curve/applets-kit";
import { useAccount, usePublicClient, useInfiniteGraphqlQuery } from "@left-curve/store";
import type { PerpsEvent } from "@left-curve/types";
import type { PropsWithChildren } from "react";
import { useCallback, useMemo, useState } from "react";

export type TradeHistoryPreset = "1d" | "1w" | "1m" | "3m";

export const PRESETS: Array<{ id: TradeHistoryPreset; days: number; label: string }> = [
  { id: "1d", days: 1, label: "1 Day" },
  { id: "1w", days: 7, label: "1 Week" },
  { id: "1m", days: 30, label: "1 Month" },
  { id: "3m", days: 90, label: "3 Months" },
];

export type TradeHistoryFilter = {
  preset: TradeHistoryPreset | null;
  from: Date;
  to: Date;
};

// TODO: Backend currently caps `perpsEvents` at max_items=100. Once bumped,
// this can be raised. 30 is sized to comfortably cover the initial viewport
// so a follow-up fetch only fires once the user actually scrolls past it.
const PAGE_SIZE = 30;

const buildPresetRange = (days: number): { from: Date; to: Date } => {
  const to = new Date();
  const from = new Date(to.getTime() - days * 24 * 60 * 60 * 1000);
  return { from, to };
};

const initialPreset: TradeHistoryPreset = "1m";

const initialFilter: TradeHistoryFilter = {
  preset: initialPreset,
  ...buildPresetRange(PRESETS.find((p) => p.id === initialPreset)?.days ?? 30),
};

type QueryRange = { earlierThan: string | undefined; laterThan: string | undefined };

type TradeHistoryContextValue = {
  filter: TradeHistoryFilter;
  setPreset: (preset: TradeHistoryPreset) => void;
  setCustomRange: (from: Date, to: Date) => void;
  queryRange: QueryRange;
  /** Flat list of perps events across all loaded pages. */
  nodes: PerpsEvent[];
  isLoading: boolean;
  isFetchingNextPage: boolean;
  hasNextPage: boolean;
  fetchNextPage: () => void;
  /**
   * Whether the filter UI (date presets, range picker) should be exposed and
   * applied to the query. When false, the query runs without date filters so
   * the user sees the most recent events regardless of when the page loaded.
   */
  filtersEnabled: boolean;
};

const [Provider, useTradeHistoryContext] = createContext<TradeHistoryContextValue>({
  name: "TradeHistoryContext",
});

export const TradeHistoryFilterProvider: React.FC<
  PropsWithChildren<{ enableFilters: boolean }>
> = ({ children, enableFilters }) => {
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const [filter, setFilter] = useState<TradeHistoryFilter>(initialFilter);

  const queryRange: QueryRange = enableFilters
    ? {
        // Rolling presets stay open-ended on the upper bound so newly indexed
        // trades aren't capped by a `to` value frozen at page load. Custom
        // ranges keep the explicit upper bound the user picked.
        earlierThan: filter.preset === null ? filter.to.toISOString() : undefined,
        laterThan: filter.from.toISOString(),
      }
    : { earlierThan: undefined, laterThan: undefined };

  const { earlierThan, laterThan } = queryRange;
  const address = account?.address;

  const infiniteQuery = useInfiniteGraphqlQuery<PerpsEvent>({
    limit: PAGE_SIZE,
    query: {
      enabled: !!address,
      queryKey: ["perpsTradeHistory", address ?? "", earlierThan ?? "", laterThan ?? ""],
      queryFn: async ({ pageParam }) => {
        if (!address) throw new Error("missing account");
        return await publicClient.queryPerpsEvents({
          userAddr: address,
          sortBy: "BLOCK_HEIGHT_DESC",
          earlierThan,
          laterThan,
          first: pageParam.first,
          last: pageParam.last,
          after: pageParam.after,
          before: pageParam.before,
        });
      },
    },
  });

  const nodes = useMemo(
    () => infiniteQuery.data?.pages.flatMap((page) => page.nodes) ?? [],
    [infiniteQuery.data],
  );

  const fetchNextPage = useCallback(() => {
    if (infiniteQuery.hasNextPage && !infiniteQuery.isFetchingNextPage) {
      infiniteQuery.fetchNextPage();
    }
  }, [infiniteQuery.fetchNextPage, infiniteQuery.hasNextPage, infiniteQuery.isFetchingNextPage]);

  const setPreset = useCallback((preset: TradeHistoryPreset) => {
    const config = PRESETS.find((p) => p.id === preset);
    if (!config) return;
    setFilter({ preset, ...buildPresetRange(config.days) });
  }, []);

  const setCustomRange = useCallback((from: Date, to: Date) => {
    setFilter({ preset: null, from, to });
  }, []);

  const value = useMemo<TradeHistoryContextValue>(
    () => ({
      filter,
      setPreset,
      setCustomRange,
      queryRange,
      nodes,
      isLoading: infiniteQuery.isLoading,
      isFetchingNextPage: infiniteQuery.isFetchingNextPage,
      hasNextPage: infiniteQuery.hasNextPage,
      fetchNextPage,
      filtersEnabled: enableFilters,
    }),
    [
      filter,
      setPreset,
      setCustomRange,
      queryRange.earlierThan,
      queryRange.laterThan,
      nodes,
      infiniteQuery.isLoading,
      infiniteQuery.isFetchingNextPage,
      infiniteQuery.hasNextPage,
      fetchNextPage,
      enableFilters,
    ],
  );

  return <Provider value={value}>{children}</Provider>;
};

export const useTradeHistoryFilter = useTradeHistoryContext;
