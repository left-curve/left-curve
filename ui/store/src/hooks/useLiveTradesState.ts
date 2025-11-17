import { useEffect, useRef } from "react";
import { useConfig } from "./useConfig.js";

import { create } from "zustand";

import type { PairId, Trade } from "@left-curve/dango/types";

export type UseLiveTradesStoreState = {
  trades: Trade[];
  addTrades: (trades: Trade[]) => void;
  getTrades: () => Trade[];
  clearTrades: () => void;
};

const liveTradesStore = create<UseLiveTradesStoreState>((set, get) => ({
  trades: [],
  addTrades: (trades) => set((state) => ({ trades: [...trades, ...state.trades].slice(0, 50) })),
  getTrades: () => get().trades,
  clearTrades: () => set(() => ({ trades: [] })),
}));

export type UseLiveTradesStateParameters = {
  pairId: PairId;
  subscribe?: boolean;
};

export function useLiveTradesState(parameters: UseLiveTradesStateParameters) {
  const { pairId, subscribe } = parameters;
  const { subscriptions, coins } = useConfig();
  const tradesBuffer = useRef<Trade[]>([]);
  const debounceTimer = useRef<NodeJS.Timeout | null>(null);

  const { addTrades, clearTrades } = liveTradesStore();

  const baseCoin = coins.byDenom[pairId.baseDenom];
  const quoteCoin = coins.byDenom[pairId.quoteDenom];

  useEffect(() => {
    if (!subscribe) return;
    const processBuffer = () => {
      if (tradesBuffer.current.length > 0) {
        addTrades(tradesBuffer.current);
        tradesBuffer.current = [];
      }
      debounceTimer.current = null;
    };

    const unsubscribe = subscriptions.subscribe("trades", {
      params: {
        baseDenom: baseCoin.denom,
        quoteDenom: quoteCoin.denom,
      },
      listener: async ({ trades: trade }) => {
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
  }, [baseCoin, quoteCoin]);

  return { liveTradesStore };
}
