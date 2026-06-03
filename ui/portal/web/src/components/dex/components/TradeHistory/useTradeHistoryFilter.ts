import { useCallback, useState } from "react";

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

export function useTradeHistoryFilter() {
  const [filter, setFilter] = useState<TradeHistoryFilter>(initialFilter);

  const queryRange: QueryRange = {
    earlierThan: filter.preset === null ? filter.to.toISOString() : undefined,
    laterThan: filter.from.toISOString(),
  };

  const setPreset = useCallback((preset: TradeHistoryPreset) => {
    const config = PRESETS.find((p) => p.id === preset);
    if (!config) return;
    setFilter({ preset, ...buildPresetRange(config.days) });
  }, []);

  const setCustomRange = useCallback((from: Date, to: Date) => {
    setFilter({ preset: null, from, to });
  }, []);

  return { filter, setPreset, setCustomRange, queryRange };
}
