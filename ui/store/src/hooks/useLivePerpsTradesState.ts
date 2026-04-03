import { useEffect, useMemo, useRef } from "react";
import { useConfig } from "./useConfig.js";
import { toPerpsPairId } from "../stores/tradePairStore.js";

import { create } from "zustand";

import type { PairId, PerpsTrade } from "@left-curve/dango/types";

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
  pairId: PairId;
  subscribe?: boolean;
};

export function useLivePerpsTradesState(parameters: UseLivePerpsTradesStateParameters) {
  const { pairId, subscribe } = parameters;
  const { subscriptions, coins } = useConfig();

  const perpsPairId = useMemo(() => {
    const baseSymbol = coins.byDenom[pairId.baseDenom]?.symbol;
    const quoteSymbol = coins.byDenom[pairId.quoteDenom]?.symbol ?? "USD";
    return baseSymbol ? toPerpsPairId(baseSymbol, quoteSymbol) : "";
  }, [pairId, coins]);
  const tradesBuffer = useRef<PerpsTrade[]>([]);
  const debounceTimer = useRef<NodeJS.Timeout | null>(null);

  const { addTrades, clearTrades } = livePerpsTradesStore();

  useEffect(() => {
    if (!subscribe || !perpsPairId) return;
    const processBuffer = () => {
      if (tradesBuffer.current.length > 0) {
        addTrades(tradesBuffer.current);
        tradesBuffer.current = [];
      }
      debounceTimer.current = null;
    };

    const unsubscribe = subscriptions.subscribe("perpsTrades", {
      params: { pairId: perpsPairId },
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
  }, [perpsPairId, subscribe]);

  return { livePerpsTradesStore };
}
