import { createContext } from "@left-curve/applets-kit";
import type { PropsWithChildren } from "react";
import { useCallback, useMemo, useState } from "react";

export type TradeHistoryPreset = "1d" | "1w" | "1m" | "3m";

export const PRESETS: ReadonlyArray<{ id: TradeHistoryPreset; days: number }> = [
  { id: "1d", days: 1 },
  { id: "1w", days: 7 },
  { id: "1m", days: 30 },
  { id: "3m", days: 90 },
];

export type TradeHistoryFilter = {
  preset: TradeHistoryPreset | null;
  from: Date;
  to: Date;
};

export type QueryRange = {
  earlierThan: string | undefined;
  laterThan: string | undefined;
};

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

type TradeHistoryFilterContextValue = {
  filter: TradeHistoryFilter;
  setPreset: (preset: TradeHistoryPreset) => void;
  setCustomRange: (from: Date, to: Date) => void;
  queryRange: QueryRange;
  filtersEnabled: boolean;
};

const [Provider, useTradeHistoryFilter] = createContext<TradeHistoryFilterContextValue>({
  name: "TradeHistoryFilterContext",
});

export { useTradeHistoryFilter };

export function TradeHistoryFilterProvider({
  children,
  enableFilters,
}: PropsWithChildren<{ enableFilters: boolean }>) {
  const [filter, setFilter] = useState<TradeHistoryFilter>(initialFilter);

  const queryRange: QueryRange = enableFilters
    ? {
        earlierThan: filter.preset === null ? filter.to.toISOString() : undefined,
        laterThan: filter.from.toISOString(),
      }
    : { earlierThan: undefined, laterThan: undefined };

  const setPreset = useCallback((preset: TradeHistoryPreset) => {
    const config = PRESETS.find((p) => p.id === preset);
    if (!config) return;
    setFilter({ preset, ...buildPresetRange(config.days) });
  }, []);

  const setCustomRange = useCallback((from: Date, to: Date) => {
    setFilter({ preset: null, from, to });
  }, []);

  const value = useMemo<TradeHistoryFilterContextValue>(
    () => ({
      filter,
      setPreset,
      setCustomRange,
      queryRange,
      filtersEnabled: enableFilters,
    }),
    [
      filter,
      setPreset,
      setCustomRange,
      queryRange.earlierThan,
      queryRange.laterThan,
      enableFilters,
    ],
  );

  return <Provider value={value}>{children}</Provider>;
}
