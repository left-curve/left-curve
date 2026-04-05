import { useEffect, useRef } from "react";
import { useConfig } from "./useConfig.js";

import { create } from "zustand";
import { TradePairStore } from "../stores/tradePairStore.js";

import type { Trade } from "@left-curve/dango/types";

export type UseLiveSpotTradesStoreState = {
  trades: Trade[];
  addTrades: (trades: Trade[]) => void;
  getTrades: () => Trade[];
  clearTrades: () => void;
};

export const liveSpotTradesStore = create<UseLiveSpotTradesStoreState>((set, get) => ({
  trades: [],
  addTrades: (trades) => set((state) => ({ trades: [...trades, ...state.trades].slice(0, 50) })),
  getTrades: () => get().trades,
  clearTrades: () => set(() => ({ trades: [] })),
}));

export type UseLiveTradesStateParameters = {
  subscribe?: boolean;
};

export function useLiveSpotTradesState(parameters: UseLiveTradesStateParameters) {
  const { subscribe } = parameters;
  const { subscriptions, coins } = useConfig();
  const tradesBuffer = useRef<Trade[]>([]);
  const debounceTimer = useRef<NodeJS.Timeout | null>(null);

  const pairId = TradePairStore((s) => s.pairId);
  const { addTrades, clearTrades } = liveSpotTradesStore();

  const baseCoin = coins.byDenom[pairId.baseDenom];
  const quoteCoin = coins.byDenom[pairId.quoteDenom];

  useEffect(() => {
    if (!subscribe || !baseCoin || !quoteCoin) return;
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

  return { liveSpotTradesStore };
}
