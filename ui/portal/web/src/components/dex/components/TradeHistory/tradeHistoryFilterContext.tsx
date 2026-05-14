import { createContext } from "@left-curve/applets-kit";
import type { PropsWithChildren } from "react";
import { useMemo, useState } from "react";

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

type FilterContextValue = {
  filter: TradeHistoryFilter;
  setPreset: (preset: TradeHistoryPreset) => void;
  setCustomRange: (from: Date, to: Date) => void;
};

const [Provider, useTradeHistoryFilterContext] = createContext<FilterContextValue>({
  name: "TradeHistoryFilterContext",
});

export const TradeHistoryFilterProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const [filter, setFilter] = useState<TradeHistoryFilter>(initialFilter);

  const value = useMemo<FilterContextValue>(
    () => ({
      filter,
      setPreset: (preset) => {
        const config = PRESETS.find((p) => p.id === preset);
        if (!config) return;
        setFilter({ preset, ...buildPresetRange(config.days) });
      },
      setCustomRange: (from, to) => setFilter({ preset: null, from, to }),
    }),
    [filter],
  );

  return <Provider value={value}>{children}</Provider>;
};

export const useTradeHistoryFilter = useTradeHistoryFilterContext;
