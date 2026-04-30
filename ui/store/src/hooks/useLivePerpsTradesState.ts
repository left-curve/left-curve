import { useEffect, useRef } from "react";
import { useConfig } from "./useConfig.js";

import { create } from "zustand";
import { TradePairStore } from "../stores/tradePairStore.js";

import type { PerpsTrade } from "@left-curve/dango/types";

export type UseLivePerpsTradesStoreState = {
  trades: PerpsTrade[];
  currentPrice: string | null;
  previousPrice: string | null;
  addTrades: (trades: PerpsTrade[]) => void;
  getTrades: () => PerpsTrade[];
  clearTrades: () => void;
};

export const livePerpsTradesStore = create<UseLivePerpsTradesStoreState>((set, get) => ({
  trades: [],
  currentPrice: null,
  previousPrice: null,
  addTrades: (trades) =>
    set((state) => {
      const newTrades = [...trades, ...state.trades].slice(0, 50);
      const latestPrice = newTrades[0]?.fillPrice ?? null;
      return {
        trades: newTrades,
        previousPrice: state.currentPrice,
        currentPrice: latestPrice,
      };
    }),
  getTrades: () => get().trades,
  clearTrades: () => set(() => ({ trades: [], currentPrice: null, previousPrice: null })),
}));

export type UseLivePerpsTradesStateParameters = {
  subscribe?: boolean;
};

export function useLivePerpsTradesState(parameters: UseLivePerpsTradesStateParameters) {
  const { subscribe } = parameters;
  const { subscriptions } = useConfig();

  const pairId = TradePairStore((s) => s.pairId);
  const getPerpsPairId = TradePairStore((s) => s.getPerpsPairId);

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
      params: { pairId: getPerpsPairId() },
      listener: async ({ perpsTrades: trade }) => {
        if (trade.isMaker === true) return;
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
