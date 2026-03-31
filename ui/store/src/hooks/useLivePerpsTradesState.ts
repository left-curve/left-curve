import { useEffect, useRef } from "react";
import { useConfig } from "./useConfig.js";

import { create } from "zustand";

import type { PerpsTrade } from "@left-curve/dango/types";

export type UseLivePerpsTradesStoreState = {
  trades: PerpsTrade[];
  addTrades: (trades: PerpsTrade[]) => void;
  getTrades: () => PerpsTrade[];
  clearTrades: () => void;
};

export const livePerpsTradesStore = create<UseLivePerpsTradesStoreState>((set, get) => ({
  trades: [],
  addTrades: (trades) => set((state) => ({ trades: [...trades, ...state.trades].slice(0, 50) })),
  getTrades: () => get().trades,
  clearTrades: () => set(() => ({ trades: [] })),
}));

export type UseLivePerpsTradesStateParameters = {
  pairId: string;
  subscribe?: boolean;
};

export function useLivePerpsTradesState(parameters: UseLivePerpsTradesStateParameters) {
  const { pairId, subscribe } = parameters;
  const { subscriptions } = useConfig();
  const tradesBuffer = useRef<PerpsTrade[]>([]);
  const debounceTimer = useRef<NodeJS.Timeout | null>(null);

  const { addTrades, clearTrades } = livePerpsTradesStore();

  useEffect(() => {
    if (!subscribe || !pairId) return;
    const processBuffer = () => {
      if (tradesBuffer.current.length > 0) {
        addTrades(tradesBuffer.current);
        tradesBuffer.current = [];
      }
      debounceTimer.current = null;
    };

    const unsubscribe = subscriptions.subscribe("perpsTrades", {
      params: { pairId },
      listener: async ({ perpsTrades: trade }) => {
        tradesBuffer.current.unshift(trade);
        if (debounceTimer.current) clearTimeout(debounceTimer.current);
        debounceTimer.current = setTimeout(processBuffer, 500);
      },
    });

    return () => {
      unsubscribe();
      clearTrades();
      if (debounceTimer.current) clearTimeout(debounceTimer.current);
    };
  }, [pairId, subscribe]);

  return { livePerpsTradesStore };
}
